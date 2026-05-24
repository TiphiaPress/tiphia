use crate::{
    error::{AppError, AppResult},
    migration::run_plugin_migrations,
    plugins::{Hook, HookContext, Plugin, SharedPlugin, config::plugin_state_key},
};
use axum::{
    Router,
    extract::{Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::Response,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use std::{sync::Arc, time::Instant};
use tracing::{debug, warn};

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
        let enabled = crate::entities::options::Entity::find()
            .filter(crate::entities::options::Column::Key.eq(plugin_state_key(plugin_name)))
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
