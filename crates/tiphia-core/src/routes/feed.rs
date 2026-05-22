use crate::{
    app::AppState,
    entities::posts::PostType,
    pagination::PaginationQuery,
    services::{
        posts::{ListPostQuery, PostResponse},
        settings::SiteSettings,
    },
};
use axum::{
    extract::State,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
};
use chrono::{SecondsFormat, Utc};

pub async fn rss(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = public_items(&state, Some(PostType::Post), 50).await?;
    Ok(xml_response(render_rss(&settings, &posts), "application/rss+xml"))
}

pub async fn atom(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = public_items(&state, Some(PostType::Post), 50).await?;
    Ok(xml_response(
        render_atom(&settings, &posts),
        "application/atom+xml",
    ))
}

pub async fn sitemap(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = public_items(&state, Some(PostType::Post), 100).await?;
    let pages = public_items(&state, Some(PostType::Page), 100).await?;
    Ok(xml_response(
        render_sitemap(&settings, &posts, &pages),
        "application/xml",
    ))
}

pub async fn robots(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let sitemap = settings
        .base_url
        .as_deref()
        .map(|base_url| format!("Sitemap: {}/sitemap.xml\n", base_url.trim_end_matches('/')))
        .unwrap_or_default();
    let body = format!("User-agent: *\nAllow: /\n{sitemap}");
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body).into_response())
}

async fn public_items(
    state: &AppState,
    post_type: Option<PostType>,
    per_page: u64,
) -> crate::AppResult<Vec<PostResponse>> {
    let page = crate::services::posts::list(
        state,
        ListPostQuery {
            q: None,
            status: None,
            post_type,
            term_id: None,
            pagination: PaginationQuery {
                page: Some(1),
                per_page: Some(per_page),
            },
        },
    )
    .await?;

    Ok(page.data)
}

fn render_rss(settings: &SiteSettings, posts: &[PostResponse]) -> String {
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

fn render_atom(settings: &SiteSettings, posts: &[PostResponse]) -> String {
    let site_url = site_url(settings);
    let updated = posts
        .first()
        .map(|post| post.post.updated_at.to_rfc3339_opts(SecondsFormat::Secs, true))
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

fn render_sitemap(settings: &SiteSettings, posts: &[PostResponse], pages: &[PostResponse]) -> String {
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
            item.post.updated_at.to_rfc3339_opts(SecondsFormat::Secs, true)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="utf-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{urls}</urlset>"#
    )
}

fn absolute_permalink(settings: &SiteSettings, post: &PostResponse) -> String {
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

fn site_url(settings: &SiteSettings) -> String {
    settings
        .base_url
        .clone()
        .unwrap_or_else(|| "/".to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn xml_response(body: String, content_type: &'static str) -> Response {
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(content_type),
    );
    response
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
