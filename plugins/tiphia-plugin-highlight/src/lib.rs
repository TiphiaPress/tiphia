use async_trait::async_trait;
use axum::{Json, Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tiphia_core::{
    AppResult, AppState,
    plugins::{
        Plugin, PluginConfigField, PluginConfigFieldType, PluginConfigSchema, PluginManifest,
        PluginRegistryBuilder, ensure_plugin_config, load_plugin_config,
    },
};
use tracing::info;

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(HighlightPlugin);
    Ok(())
}

pub struct HighlightPlugin;

static HIGHLIGHT_MANIFEST: PluginManifest = PluginManifest {
    name: "tiphia-highlight",
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
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "style",
                    label: "Highlight style",
                    field_type: PluginConfigFieldType::Text,
                    required: true,
                    default: Some(json!("github")),
                    help: Some("Allowed values: github, one_light, dracula, solarized_dark."),
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
            ],
        })
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(
            Router::new()
                .route("/highlight/config", get(config))
                .route("/highlight/config/", get(config)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HighlightConfig {
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_true")]
    pub mac_window: bool,
    #[serde(default = "default_true")]
    pub show_language: bool,
    #[serde(default)]
    pub line_wrap: bool,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            style: default_style(),
            mac_window: true,
            show_language: true,
            line_wrap: false,
        }
    }
}

async fn config(State(state): State<AppState>) -> AppResult<Json<HighlightConfig>> {
    let mut config = load_config(&state.db, HIGHLIGHT_MANIFEST.name).await?;
    config.style = normalize_style(&config.style).to_owned();
    Ok(Json(config))
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<HighlightConfig> {
    load_plugin_config(db, plugin_name, HighlightConfig::default()).await
}

fn normalize_style(style: &str) -> &'static str {
    match style.trim() {
        "github" | "one_light" | "dracula" | "solarized_dark" => match style.trim() {
            "one_light" => "one_light",
            "dracula" => "dracula",
            "solarized_dark" => "solarized_dark",
            _ => "github",
        },
        _ => "github",
    }
}

fn default_style() -> String {
    "github".to_owned()
}

fn default_true() -> bool {
    true
}
