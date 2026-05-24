use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct HighlightConfig {
    #[serde(default = "default_style")]
    pub style: String,
    #[serde(default = "default_true")]
    pub mac_window: bool,
    #[serde(default = "default_true")]
    pub show_language: bool,
    #[serde(default)]
    pub line_wrap: bool,
    #[serde(default)]
    pub line_numbers: bool,
}

impl Default for HighlightConfig {
    fn default() -> Self {
        Self {
            style: default_style(),
            mac_window: true,
            show_language: true,
            line_wrap: false,
            line_numbers: false,
        }
    }
}

pub fn normalize_style(style: &str) -> &'static str {
    match style.trim() {
        "BlackMac.css" => "BlackMac.css",
        "coy.css" => "coy.css",
        "dark.css" => "dark.css",
        "default.css" | "github" | "one_light" => "default.css",
        "GrayMac.css" => "GrayMac.css",
        "solarized-light.css" => "solarized-light.css",
        "tomorrow-night.css" | "dracula" => "tomorrow-night.css",
        "twilight.css" | "solarized_dark" => "twilight.css",
        "WhiteMac.css" => "WhiteMac.css",
        _ => "GrayMac.css",
    }
}

fn default_style() -> String {
    "GrayMac.css".to_owned()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_style_keeps_allowed_css_files() {
        assert_eq!(normalize_style("BlackMac.css"), "BlackMac.css");
        assert_eq!(normalize_style("WhiteMac.css"), "WhiteMac.css");
        assert_eq!(
            normalize_style(" solarized-light.css "),
            "solarized-light.css"
        );
    }

    #[test]
    fn normalize_style_maps_legacy_frontend_names() {
        assert_eq!(normalize_style("github"), "default.css");
        assert_eq!(normalize_style("one_light"), "default.css");
        assert_eq!(normalize_style("dracula"), "tomorrow-night.css");
        assert_eq!(normalize_style("solarized_dark"), "twilight.css");
    }

    #[test]
    fn normalize_style_falls_back_to_gray_mac() {
        assert_eq!(normalize_style(""), "GrayMac.css");
        assert_eq!(normalize_style("unknown.css"), "GrayMac.css");
    }
}
