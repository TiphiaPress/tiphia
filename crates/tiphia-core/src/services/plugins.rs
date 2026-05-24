use crate::{
    app::AppState,
    error::{AppError, AppResult},
    plugins::{AdminMenuItem, PluginConfigSchema, plugin_config_key, plugin_state_key},
};
use serde_json::json;

#[path = "plugins/model.rs"]
mod model;
#[path = "plugins/schema.rs"]
mod schema;

pub use model::{
    PluginConfigResponse, PluginHealth, PluginHookInfo, PluginInfo, PluginStateResponse,
    UpdatePluginConfigInput, UpdatePluginStateInput,
};

pub async fn list(state: &AppState) -> AppResult<Vec<PluginInfo>> {
    let mut plugins = Vec::new();
    for plugin in state.plugins.plugins() {
        let enabled = state.plugins.plugin_enabled(plugin.manifest().name).await?;
        let admin_menu = plugin.admin_menu();
        let config_schema = plugin.config_schema();
        let hooks = plugin
            .hooks()
            .into_iter()
            .map(|(hook, priority)| PluginHookInfo {
                hook: format!("{hook:?}"),
                priority,
            })
            .collect::<Vec<_>>();
        plugins.push(PluginInfo {
            manifest: plugin.manifest(),
            health: PluginHealth {
                installed: true,
                active: enabled,
                hook_count: hooks.len(),
                admin_menu_count: admin_menu.len(),
                configurable: config_schema.is_some(),
            },
            admin_menu,
            config_schema,
            hooks,
        });
    }

    Ok(plugins)
}

pub async fn admin_menu(state: &AppState) -> AppResult<Vec<AdminMenuItem>> {
    let mut items = Vec::new();
    for plugin in state.plugins.plugins() {
        if state.plugins.plugin_enabled(plugin.manifest().name).await? {
            items.extend(plugin.admin_menu());
        }
    }
    items.sort_by_key(|item| item.order);
    Ok(items)
}

pub async fn get_state(state: &AppState, plugin_name: &str) -> AppResult<PluginStateResponse> {
    ensure_plugin_exists(state, plugin_name)?;
    Ok(PluginStateResponse {
        plugin: plugin_name.to_owned(),
        enabled: state.plugins.plugin_enabled(plugin_name).await?,
    })
}

pub async fn update_state(
    state: &AppState,
    plugin_name: &str,
    input: UpdatePluginStateInput,
) -> AppResult<PluginStateResponse> {
    ensure_plugin_exists(state, plugin_name)?;

    let key = plugin_state_key(plugin_name);
    crate::services::options::upsert_json(state, &key, json!({ "enabled": input.enabled }), true)
        .await?;

    Ok(PluginStateResponse {
        plugin: plugin_name.to_owned(),
        enabled: input.enabled,
    })
}

pub async fn get_config(state: &AppState, plugin_name: &str) -> AppResult<PluginConfigResponse> {
    ensure_plugin_exists(state, plugin_name)?;

    let config = crate::services::options::get_json(state, &plugin_config_key(plugin_name))
        .await?
        .unwrap_or_else(|| json!({}));

    Ok(PluginConfigResponse {
        plugin: plugin_name.to_owned(),
        config,
    })
}

pub async fn update_config(
    state: &AppState,
    plugin_name: &str,
    input: UpdatePluginConfigInput,
) -> AppResult<PluginConfigResponse> {
    let plugin_schema = plugin_schema(state, plugin_name)?;
    if let Some(plugin_schema) = &plugin_schema {
        schema::validate_config(plugin_schema, &input.config)?;
    }

    let key = plugin_config_key(plugin_name);
    crate::services::options::upsert_json(state, &key, input.config.clone(), false).await?;

    Ok(PluginConfigResponse {
        plugin: plugin_name.to_owned(),
        config: input.config,
    })
}

fn ensure_plugin_exists(state: &AppState, plugin_name: &str) -> AppResult<()> {
    if state
        .plugins
        .plugins()
        .iter()
        .any(|plugin| plugin.manifest().name == plugin_name)
    {
        return Ok(());
    }

    Err(AppError::NotFound("plugin"))
}

fn plugin_schema(state: &AppState, plugin_name: &str) -> AppResult<Option<PluginConfigSchema>> {
    state
        .plugins
        .plugins()
        .iter()
        .find(|plugin| plugin.manifest().name == plugin_name)
        .map(|plugin| plugin.config_schema())
        .ok_or(AppError::NotFound("plugin"))
}
