use crate::plugins::{AdminMenuItem, PluginConfigSchema, PluginManifest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize)]
pub struct PluginInfo {
    pub manifest: &'static PluginManifest,
    pub admin_menu: Vec<AdminMenuItem>,
    pub config_schema: Option<PluginConfigSchema>,
    pub hooks: Vec<PluginHookInfo>,
    pub health: PluginHealth,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginHookInfo {
    pub hook: String,
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginHealth {
    pub installed: bool,
    pub active: bool,
    pub hook_count: usize,
    pub admin_menu_count: usize,
    pub configurable: bool,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PluginStateResponse {
    pub plugin: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdatePluginStateInput {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginConfigResponse {
    pub plugin: String,
    pub config: Value,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdatePluginConfigInput {
    pub config: Value,
}
