use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SiteSettings {
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub gravatar_base_url: Option<String>,
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
            gravatar_base_url: Some("https://www.gravatar.com/avatar/".to_owned()),
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
