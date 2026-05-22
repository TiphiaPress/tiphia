use async_trait::async_trait;
use axum::{Json, Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tiphia_core::{
    AppResult, AppState,
    plugins::{
        Plugin, PluginConfigField, PluginConfigFieldType, PluginConfigSchema, PluginManifest,
        PluginRegistryBuilder, ensure_plugin_config, load_plugin_config_with,
    },
};
use tracing::info;

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(LinksPlugin);
    Ok(())
}

pub struct LinksPlugin;

static LINKS_MANIFEST: PluginManifest = PluginManifest {
    name: "tiphia-links",
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
        Some(PluginConfigSchema {
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
                .route("/links", get(list_links))
                .route("/links/", get(list_links)),
        )
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct LinksConfig {
    #[serde(default)]
    links: Vec<LinkItem>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LinkItem {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub url: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
}

async fn list_links(State(state): State<AppState>) -> AppResult<Json<Vec<LinkItem>>> {
    Ok(Json(load_config(&state.db, LINKS_MANIFEST.name).await?.links))
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<LinksConfig> {
    load_plugin_config_with(db, plugin_name, LinksConfig::default(), normalize_links_config).await
}

fn normalize_links_config(value: Value) -> Value {
    match value {
        Value::Array(_) => json!({ "links": value }),
        Value::Object(mut object) => {
            if let Some(Value::Object(mut nested)) = object.remove("links") {
                if let Some(Value::Array(links)) = nested.remove("links") {
                    object.insert("links".to_owned(), Value::Array(links));
                    return Value::Object(object);
                }
                object.insert("links".to_owned(), Value::Object(nested));
            }
            Value::Object(object)
        }
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiphia_core::plugins::merge_json_config;

    fn merge_config(value: Value) -> LinksConfig {
        let mut base = json!(LinksConfig::default());
        merge_json_config(&mut base, normalize_links_config(value));
        serde_json::from_value(base).expect("config")
    }

    #[test]
    fn links_config_accepts_nested_links_from_json_field() {
        let config = merge_config(json!({
            "links": {
                "links": [
                    {
                        "name": "Rust",
                        "url": "https://www.rust-lang.org/",
                        "description": "Rust",
                        "avatar_url": null,
                        "category": "Tech"
                    }
                ]
            }
        }));

        assert_eq!(config.links.len(), 1);
        assert_eq!(config.links[0].name, "Rust");
    }

    #[test]
    fn links_config_accepts_array_as_root() {
        let config = merge_config(json!([
            {
                "name": "Tiphia",
                "url": "https://example.com/"
            }
        ]));

        assert_eq!(config.links.len(), 1);
        assert_eq!(config.links[0].name, "Tiphia");
    }
}
