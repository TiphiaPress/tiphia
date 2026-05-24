use serde_json::json;
use tiphia_core::plugins::{PluginConfigField, PluginConfigFieldType, PluginConfigSchema};

pub fn config_schema() -> PluginConfigSchema {
    PluginConfigSchema {
        fields: vec![PluginConfigField {
            key: "links",
            label: "Friend links",
            field_type: PluginConfigFieldType::Json,
            required: false,
            default: Some(json!([])),
            help: Some(
                "Array of links: name, description, url, avatar_url, category. Categories are user-defined strings.",
            ),
        }],
    }
}
