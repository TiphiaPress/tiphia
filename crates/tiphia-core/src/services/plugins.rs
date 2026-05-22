use crate::{
    app::AppState,
    entities::options,
    error::{AppError, AppResult, validation_on_unique},
    plugins::{
        AdminMenuItem, PluginConfigField, PluginConfigFieldType, PluginConfigSchema,
        PluginManifest, plugin_config_key, plugin_state_key,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize)]
pub struct PluginInfo {
    pub manifest: &'static PluginManifest,
    pub admin_menu: Vec<AdminMenuItem>,
    pub config_schema: Option<PluginConfigSchema>,
    pub hooks: Vec<PluginHookInfo>,
    pub health: PluginHealth,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginHookInfo {
    pub hook: String,
    pub priority: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginHealth {
    pub installed: bool,
    pub active: bool,
    pub hook_count: usize,
    pub admin_menu_count: usize,
    pub configurable: bool,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PluginStateResponse {
    pub plugin: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdatePluginStateInput {
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PluginConfigResponse {
    pub plugin: String,
    pub config: Value,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct UpdatePluginConfigInput {
    pub config: Value,
}

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

    let now = Utc::now();
    let key = plugin_state_key(plugin_name);
    let value = json!({ "enabled": input.enabled });
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(key.clone()))
        .one(&state.db)
        .await?
    {
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(value);
        model.updated_at = Set(now);
        model.update(&state.db).await?;
    } else {
        options::ActiveModel {
            key: Set(key),
            value: Set(value),
            autoload: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "plugin state already exists"))?;
    }

    Ok(PluginStateResponse {
        plugin: plugin_name.to_owned(),
        enabled: input.enabled,
    })
}

pub async fn get_config(state: &AppState, plugin_name: &str) -> AppResult<PluginConfigResponse> {
    ensure_plugin_exists(state, plugin_name)?;

    let config = options::Entity::find()
        .filter(options::Column::Key.eq(plugin_config_key(plugin_name)))
        .one(&state.db)
        .await?
        .map(|option| option.value)
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
    let schema = plugin_schema(state, plugin_name)?;
    if let Some(schema) = &schema {
        validate_config(schema, &input.config)?;
    }

    let now = Utc::now();
    let key = plugin_config_key(plugin_name);
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(key.clone()))
        .one(&state.db)
        .await?
    {
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(input.config.clone());
        model.updated_at = Set(now);
        model.update(&state.db).await?;
    } else {
        options::ActiveModel {
            key: Set(key),
            value: Set(input.config.clone()),
            autoload: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "plugin config already exists"))?;
    }

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

fn validate_config(schema: &PluginConfigSchema, config: &Value) -> AppResult<()> {
    let object = config
        .as_object()
        .ok_or_else(|| AppError::Validation("plugin config must be a JSON object".to_owned()))?;

    for field in &schema.fields {
        match object.get(field.key) {
            Some(value) => validate_field(field, value)?,
            None if field.required => {
                return Err(AppError::Validation(format!(
                    "plugin config field `{}` is required",
                    field.key
                )));
            }
            None => {}
        }
    }

    Ok(())
}

fn validate_field(field: &PluginConfigField, value: &Value) -> AppResult<()> {
    let valid = match field.field_type {
        PluginConfigFieldType::Text | PluginConfigFieldType::Textarea => value.is_string(),
        PluginConfigFieldType::Number => value.is_number(),
        PluginConfigFieldType::Boolean => value.is_boolean(),
        PluginConfigFieldType::Json => true,
    };

    if !valid {
        return Err(AppError::Validation(format!(
            "plugin config field `{}` must be {}",
            field.key,
            field_type_name(&field.field_type)
        )));
    }

    Ok(())
}

fn field_type_name(field_type: &PluginConfigFieldType) -> &'static str {
    match field_type {
        PluginConfigFieldType::Text | PluginConfigFieldType::Textarea => "a string",
        PluginConfigFieldType::Number => "a number",
        PluginConfigFieldType::Boolean => "a boolean",
        PluginConfigFieldType::Json => "valid JSON",
    }
}
