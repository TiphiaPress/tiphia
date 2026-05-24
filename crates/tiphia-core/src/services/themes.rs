#[path = "themes/model.rs"]
mod model;
#[path = "themes/settings.rs"]
mod settings;

pub use model::ThemeInfo;
pub use settings::normalize_settings;

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

fn theme_definitions() -> Vec<ThemeInfo> {
    Vec::new()
}
