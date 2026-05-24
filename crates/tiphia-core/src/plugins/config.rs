use crate::{
    entities::options,
    error::{AppError, AppResult},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

pub const PLUGIN_OPTION_PREFIX: &str = "plugin:";

pub fn plugin_state_key(plugin_name: &str) -> String {
    format!("{PLUGIN_OPTION_PREFIX}{plugin_name}:state")
}

pub fn plugin_config_key(plugin_name: &str) -> String {
    format!("{PLUGIN_OPTION_PREFIX}{plugin_name}:config")
}

pub fn default_plugin_state() -> Value {
    json!({ "enabled": false })
}

pub async fn ensure_plugin_config(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: Value,
) -> AppResult<()> {
    let key = plugin_config_key(plugin_name);
    let exists = options::Entity::find()
        .filter(options::Column::Key.eq(key.clone()))
        .one(db)
        .await?
        .is_some();

    if exists {
        return Ok(());
    }

    let now = Utc::now();
    options::ActiveModel {
        key: Set(key),
        value: Set(default_config),
        autoload: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

pub async fn load_plugin_config<T>(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: T,
) -> AppResult<T>
where
    T: DeserializeOwned + Serialize,
{
    load_plugin_config_with(db, plugin_name, default_config, |value| value).await
}

pub async fn load_plugin_config_with<T, F>(
    db: &DatabaseConnection,
    plugin_name: &str,
    default_config: T,
    normalize: F,
) -> AppResult<T>
where
    T: DeserializeOwned + Serialize,
    F: FnOnce(Value) -> Value,
{
    let default_value =
        serde_json::to_value(default_config).map_err(|err| AppError::Plugin(err.to_string()))?;
    let stored = options::Entity::find()
        .filter(options::Column::Key.eq(plugin_config_key(plugin_name)))
        .one(db)
        .await?
        .map(|option| option.value)
        .unwrap_or_else(|| default_value.clone());

    let mut merged = default_value;
    merge_json_config(&mut merged, normalize(unwrap_config_envelope(stored)));
    serde_json::from_value(merged).map_err(|err| AppError::Plugin(err.to_string()))
}

pub fn merge_json_config(base: &mut Value, patch: Value) {
    match (base, patch) {
        (Value::Object(base), Value::Object(patch)) => {
            for (key, value) in patch {
                merge_json_config(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, patch) => *base = patch,
    }
}

fn unwrap_config_envelope(value: Value) -> Value {
    value.get("config").cloned().unwrap_or(value)
}
