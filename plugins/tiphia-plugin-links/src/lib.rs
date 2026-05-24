mod config;
mod routes;
mod schema;

use async_trait::async_trait;
use axum::Router;
use sea_orm::DatabaseConnection;
use serde_json::json;
use tiphia_core::{
    AppResult, AppState,
    plugins::{
        Plugin, PluginConfigSchema, PluginManifest, PluginRegistryBuilder, ensure_plugin_config,
    },
};
use tracing::info;

use crate::config::LinksConfig;

pub(crate) const LINKS_PLUGIN_NAME: &str = "tiphia-links";

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(LinksPlugin);
    Ok(())
}

pub struct LinksPlugin;

static LINKS_MANIFEST: PluginManifest = PluginManifest {
    name: LINKS_PLUGIN_NAME,
    version: "0.1.0",
    description: "Stores friend links and exposes them to public frontends.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for LinksPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &LINKS_MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(db, self.manifest().name, json!(LinksConfig::default())).await
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(schema::config_schema())
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(routes::router())
    }
}
