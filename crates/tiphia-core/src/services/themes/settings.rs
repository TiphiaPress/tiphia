use crate::services::settings::ThemeSettings;
use serde_json::json;

pub fn normalize_settings(settings: ThemeSettings) -> ThemeSettings {
    let active = settings.active.trim().to_owned();
    let mut configs = if settings.configs.is_object() {
        settings.configs
    } else {
        json!({})
    };
    migrate_legacy_config(
        &active,
        &mut configs,
        settings.configs_migrated,
        settings.config,
    );
    let config = active_config(&active, &configs);

    ThemeSettings {
        active,
        configs,
        config,
        configs_migrated: true,
    }
}

fn migrate_legacy_config(
    active: &str,
    configs: &mut serde_json::Value,
    configs_migrated: bool,
    legacy_config: serde_json::Value,
) {
    if configs_migrated || !legacy_config.is_object() || active.is_empty() {
        return;
    }

    let has_legacy_values = legacy_config
        .as_object()
        .map(|object| !object.is_empty())
        .unwrap_or(false);
    if !has_legacy_values {
        return;
    }

    let active_missing = configs
        .get(active)
        .and_then(serde_json::Value::as_object)
        .is_none();
    if active_missing && let Some(configs_object) = configs.as_object_mut() {
        configs_object.insert(active.to_owned(), legacy_config);
    }
}

fn active_config(active: &str, configs: &serde_json::Value) -> serde_json::Value {
    if active.is_empty() {
        return json!({});
    }

    configs
        .get(active)
        .filter(|value| value.is_object())
        .cloned()
        .unwrap_or_else(|| json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_config_migrates_to_active_theme_once() {
        let settings = ThemeSettings {
            active: "default".to_owned(),
            configs: json!({}),
            config: json!({ "accent": "#2563eb" }),
            configs_migrated: false,
        };
        let normalized = normalize_settings(settings);
        assert_eq!(normalized.config["accent"], "#2563eb");
        assert_eq!(normalized.configs["default"]["accent"], "#2563eb");
        assert!(normalized.configs_migrated);
    }

    #[test]
    fn empty_active_theme_returns_empty_config() {
        let normalized = normalize_settings(ThemeSettings::default());
        assert_eq!(normalized.config, json!({}));
    }
}
