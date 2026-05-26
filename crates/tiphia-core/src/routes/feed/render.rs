use crate::services::{posts::PostResponse, settings::SiteSettings};
use chrono::{SecondsFormat, Utc};

use super::xml::{absolute_permalink, escape_xml, site_url};

pub fn render_rss(settings: &SiteSettings, posts: &[PostResponse]) -> String {
    let site_url = site_url(settings);
    let updated = posts
        .first()
        .map(|post| post.post.updated_at.to_rfc2822())
        .unwrap_or_else(|| Utc::now().to_rfc2822());
    let items = posts
        .iter()
        .map(|post| {
            let link = absolute_permalink(settings, post);
            format!(
                "<item><title>{}</title><link>{}</link><guid>{}</guid><pubDate>{}</pubDate><description>{}</description></item>",
                escape_xml(&post.post.title),
                escape_xml(&link),
                escape_xml(&link),
                post.post.published_at.unwrap_or(post.post.created_at).to_rfc2822(),
                escape_xml(post.post.excerpt.as_deref().unwrap_or_default())
            )
        })
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><rss version="2.0"><channel><title>{}</title><link>{}</link><description>{}</description><lastBuildDate>{}</lastBuildDate>{}</channel></rss>"#,
        escape_xml(&settings.title),
        escape_xml(&site_url),
        escape_xml(&settings.description),
        updated,
        items
    )
}

pub fn render_atom(settings: &SiteSettings, posts: &[PostResponse]) -> String {
    let site_url = site_url(settings);
    let updated = posts
        .first()
        .map(|post| {
            post.post
                .updated_at
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        })
        .unwrap_or_else(|| Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true));
    let entries = posts
        .iter()
        .map(|post| {
            let link = absolute_permalink(settings, post);
            format!(
                r#"<entry><title>{}</title><link href="{}"/><id>{}</id><updated>{}</updated><summary>{}</summary></entry>"#,
                escape_xml(&post.post.title),
                escape_xml(&link),
                escape_xml(&link),
                post.post.updated_at.to_rfc3339_opts(SecondsFormat::Secs, true),
                escape_xml(post.post.excerpt.as_deref().unwrap_or_default())
            )
        })
        .collect::<String>();

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><feed xmlns="http://www.w3.org/2005/Atom"><title>{}</title><link href="{}"/><id>{}</id><updated>{}</updated>{}</feed>"#,
        escape_xml(&settings.title),
        escape_xml(&site_url),
        escape_xml(&site_url),
        updated,
        entries
    )
}

pub fn render_sitemap(
    settings: &SiteSettings,
    posts: &[PostResponse],
    pages: &[PostResponse],
) -> String {
    let mut urls = String::new();
    if settings.base_url.is_some() {
        urls.push_str(&format!(
            "<url><loc>{}</loc></url>",
            escape_xml(&site_url(settings))
        ));
    }
    for item in posts.iter().chain(pages.iter()) {
        urls.push_str(&format!(
            "<url><loc>{}</loc><lastmod>{}</lastmod></url>",
            escape_xml(&absolute_permalink(settings, item)),
            item.post
                .updated_at
                .to_rfc3339_opts(SecondsFormat::Secs, true)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::posts::{self, PostStatus, PostType};
    use chrono::TimeZone;

    fn settings() -> SiteSettings {
        SiteSettings {
            title: "我的博客 & <Tiphia>".to_owned(),
            description: "中文描述 & symbols".to_owned(),
            base_url: Some("https://example.com/".to_owned()),
            ..SiteSettings::default()
        }
    }

    fn post_response() -> PostResponse {
        let created_at = Utc.with_ymd_and_hms(2026, 5, 23, 8, 0, 0).unwrap();
        PostResponse {
            post: posts::Model {
                id: 1,
                slug: "hello".to_owned(),
                title: "标题 & <测试>".to_owned(),
                markdown: "".to_owned(),
                html: "".to_owned(),
                excerpt: Some("摘要 & <xml>".to_owned()),
                status: PostStatus::Published,
                post_type: PostType::Post,
                author_id: 1,
                published_at: Some(created_at),
                created_at,
                updated_at: created_at,
            },
            permalink: "/legacy/hello".to_owned(),
            view_count: 0,
            comment_count: 0,
        }
    }

    #[test]
    fn rss_escapes_xml_and_keeps_utf8_declaration() {
        let xml = render_rss(&settings(), &[post_response()]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>"));
        assert!(xml.contains("我的博客 &amp; &lt;Tiphia&gt;"));
        assert!(xml.contains("标题 &amp; &lt;测试&gt;"));
        assert!(xml.contains("摘要 &amp; &lt;xml&gt;"));
        assert!(xml.contains("https://example.com/posts/hello"));
    }

    #[test]
    fn atom_escapes_xml_and_uses_absolute_links() {
        let xml = render_atom(&settings(), &[post_response()]);
        assert!(xml.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(xml.contains("标题 &amp; &lt;测试&gt;"));
        assert!(xml.contains("<link href=\"https://example.com/posts/hello\"/>"));
    }

    #[test]
    fn sitemap_includes_site_and_public_items() {
        let item = post_response();
        let xml = render_sitemap(
            &settings(),
            std::slice::from_ref(&item),
            std::slice::from_ref(&item),
        );
        assert!(xml.contains("<loc>https://example.com</loc>"));
        assert_eq!(xml.matches("https://example.com/posts/hello").count(), 2);
    }
}
