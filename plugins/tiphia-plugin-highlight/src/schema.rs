use serde_json::json;
use tiphia_core::plugins::{PluginConfigField, PluginConfigFieldType, PluginConfigSchema};

pub fn config_schema() -> PluginConfigSchema {
    PluginConfigSchema {
        fields: vec![
            PluginConfigField {
                key: "style",
                label: "Highlight style",
                field_type: PluginConfigFieldType::Text,
                required: true,
                default: Some(json!("GrayMac.css")),
                help: Some(
                    "Allowed values: BlackMac.css, coy.css, dark.css, default.css, GrayMac.css, solarized-light.css, tomorrow-night.css, twilight.css, WhiteMac.css.",
                ),
            },
            PluginConfigField {
                key: "mac_window",
                label: "Mac style window border",
                field_type: PluginConfigFieldType::Boolean,
                required: true,
                default: Some(json!(true)),
                help: Some("Render code blocks with macOS-like traffic-light dots."),
            },
            PluginConfigField {
                key: "show_language",
                label: "Show language label",
                field_type: PluginConfigFieldType::Boolean,
                required: true,
                default: Some(json!(true)),
                help: Some("Show detected language name in the code block header."),
            },
            PluginConfigField {
                key: "line_wrap",
                label: "Wrap long lines",
                field_type: PluginConfigFieldType::Boolean,
                required: true,
                default: Some(json!(false)),
                help: Some("Wrap long code lines instead of horizontal scrolling."),
            },
            PluginConfigField {
                key: "line_numbers",
                label: "Show line numbers",
                field_type: PluginConfigFieldType::Boolean,
                required: true,
                default: Some(json!(false)),
                help: Some("Render unselectable line numbers beside code lines."),
            },
        ],
    }
}
