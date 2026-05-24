use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tiphia_core::{
    AppResult,
    entities::options,
    plugins::{merge_json_config, plugin_config_key},
};
use tracing::warn;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LinksConfig {
    #[serde(default)]
    pub links: Vec<LinkItem>,
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

pub async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<LinksConfig> {
    let current_key = plugin_config_key(plugin_name);
    if let Some(config) = load_config_by_key(db, &current_key).await? {
        if !config.links.is_empty() {
            return Ok(config);
        }
    }

    for legacy_name in legacy_plugin_names(plugin_name) {
        let legacy_key = plugin_config_key(legacy_name);
        if let Some(config) = load_config_by_key(db, &legacy_key).await? {
            if !config.links.is_empty() {
                migrate_legacy_config(db, &current_key, &config).await?;
                warn!(
                    plugin = plugin_name,
                    legacy_plugin = legacy_name,
                    "migrated legacy links plugin config"
                );
                return Ok(config);
            }
        }
    }

    Ok(LinksConfig::default())
}

fn legacy_plugin_names(plugin_name: &str) -> Vec<&'static str> {
    match plugin_name {
        "tiphia-links" => vec!["tiphia-plugin-links", "links"],
        _ => Vec::new(),
    }
}

async fn load_config_by_key(db: &DatabaseConnection, key: &str) -> AppResult<Option<LinksConfig>> {
    let Some(option) = options::Entity::find()
        .filter(options::Column::Key.eq(key.to_owned()))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(parse_config_value(option.value)?))
}

fn parse_config_value(value: Value) -> AppResult<LinksConfig> {
    let mut merged = json!(LinksConfig::default());
    merge_json_config(&mut merged, normalize_links_config(value));
    serde_json::from_value(merged)
        .map_err(|err| tiphia_core::error::AppError::Plugin(err.to_string()))
}

async fn migrate_legacy_config(
    db: &DatabaseConnection,
    current_key: &str,
    config: &LinksConfig,
) -> AppResult<()> {
    let now = chrono::Utc::now();
    let value = json!(config);
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(current_key.to_owned()))
        .one(db)
        .await?
    {
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(value);
        model.updated_at = Set(now);
        model.update(db).await?;
    } else {
        options::ActiveModel {
            key: Set(current_key.to_owned()),
            value: Set(value),
            autoload: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

fn unwrap_config_envelope(value: Value) -> Value {
    value.get("config").cloned().unwrap_or(value)
}

fn normalize_links_config(value: Value) -> Value {
    let value = unwrap_config_envelope(value);
    match value {
        Value::Array(_) => json!({ "links": value }),
        Value::Object(mut object) => {
            match object.remove("links") {
                Some(Value::Object(mut nested)) => {
                    if let Some(Value::Array(links)) = nested.remove("links") {
                        object.insert("links".to_owned(), Value::Array(links));
                    } else {
                        object.insert("links".to_owned(), Value::Object(nested));
                    }
                }
                Some(value) => {
                    object.insert("links".to_owned(), value);
                }
                None => {}
            }
            Value::Object(object)
        }
        value => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn merge_config(value: Value) -> LinksConfig {
        parse_config_value(value).expect("config")
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

    #[test]
    fn links_config_accepts_config_envelope() {
        let config = merge_config(json!({
            "config": {
                "links": [{ "name": "Envelope", "url": "https://example.com" }]
            }
        }));

        assert_eq!(config.links.len(), 1);
        assert_eq!(config.links[0].name, "Envelope");
    }
}
