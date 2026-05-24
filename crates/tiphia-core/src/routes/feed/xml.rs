use crate::services::{posts::PostResponse, settings::SiteSettings};

pub fn absolute_permalink(settings: &SiteSettings, post: &PostResponse) -> String {
    let permalink = if post.permalink.starts_with('/') {
        post.permalink.clone()
    } else {
        format!("/{}", post.permalink)
    };

    settings
        .base_url
        .as_deref()
        .map(|base_url| format!("{}{}", base_url.trim_end_matches('/'), permalink))
        .unwrap_or(permalink)
}

pub fn site_url(settings: &SiteSettings) -> String {
    settings
        .base_url
        .clone()
        .unwrap_or_else(|| "/".to_owned())
        .trim_end_matches('/')
        .to_owned()
}

pub fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_xml_covers_quotes_and_apostrophes() {
        assert_eq!(escape_xml("&<>\"'"), "&amp;&lt;&gt;&quot;&apos;");
    }
}
