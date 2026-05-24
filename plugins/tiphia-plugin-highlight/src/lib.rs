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

use crate::config::HighlightConfig;

pub(crate) const HIGHLIGHT_PLUGIN_NAME: &str = "tiphia-highlight";

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(HighlightPlugin);
    Ok(())
}

pub struct HighlightPlugin;

static HIGHLIGHT_MANIFEST: PluginManifest = PluginManifest {
    name: HIGHLIGHT_PLUGIN_NAME,
    version: "0.1.0",
    description: "Adds configurable code block highlighting for public blog themes.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for HighlightPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &HIGHLIGHT_MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(db, self.manifest().name, json!(HighlightConfig::default())).await
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
