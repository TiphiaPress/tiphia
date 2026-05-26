mod config;
mod mailer;
mod password_reset;
mod routes;
mod schema;

use async_trait::async_trait;
use axum::Router;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;
use std::collections::HashSet;
use tiphia_core::{
    AppResult, AppState,
    entities::{comments, options, posts, users},
    plugins::{
        Hook, HookContext, HookMap, Plugin, PluginConfigSchema, PluginManifest,
        PluginRegistryBuilder, ensure_plugin_config, load_plugin_config,
    },
    services::settings::{self, SiteSettings},
};
use tracing::{info, warn};

use crate::{
    config::CommentMailPushConfig,
    mailer::{escape_html, render_email_template, send_mail},
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
    if !config.comment_push_ready() && !config.comment_reply_ready() {
        return Ok(());
    }
    let Some(comment) = context.subject_as::<comments::Model>()? else {
        return Ok(());
    };

    let Some(post) = posts::Entity::find_by_id(comment.post_id).one(&db).await? else {
        return Ok(());
    };
    let post_author = users::Entity::find_by_id(post.author_id).one(&db).await?;
    let settings = site_settings(&db).await?;
    let post_url = absolute_post_url(&settings, &post);

    if config.comment_push_ready() {
        let mut recipients = Vec::new();
        if let Some(recipient) = config.comment_recipient() {
            recipients.push(recipient);
        }
        if config.notify_post_author_enabled {
            if let Some(author) = post_author.as_ref() {
                recipients.push(author.email.clone());
            }
        }
        send_comment_notifications(
            &config,
            &post,
            post_author.as_ref(),
            &comment,
            &post_url,
            recipients,
        )
        .await;
    }

    if config.comment_reply_ready() {
        send_reply_notification(&config, &post, &comment, &post_url, &db).await?;
    }

    Ok(())
}

async fn send_comment_notifications(
    config: &CommentMailPushConfig,
    post: &posts::Model,
    post_author: Option<&users::Model>,
    comment: &comments::Model,
    post_url: &str,
    recipients: Vec<String>,
) {
    let post_author_name = post_author
        .map(|author| author.display_name.as_str())
        .unwrap_or("");
    let post_author_email = post_author
        .map(|author| author.email.as_str())
        .unwrap_or("");
    let html = render_email_template(
        config.comment_template(),
        &[
            ("post_title", escape_html(&post.title)),
            ("post_url", escape_html(post_url)),
            ("sender_name", escape_html(&comment.author_name)),
            ("sender_email", escape_html(&comment.author_email)),
            ("commenter_name", escape_html(&comment.author_name)),
            ("commenter_email", escape_html(&comment.author_email)),
            ("post_author_name", escape_html(post_author_name)),
            ("post_author_email", escape_html(post_author_email)),
            ("comment_status", format!("{:?}", comment.status)),
            (
                "comment_content",
                escape_html(&comment.content).replace('\n', "<br>"),
            ),
            // Backward compatibility for old custom templates.
            ("author_name", escape_html(&comment.author_name)),
            ("author_email", escape_html(&comment.author_email)),
        ],
    );
    let subject = format!("新评论：{}", post.title);
    for recipient in unique_emails(recipients) {
        if let Err(err) = send_mail(config, &recipient, &subject, &html).await {
            warn!(plugin = COMMENT_MAIL_PUSH_PLUGIN_NAME, to = %recipient, error = %err, "failed to send comment notification");
        }
    }
}

async fn send_reply_notification(
    config: &CommentMailPushConfig,
    post: &posts::Model,
    comment: &comments::Model,
    post_url: &str,
    db: &DatabaseConnection,
) -> AppResult<()> {
    let Some(parent_id) = comment.parent_id else {
        return Ok(());
    };
    let Some(parent) = comments::Entity::find_by_id(parent_id).one(db).await? else {
        return Ok(());
    };
    if parent.author_email.trim().is_empty()
        || parent
            .author_email
            .eq_ignore_ascii_case(comment.author_email.trim())
    {
        return Ok(());
    }

    let html = render_email_template(
        config.comment_reply_template(),
        &[
            ("post_title", escape_html(&post.title)),
            ("post_url", escape_html(post_url)),
            ("sender_name", escape_html(&comment.author_name)),
            ("sender_email", escape_html(&comment.author_email)),
            ("recipient_name", escape_html(&parent.author_name)),
            ("recipient_email", escape_html(&parent.author_email)),
            (
                "comment_content",
                escape_html(&comment.content).replace('\n', "<br>"),
            ),
            (
                "replied_content",
                escape_html(&parent.content).replace('\n', "<br>"),
            ),
            // Backward compatibility for old custom templates.
            ("author_name", escape_html(&comment.author_name)),
            ("author_email", escape_html(&comment.author_email)),
        ],
    );
    let subject = format!("你的评论收到回复：{}", post.title);
    if let Err(err) = send_mail(config, &parent.author_email, &subject, &html).await {
        warn!(plugin = COMMENT_MAIL_PUSH_PLUGIN_NAME, to = %parent.author_email, error = %err, "failed to send comment reply notification");
    }
    Ok(())
}

fn unique_emails(emails: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    emails
        .into_iter()
        .map(|email| email.trim().to_owned())
        .filter(|email| !email.is_empty())
        .filter(|email| seen.insert(email.to_ascii_lowercase()))
        .collect()
}

fn absolute_post_url(settings: &settings::SiteSettings, post: &posts::Model) -> String {
    let path = match &post.post_type {
        posts::PostType::Page => format!("/pages/{}", post.slug),
        posts::PostType::Post => format!("/posts/{}", post.slug),
    };
    settings
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|base_url| !base_url.is_empty())
        .map(|base_url| format!("{}{}", base_url.trim_end_matches('/'), path))
        .unwrap_or(path)
}

async fn site_settings(db: &DatabaseConnection) -> AppResult<SiteSettings> {
    let Some(row) = options::Entity::find()
        .filter(options::Column::Key.eq("site:settings"))
        .one(db)
        .await?
    else {
        return Ok(SiteSettings::default());
    };
    Ok(serde_json::from_value(row.value).unwrap_or_default())
}
