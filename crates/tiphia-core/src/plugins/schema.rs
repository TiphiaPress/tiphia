use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

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
