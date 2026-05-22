use crate::{
    entities::options,
    error::{AppError, AppResult},
    migration::{SharedMigration, run_plugin_migrations},
};
use async_trait::async_trait;
use axum::Router;
use axum::{
    extract::State,
    http::StatusCode,
    middleware::{self, Next},
    extract::Request,
    response::Response,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::Arc, time::Instant};
use tracing::{debug, warn};
use utoipa::ToSchema;

pub type SharedPlugin = Arc<dyn Plugin>;
pub const PLUGIN_OPTION_PREFIX: &str = "plugin:";

#[derive(Clone)]
pub struct PluginRegistry {
    db: DatabaseConnection,
    plugins: Vec<SharedPlugin>,
}

impl PluginRegistry {
    pub async fn boot<F>(db: DatabaseConnection, register_plugins: F) -> AppResult<Self>
    where
        F: FnOnce(&mut PluginRegistryBuilder) -> AppResult<()>,
    {
        let mut builder = PluginRegistryBuilder::new(db);
        register_plugins(&mut builder)?;
        builder.build().await
    }

    pub fn plugins(&self) -> &[SharedPlugin] {
        &self.plugins
    }

    pub async fn dispatch(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        context.attach_database(self.db.clone());

        let mut listeners = Vec::new();
        for plugin in &self.plugins {
            if !self.plugin_enabled(plugin.manifest().name).await? {
                continue;
            }
            if let Some(priority) = plugin.hooks().get(&hook).copied() {
                listeners.push((priority, plugin));
            }
        }

        listeners.sort_by_key(|(priority, _)| *priority);

        for (priority, plugin) in listeners {
            let started_at = Instant::now();
            let plugin_name = plugin.manifest().name;
            let result = plugin.handle(hook, context).await.map_err(|err| {
                AppError::Plugin(format!("{} failed on {hook:?}: {err}", plugin_name))
            });
            let elapsed_ms = started_at.elapsed().as_millis();

            match &result {
                Ok(()) => debug!(
                    plugin = plugin_name,
                    hook = ?hook,
                    priority,
                    elapsed_ms,
                    stopped = context.is_stopped(),
                    "plugin hook handled"
                ),
                Err(err) => warn!(
                    plugin = plugin_name,
                    hook = ?hook,
                    priority,
                    elapsed_ms,
                    error = %err,
                    "plugin hook failed"
                ),
            }

            result?;

            if context.is_stopped() {
                break;
            }
        }

        Ok(())
    }

    pub async fn plugin_enabled(&self, plugin_name: &str) -> AppResult<bool> {
        let enabled = options::Entity::find()
            .filter(options::Column::Key.eq(plugin_state_key(plugin_name)))
            .one(&self.db)
            .await?
            .and_then(|option| {
                option
                    .value
                    .get("enabled")
                    .and_then(serde_json::Value::as_bool)
            })
            .unwrap_or(false);

        Ok(enabled)
    }

    pub fn mount_routes(
        &self,
        mut router: Router<crate::app::AppState>,
    ) -> Router<crate::app::AppState> {
        for plugin in &self.plugins {
            if let (Some(prefix), Some(plugin_router)) =
                (plugin.route_prefix(), plugin.route_router())
            {
                let plugin_name = plugin.manifest().name;
                router = router.nest(
                    prefix,
                    plugin_router.layer(middleware::from_fn_with_state(
                        PluginRouteState {
                            plugin_name,
                            registry: self.clone(),
                        },
                        plugin_route_enabled,
                    )),
                );
            }
            router = plugin.routes(router);
        }
        router
    }
}

#[derive(Clone)]
struct PluginRouteState {
    plugin_name: &'static str,
    registry: PluginRegistry,
}

async fn plugin_route_enabled(
    State(state): State<PluginRouteState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match state.registry.plugin_enabled(state.plugin_name).await {
        Ok(true) => Ok(next.run(request).await),
        Ok(false) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub fn plugin_state_key(plugin_name: &str) -> String {
    format!("{PLUGIN_OPTION_PREFIX}{plugin_name}:state")
}

pub fn plugin_config_key(plugin_name: &str) -> String {
    format!("{PLUGIN_OPTION_PREFIX}{plugin_name}:config")
}

pub fn default_plugin_state() -> Value {
    json!({ "enabled": false })
}

pub async fn ensure_plugin_config(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: Value,
) -> AppResult<()> {
    let key = plugin_config_key(plugin_name);
    let exists = options::Entity::find()
        .filter(options::Column::Key.eq(key.clone()))
        .one(db)
        .await?
        .is_some();

    if exists {
        return Ok(());
    }

    let now = Utc::now();
    options::ActiveModel {
        key: Set(key),
        value: Set(default_config),
        autoload: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn load_plugin_config<T>(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: T,
) -> AppResult<T>
where
    T: DeserializeOwned + Serialize,
{
    load_plugin_config_with(db, plugin_name, default_config, |value| value).await
}

pub async fn load_plugin_config_with<T, F>(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: T,
    normalize: F,
) -> AppResult<T>
where
    T: DeserializeOwned + Serialize,
    F: FnOnce(Value) -> Value,
{
    let default_value =
        serde_json::to_value(default_config).map_err(|err| AppError::Plugin(err.to_string()))?;
    let stored = options::Entity::find()
        .filter(options::Column::Key.eq(plugin_config_key(plugin_name)))
        .one(db)
        .await?
        .map(|option| option.value)
        .unwrap_or_else(|| default_value.clone());

    let mut merged = default_value;
    merge_json_config(&mut merged, normalize(unwrap_config_envelope(stored)));
    serde_json::from_value(merged).map_err(|err| AppError::Plugin(err.to_string()))
}

pub fn merge_json_config(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                merge_json_config(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, patch) => *base = patch,
    }
}

fn unwrap_config_envelope(value: Value) -> Value {
    value.get("config").cloned().unwrap_or(value)
}

pub struct PluginRegistryBuilder {
    db: DatabaseConnection,
    plugins: Vec<SharedPlugin>,
}

impl PluginRegistryBuilder {
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            plugins: Vec::new(),
        }
    }

    pub fn database(&self) -> &DatabaseConnection {
        &self.db
    }

    pub fn register<P>(&mut self, plugin: P)
    where
        P: Plugin + 'static,
    {
        self.plugins.push(Arc::new(plugin));
    }

    async fn build(self) -> AppResult<PluginRegistry> {
        let registry = PluginRegistry {
            db: self.db.clone(),
            plugins: self.plugins,
        };

        for plugin in registry.plugins() {
            run_plugin_migrations(&self.db, plugin.as_ref()).await?;
            plugin.install(&self.db).await?;
            plugin.activate().await?;
        }

        Ok(registry)
    }
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &'static PluginManifest;

    async fn install(&self, _db: &DatabaseConnection) -> AppResult<()> {
        Ok(())
    }

    fn migrations(&self) -> Vec<SharedMigration> {
        Vec::new()
    }

    fn hooks(&self) -> HookMap {
        HookMap::default()
    }

    fn admin_menu(&self) -> Vec<AdminMenuItem> {
        Vec::new()
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        None
    }

    async fn activate(&self) -> AppResult<()> {
        Ok(())
    }

    async fn handle(&self, _hook: Hook, _context: &mut HookContext) -> AppResult<()> {
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        None
    }

    fn route_router(&self) -> Option<Router<crate::app::AppState>> {
        None
    }

    fn routes(&self, router: Router<crate::app::AppState>) -> Router<crate::app::AppState> {
        router
    }
}

pub type HookMap = BTreeMap<Hook, i32>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Hook {
    AppBooting,
    AppBooted,
    RequestReceived,
    BeforePostList,
    AfterPostList,
    BeforePostCreate,
    AfterPostCreate,
    BeforePostUpdate,
    AfterPostUpdate,
    BeforePostDelete,
    AfterPostDelete,
    BeforePageList,
    AfterPageList,
    BeforePageCreate,
    AfterPageCreate,
    BeforeCommentCreate,
    AfterCommentCreate,
    BeforeCommentModerate,
    AfterCommentModerate,
    BeforeAuthLogin,
    AfterAuthLogin,
    BeforeAuthRegister,
    AfterAuthRegister,
    BeforeAuthBootstrap,
    AfterAuthBootstrap,
    BeforeSettingsRead,
    AfterSettingsRead,
    BeforeSettingsUpdate,
    AfterSettingsUpdate,
    BeforeTermCreate,
    AfterTermCreate,
    BeforeTermUpdate,
    AfterTermUpdate,
    BeforeTermDelete,
    AfterTermDelete,
    BeforePostTermsSync,
    AfterPostTermsSync,
    BeforeRender,
    AfterRender,
    FrontendHead,
    FrontendHeader,
    FrontendFooter,
    FrontendAuthForm,
    FrontendCommentForm,
    FrontendPostContent,
    AdminMenu,
}

#[derive(Clone, Debug, Default)]
pub struct HookContext {
    db: Option<DatabaseConnection>,
    pub subject: Option<Value>,
    pub meta: BTreeMap<String, Value>,
    pub stopped: bool,
    pub stop_reason: Option<String>,
}

impl HookContext {
    pub fn with_subject<T>(subject: T) -> AppResult<Self>
    where
        T: Serialize,
    {
        Ok(Self {
            db: None,
            subject: Some(
                serde_json::to_value(subject).map_err(|err| AppError::Plugin(err.to_string()))?,
            ),
            meta: BTreeMap::new(),
            stopped: false,
            stop_reason: None,
        })
    }

    pub fn attach_database(&mut self, db: DatabaseConnection) {
        self.db = Some(db);
    }

    pub fn database(&self) -> AppResult<&DatabaseConnection> {
        self.db
            .as_ref()
            .ok_or_else(|| AppError::Plugin("hook context has no database".to_owned()))
    }

    pub fn subject_as<T>(&self) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.subject
            .clone()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| AppError::Plugin(err.to_string()))
    }

    pub fn take_subject<T>(&mut self) -> AppResult<Option<T>>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.subject
            .take()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| AppError::Plugin(err.to_string()))
    }

    pub fn replace_subject<T>(&mut self, subject: T) -> AppResult<()>
    where
        T: Serialize,
    {
        self.subject =
            Some(serde_json::to_value(subject).map_err(|err| AppError::Plugin(err.to_string()))?);
        Ok(())
    }

    pub fn insert_meta<T>(&mut self, key: impl Into<String>, value: T) -> AppResult<()>
    where
        T: Serialize,
    {
        self.meta.insert(
            key.into(),
            serde_json::to_value(value).map_err(|err| AppError::Plugin(err.to_string()))?,
        );
        Ok(())
    }

    pub fn stop(&mut self, reason: impl Into<String>) {
        self.stopped = true;
        self.stop_reason = Some(reason.into());
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    pub fn ensure_not_stopped(&self) -> AppResult<()> {
        if self.stopped {
            return Err(AppError::Plugin(
                self.stop_reason
                    .clone()
                    .unwrap_or_else(|| "hook stopped execution".to_owned()),
            ));
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub author: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct AdminMenuItem {
    pub label: &'static str,
    pub path: &'static str,
    pub icon: Option<&'static str>,
    pub order: i32,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PluginConfigSchema {
    pub fields: Vec<PluginConfigField>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PluginConfigField {
    pub key: &'static str,
    pub label: &'static str,
    pub field_type: PluginConfigFieldType,
    pub required: bool,
    pub default: Option<Value>,
    pub help: Option<&'static str>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PluginConfigFieldType {
    Text,
    Textarea,
    Number,
    Boolean,
    Json,
}
