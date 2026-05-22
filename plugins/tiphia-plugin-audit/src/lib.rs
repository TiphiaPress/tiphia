use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    PaginatorTrait, QueryOrder, Schema, Set,
};
use sea_orm::{DeriveEntityModel, DeriveRelation, EnumIter, entity::prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tiphia_core::{
    AppResult, AppState,
    migration::{Migration, SharedMigration},
    pagination::{Page, PaginationQuery},
    plugins::{
        AdminMenuItem, Hook, HookContext, HookMap, Plugin, PluginConfigField,
        PluginConfigFieldType, PluginConfigSchema, PluginManifest, PluginRegistryBuilder,
        ensure_plugin_config, load_plugin_config,
    },
    routes::auth::CurrentUser,
    services::comments::CreateCommentInput,
};
use tracing::info;

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(AuditPlugin);
    Ok(())
}

pub struct AuditPlugin;

static AUDIT_MANIFEST: PluginManifest = PluginManifest {
    name: "tiphia-audit",
    version: "0.1.0",
    description: "Audits content lifecycle hooks and exposes a plugin-owned status endpoint.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for AuditPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &AUDIT_MANIFEST
    }

    fn hooks(&self) -> HookMap {
        [
            (Hook::AppBooted, 50),
            (Hook::AfterPostCreate, 100),
            (Hook::AfterPostUpdate, 100),
            (Hook::BeforeCommentCreate, 90),
            (Hook::AfterCommentCreate, 100),
        ]
        .into_iter()
        .collect()
    }

    fn migrations(&self) -> Vec<SharedMigration> {
        vec![Box::new(CreateAuditEvents)]
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(db, self.manifest().name, json!(AuditConfig::default())).await
    }

    fn admin_menu(&self) -> Vec<AdminMenuItem> {
        vec![AdminMenuItem {
            label: "Audit",
            path: "/admin/plugins/audit",
            icon: Some("activity"),
            order: 500,
        }]
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "log_post_events",
                    label: "Log post events",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(json!(true)),
                    help: Some("Record post create and update lifecycle hooks."),
                },
                PluginConfigField {
                    key: "log_comment_events",
                    label: "Log comment events",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(json!(true)),
                    help: Some("Record comment lifecycle hooks."),
                },
                PluginConfigField {
                    key: "blocked_comment_words",
                    label: "Blocked comment words",
                    field_type: PluginConfigFieldType::Json,
                    required: false,
                    default: Some(json!([])),
                    help: Some("Reject comments containing any configured word."),
                },
            ],
        })
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        let config = load_config(context.database()?, self.manifest().name).await?;
        if matches!(hook, Hook::BeforeCommentCreate)
            && let Some(input) = context.subject_as::<CreateCommentInput>()?
        {
            let lower_content = input.content.to_lowercase();
            if config
                .blocked_comment_words
                .iter()
                .map(|word| word.trim().to_lowercase())
                .any(|word| !word.is_empty() && lower_content.contains(&word))
            {
                context.stop("comment rejected by audit plugin");
                return Ok(());
            }
        }

        info!(
            plugin = self.manifest().name,
            ?hook,
            subject = ?context.subject,
            meta = ?context.meta,
            "audit plugin hook handled"
        );

        if !should_log(hook, &config) {
            return Ok(());
        }

        audit_events::ActiveModel {
            event: Set(format!("{hook:?}")),
            subject: Set(json!({
                "subject": context.subject,
                "meta": context.meta,
            })),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(context.database()?)
        .await?;
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1/audit")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(
            Router::new()
                .route("/status", get(status))
                .route("/events", get(events)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuditConfig {
    log_post_events: bool,
    log_comment_events: bool,
    blocked_comment_words: Vec<String>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            log_post_events: true,
            log_comment_events: true,
            blocked_comment_words: Vec::new(),
        }
    }
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<AuditConfig> {
    load_plugin_config(db, plugin_name, AuditConfig::default()).await
}

fn should_log(hook: Hook, config: &AuditConfig) -> bool {
    match hook {
        Hook::AfterPostCreate | Hook::AfterPostUpdate => config.log_post_events,
        Hook::BeforeCommentCreate | Hook::AfterCommentCreate => config.log_comment_events,
        _ => true,
    }
}

async fn status() -> Json<AuditStatus> {
    Json(AuditStatus {
        plugin: AUDIT_MANIFEST.name,
        enabled: true,
    })
}

#[derive(serde::Serialize)]
struct AuditStatus {
    plugin: &'static str,
    enabled: bool,
}

#[derive(serde::Deserialize)]
struct AuditEventQuery {
    #[serde(flatten)]
    pagination: PaginationQuery,
}

async fn events(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(query): Query<AuditEventQuery>,
) -> AppResult<Json<Page<audit_events::Model>>> {
    current_user.0.require_editor()?;

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = audit_events::Entity::find()
        .order_by_desc(audit_events::Column::CreatedAt)
        .paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Json(Page::new(items, page, per_page, total, total_pages)))
}

struct CreateAuditEvents;

#[async_trait]
impl Migration for CreateAuditEvents {
    fn id(&self) -> &'static str {
        "plugin:tiphia-audit:0001:create-audit-events"
    }

    async fn up(&self, db: &DatabaseTransaction) -> AppResult<()> {
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        let statement = schema
            .create_table_from_entity(audit_events::Entity)
            .if_not_exists()
            .to_owned();
        db.execute(backend.build(&statement)).await?;
        Ok(())
    }
}

mod audit_events {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize)]
    #[sea_orm(table_name = "audit_events")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub event: String,
        pub subject: sea_orm::prelude::Json,
        pub created_at: DateTimeUtc,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
