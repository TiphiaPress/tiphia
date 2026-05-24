use crate::plugins::PluginConfigSchema;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ThemeInfo {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub author: &'static str,
    pub preview: &'static str,
    pub active: bool,
    pub schema: PluginConfigSchema,
}
