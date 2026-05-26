mod config;
mod mailer;
mod password_reset;
mod routes;
mod schema;

use async_trait::async_trait;
use axum::Router;
use sea_orm::{DatabaseConnection, EntityTrait};
use serde_json::json;
use tiphia_core::{
    AppResult, AppState,
    entities::{comments, posts},
    plugins::{
        Hook, HookContext, HookMap, Plugin, PluginConfigSchema, PluginManifest,
        PluginRegistryBuilder, ensure_plugin_config, load_plugin_config,
    },
};
use tracing::{info, warn};

use crate::{
    config::CommentMailPushConfig,
    mailer::{escape_html, send_mail},
};

pub const COMMENT_MAIL_PUSH_PLUGIN_NAME: &str = "tiphia-comment-mail-push";

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(CommentMailPushPlugin);
    Ok(())
}

pub struct CommentMailPushPlugin;

static MANIFEST: PluginManifest = PluginManifest {
    name: COMMENT_MAIL_PUSH_PLUGIN_NAME,
    version: "0.1.0",
    description: "Sends comment notification emails and provides password recovery via SMTP.",
    author: "TiphiaPress",
};

#[async_trait]
impl Plugin for CommentMailPushPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(
            db,
            self.manifest().name,
            json!(CommentMailPushConfig::default()),
        )
        .await
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(schema::config_schema())
    }

    fn hooks(&self) -> HookMap {
        HookMap::from([(Hook::AfterCommentCreate, 80)])
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        if hook == Hook::AfterCommentCreate {
            notify_new_comment(context).await?;
        }
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(routes::router())
    }
}

async fn notify_new_comment(context: &mut HookContext) -> AppResult<()> {
    let db = context.database()?.clone();
    let config = load_plugin_config(
        &db,
        COMMENT_MAIL_PUSH_PLUGIN_NAME,
        CommentMailPushConfig::default(),
    )
    .await?;
    if !config.comment_push_ready() {
        return Ok(());
    }
    let Some(recipient) = config.comment_recipient() else {
        return Ok(());
    };
    let Some(comment) = context.subject_as::<comments::Model>()? else {
        return Ok(());
    };

    let post_title = posts::Entity::find_by_id(comment.post_id)
        .one(&db)
        .await?
        .map(|post| post.title)
        .unwrap_or_else(|| format!("#{}", comment.post_id));

    let subject = format!("新评论：{}", post_title);
    let html = format!(
        "<h2>收到新评论</h2><p><strong>文章：</strong>{}</p><p><strong>作者：</strong>{} &lt;{}&gt;</p><p><strong>状态：</strong>{:?}</p><blockquote>{}</blockquote>",
        escape_html(&post_title),
        escape_html(&comment.author_name),
        escape_html(&comment.author_email),
        comment.status,
        escape_html(&comment.content).replace('\n', "<br>"),
    );

    if let Err(err) = send_mail(&config, &recipient, &subject, &html).await {
        // 评论已经创建成功，邮件失败不应阻断评论流程。
        warn!(plugin = COMMENT_MAIL_PUSH_PLUGIN_NAME, error = %err, "failed to send comment notification");
    }

    Ok(())
}
