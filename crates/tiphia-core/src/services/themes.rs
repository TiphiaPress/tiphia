use crate::{plugins::PluginConfigSchema, services::settings::ThemeSettings};
use serde::Serialize;
use serde_json::json;
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

pub fn list(active_theme: &str) -> Vec<ThemeInfo> {
    theme_definitions()
        .into_iter()
        .map(|mut theme| {
            theme.active = theme.name == active_theme;
            theme
        })
        .collect()
}

pub fn find(name: &str) -> Option<ThemeInfo> {
    theme_definitions()
        .into_iter()
        .find(|theme| theme.name == name)
}

pub fn normalize_settings(settings: ThemeSettings) -> ThemeSettings {
    let active = settings.active.trim().to_owned();
    let mut configs = if settings.configs.is_object() {
        settings.configs
    } else {
        json!({})
    };
    if !settings.configs_migrated
        && settings.config.is_object()
        && settings
            .config
            .as_object()
            .map(|object| !object.is_empty())
            .unwrap_or(false)
        && configs
            .get(&active)
            .and_then(serde_json::Value::as_object)
            .is_none()
        && let Some(configs_object) = configs.as_object_mut()
    {
        configs_object.insert(active.clone(), settings.config);
    }
    let config = if active.is_empty() {
        json!({})
    } else {
        configs
            .get(&active)
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| json!({}))
    };

    ThemeSettings {
        active,
        configs,
        config,
        configs_migrated: true,
    }
}

fn theme_definitions() -> Vec<ThemeInfo> {
    Vec::new()
}
