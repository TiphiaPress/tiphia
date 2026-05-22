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
    builder.register(FilingPlugin);
    Ok(())
}

pub struct FilingPlugin;

static FILING_MANIFEST: PluginManifest = PluginManifest {
    name: "tiphia-filing",
    version: "0.1.0",
    description: "Stores ICP and public security filing information for public frontends.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for FilingPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &FILING_MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(db, self.manifest().name, json!(FilingResponse::default())).await
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "icp_number",
                    label: "ICP filing number",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("Example: 京ICP备00000000号-1."),
                },
                PluginConfigField {
                    key: "icp_url",
                    label: "ICP filing URL",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("https://beian.miit.gov.cn/")),
                    help: Some("Usually https://beian.miit.gov.cn/."),
                },
                PluginConfigField {
                    key: "police_html",
                    label: "Public security filing HTML",
                    field_type: PluginConfigFieldType::Textarea,
                    required: false,
                    default: Some(json!("")),
                    help: Some("Custom sanitized HTML snippet for public security filing."),
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
                .route("/filing", get(filing))
                .route("/filing/", get(filing)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FilingResponse {
    #[serde(default)]
    pub icp_number: String,
    #[serde(default)]
    pub icp_url: String,
    #[serde(default)]
    pub police_html: String,
}

impl Default for FilingResponse {
    fn default() -> Self {
        Self {
            icp_number: String::new(),
            icp_url: "https://beian.miit.gov.cn/".to_owned(),
            police_html: String::new(),
        }
    }
}

async fn filing(State(state): State<AppState>) -> AppResult<Json<FilingResponse>> {
    let mut config = load_config(&state.db, FILING_MANIFEST.name).await?;
    config.police_html = ammonia::Builder::default()
        .link_rel(Some("noreferrer noopener"))
        .clean(&config.police_html)
        .to_string();
    Ok(Json(config))
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<FilingResponse> {
    load_plugin_config(db, plugin_name, FilingResponse::default()).await
}
