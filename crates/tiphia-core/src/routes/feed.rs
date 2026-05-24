use crate::{app::AppState, entities::posts::PostType};
use axum::{extract::State, response::Response};

#[path = "feed/query.rs"]
mod query;
#[path = "feed/render.rs"]
mod render;
#[path = "feed/response.rs"]
mod response;
#[path = "feed/xml.rs"]
mod xml;

pub async fn rss(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = query::public_items(&state, Some(PostType::Post), 50).await?;
    Ok(response::xml_response(
        render::render_rss(&settings, &posts),
        "application/rss+xml; charset=utf-8",
    ))
}

pub async fn atom(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = query::public_items(&state, Some(PostType::Post), 50).await?;
    Ok(response::xml_response(
        render::render_atom(&settings, &posts),
        "application/atom+xml; charset=utf-8",
    ))
}

pub async fn sitemap(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let posts = query::public_items(&state, Some(PostType::Post), 100).await?;
    let pages = query::public_items(&state, Some(PostType::Page), 100).await?;
    Ok(response::xml_response(
        render::render_sitemap(&settings, &posts, &pages),
        "application/xml; charset=utf-8",
    ))
}

pub async fn robots(State(state): State<AppState>) -> crate::AppResult<Response> {
    let settings = crate::services::settings::get(&state).await?;
    let sitemap = settings
        .base_url
        .as_deref()
        .map(|base_url| format!("Sitemap: {}/sitemap.xml\n", base_url.trim_end_matches('/')))
        .unwrap_or_default();
    Ok(response::text_response(format!(
        "User-agent: *\nAllow: /\n{sitemap}"
    )))
}
