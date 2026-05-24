use crate::{app::AppState, error::AppResult};

#[path = "settings/model.rs"]
mod model;
#[path = "settings/validation.rs"]
mod validation;

pub use model::{SeoSettings, SiteSettings, ThemeSettings};

const SITE_SETTINGS_KEY: &str = "site:settings";

pub async fn get(state: &AppState) -> AppResult<SiteSettings> {
    let mut settings: SiteSettings = crate::services::options::get_typed(state, SITE_SETTINGS_KEY)
        .await?
        .unwrap_or_default();
    settings.theme = crate::services::themes::normalize_settings(settings.theme);

    Ok(settings)
}

pub async fn update(state: &AppState, mut settings: SiteSettings) -> AppResult<SiteSettings> {
    settings.theme = crate::services::themes::normalize_settings(settings.theme);
    validation::validate(&settings)?;
    crate::services::options::upsert_typed(state, SITE_SETTINGS_KEY, &settings, true).await?;
    Ok(settings)
}
