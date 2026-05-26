mod support;

use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    routing::get,
};
use std::collections::BTreeMap;
use tiphia_core::{
    AppError, AppResult, AppState, build_router_with_plugins,
    plugins::{
        Hook, HookContext, HookMap, Plugin, PluginConfigField, PluginConfigFieldType,
        PluginConfigSchema, PluginManifest,
    },
    services::{
        auth::{self, BootstrapAdminInput},
        comments::{self, CreateCommentInput},
        plugins::{self, UpdatePluginConfigInput},
        posts::{self, CreatePostInput},
    },
};
use tower::ServiceExt;

static MANIFEST: PluginManifest = PluginManifest {
    name: "test-comment-prefix",
    version: "0.1.0",
    description: "Test plugin that rewrites comment content.",
    author: "Tiphia Tests",
};

struct CommentPrefixPlugin;

#[async_trait]
impl Plugin for CommentPrefixPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn hooks(&self) -> HookMap {
        BTreeMap::from([(Hook::BeforeCommentCreate, 10)])
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "enabled",
                    label: "Enabled",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(serde_json::json!(true)),
                    help: None,
                },
                PluginConfigField {
                    key: "prefix",
                    label: "Prefix",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(serde_json::json!("[plugin]")),
                    help: None,
                },
            ],
        })
    }

    async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        if hook == Hook::BeforeCommentCreate {
            let mut input = context
                .take_subject::<CreateCommentInput>()?
                .expect("comment input");
            input.content = format!("[plugin] {}", input.content);
            context.replace_subject(input)?;
        }

        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(Router::new().route("/test-plugin", get(|| async { "ok" })))
    }
}

#[tokio::test]
async fn plugin_routes_receive_global_cors_headers() {
    let config = support::config();
    let db = support::database().await;
    enable_plugin(&db, MANIFEST.name).await;
    let app = build_router_with_plugins(db, config, |plugins| {
        plugins.register(CommentPrefixPlugin);
        Ok(())
    })
    .await
    .expect("router");

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/test-plugin")
                .header(header::ORIGIN, "http://127.0.0.1:5174")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("plugin response");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|value| value.to_str().ok()),
        Some("*")
    );
    let body = to_bytes(response.into_body(), 1024).await.expect("body");
    assert_eq!(&body[..], b"ok");
}

#[tokio::test]
async fn plugin_can_rewrite_comment_create_input() {
    let state = support::state_with_plugins(|plugins| {
        plugins.register(CommentPrefixPlugin);
        Ok(())
    })
    .await;
    plugins::update_state(
        &state,
        "test-comment-prefix",
        plugins::UpdatePluginStateInput { enabled: true },
    )
    .await
    .expect("enable plugin");

    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    let post = posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "plugin-post".to_owned(),
            title: "Plugin Post".to_owned(),
            markdown: "content".to_owned(),
            html: None,
            excerpt: None,
            status: None,
            post_type: None,
            published_at: None,
            extensions: Default::default(),
        },
    )
    .await
    .expect("create post");

    let comment = comments::create(
        &state,
        CreateCommentInput {
            post_id: post.id,
            parent_id: None,
            author_name: "Alice".to_owned(),
            author_email: "alice@example.com".to_owned(),
            author_url: None,
            content: "hello".to_owned(),
            extensions: Default::default(),
            captcha: None,
        },
    )
    .await
    .expect("create comment");

    assert_eq!(comment.content, "[plugin] hello");
}

#[tokio::test]
async fn disabled_plugin_does_not_run_hooks_or_admin_menu() {
    let state = support::state_with_plugins(|plugins| {
        plugins.register(CommentPrefixPlugin);
        Ok(())
    })
    .await;

    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    let post = posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "disabled-plugin-post".to_owned(),
            title: "Disabled Plugin Post".to_owned(),
            markdown: "content".to_owned(),
            html: None,
            excerpt: None,
            status: None,
            post_type: None,
            published_at: None,
            extensions: Default::default(),
        },
    )
    .await
    .expect("create post");

    let comment = comments::create(
        &state,
        CreateCommentInput {
            post_id: post.id,
            parent_id: None,
            author_name: "Alice".to_owned(),
            author_email: "alice@example.com".to_owned(),
            author_url: None,
            content: "hello".to_owned(),
            extensions: Default::default(),
            captcha: None,
        },
    )
    .await
    .expect("create comment");

    assert_eq!(comment.content, "hello");

    let listed = plugins::list(&state).await.expect("list plugins");
    assert!(!listed[0].health.active);
}

async fn enable_plugin(db: &sea_orm::DatabaseConnection, name: &str) {
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, Set};
    use tiphia_core::{entities::options, plugins::plugin_state_key};

    let now = Utc::now();
    options::ActiveModel {
        key: Set(plugin_state_key(name)),
        value: Set(serde_json::json!({ "enabled": true })),
        autoload: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("enable plugin");
}

#[tokio::test]
async fn plugin_config_schema_rejects_invalid_values() {
    let state = support::state_with_plugins(|plugins| {
        plugins.register(CommentPrefixPlugin);
        Ok(())
    })
    .await;

    let err = plugins::update_config(
        &state,
        "test-comment-prefix",
        UpdatePluginConfigInput {
            config: serde_json::json!({
                "enabled": "yes",
                "prefix": "[test]"
            }),
        },
    )
    .await
    .expect_err("invalid config should fail");

    assert!(matches!(err, AppError::Validation(_)));

    let config = plugins::update_config(
        &state,
        "test-comment-prefix",
        UpdatePluginConfigInput {
            config: serde_json::json!({
                "enabled": true,
                "prefix": "[test]"
            }),
        },
    )
    .await
    .expect("valid config should save");

    assert_eq!(config.config["enabled"], true);
}
