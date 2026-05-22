use crate::{
    app::AppState,
    entities::options,
    error::{AppResult, validation_on_unique},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

const SITE_SETTINGS_KEY: &str = "site:settings";

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SiteSettings {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    pub base_url: Option<String>,
    pub timezone: String,
    pub default_page_size: u64,
    pub comments_enabled: bool,
    pub comment_moderation: bool,
    #[serde(default)]
    pub registration_enabled: bool,
    pub permalink_format: String,
    #[serde(default)]
    pub theme: ThemeSettings,
    #[serde(default)]
    pub seo: SeoSettings,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, ToSchema)]
pub struct SeoSettings {
    pub meta_title_suffix: Option<String>,
    pub meta_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ThemeSettings {
    pub active: String,
    #[serde(default)]
    pub configs: serde_json::Value,
    #[serde(default)]
    pub config: serde_json::Value,
    #[serde(default)]
    pub configs_migrated: bool,
}

impl Default for ThemeSettings {
    fn default() -> Self {
        Self {
            active: "".to_owned(),
            configs: json!({}),
            config: json!({}),
            configs_migrated: true,
        }
    }
}

impl Default for SiteSettings {
    fn default() -> Self {
        Self {
            title: "Tiphia".to_owned(),
            description: "A Rust blog powered by Tiphia.".to_owned(),
            avatar_url: None,
            base_url: None,
            timezone: "UTC".to_owned(),
            default_page_size: 20,
            comments_enabled: true,
            comment_moderation: true,
            registration_enabled: false,
            permalink_format: "/archives/{slug}".to_owned(),
            theme: ThemeSettings::default(),
            seo: SeoSettings {
                meta_title_suffix: None,
                meta_description: None,
            },
        }
    }
}

pub async fn get(state: &AppState) -> AppResult<SiteSettings> {
    let mut settings: SiteSettings = options::Entity::find()
        .filter(options::Column::Key.eq(SITE_SETTINGS_KEY))
        .one(&state.db)
        .await?
        .map(|option| serde_json::from_value(option.value))
        .transpose()
        .map_err(|err| crate::error::AppError::Validation(err.to_string()))?
        .unwrap_or_default();
    settings.theme = crate::services::themes::normalize_settings(settings.theme);

    Ok(settings)
}

pub async fn update(state: &AppState, mut settings: SiteSettings) -> AppResult<SiteSettings> {
    settings.theme = crate::services::themes::normalize_settings(settings.theme);
    validate(&settings)?;

    let now = Utc::now();
    let value = json!(settings);
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(SITE_SETTINGS_KEY))
        .one(&state.db)
        .await?
    {
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(value);
        model.updated_at = Set(now);
        model.update(&state.db).await?;
    } else {
        options::ActiveModel {
            key: Set(SITE_SETTINGS_KEY.to_owned()),
            value: Set(value),
            autoload: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "site settings already exist"))?;
    }

    Ok(settings)
}

fn validate(settings: &SiteSettings) -> AppResult<()> {
    if settings.title.trim().is_empty() {
        return Err(crate::error::AppError::Validation(
            "title is required".to_owned(),
        ));
    }

    if !(1..=100).contains(&settings.default_page_size) {
        return Err(crate::error::AppError::Validation(
            "default_page_size must be between 1 and 100".to_owned(),
        ));
    }

    if !settings.permalink_format.contains("{slug}") {
        return Err(crate::error::AppError::Validation(
            "permalink_format must contain {slug}".to_owned(),
        ));
    }

    if let Some(avatar_url) = settings
        .avatar_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        validate_avatar_url(avatar_url)?;
    }

    Ok(())
}

fn validate_avatar_url(value: &str) -> AppResult<()> {
    if value.starts_with('/') && !value.starts_with("//") && !value.chars().any(char::is_whitespace)
    {
        return Ok(());
    }

    crate::services::validation::optional_http_url(Some(value), "avatar_url")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avatar_url_accepts_http_https_or_site_relative_paths() {
        assert!(validate_avatar_url("https://example.com/avatar.png").is_ok());
        assert!(validate_avatar_url("/assets/avatar.png").is_ok());
        assert!(validate_avatar_url("avatar.png").is_err());
        assert!(validate_avatar_url("//example.com/avatar.png").is_err());
    }
}
