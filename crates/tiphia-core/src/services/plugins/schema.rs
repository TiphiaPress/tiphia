use crate::{
    error::{AppError, AppResult},
    plugins::{PluginConfigField, PluginConfigFieldType, PluginConfigSchema},
};
use serde_json::Value;

pub fn validate_config(schema: &PluginConfigSchema, config: &Value) -> AppResult<()> {
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
