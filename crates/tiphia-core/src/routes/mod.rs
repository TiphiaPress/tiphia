use crate::app::AppState;
use axum::{Json, Router, routing::get};
use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

pub mod auth;
pub mod comments;
pub mod feed;
pub mod openapi;
pub mod plugins;
pub mod posts;
pub mod settings;
pub mod terms;
pub mod themes;
pub mod users;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/openapi.json", get(openapi::openapi))
        .route("/feed.xml", get(feed::rss))
        .route("/atom.xml", get(feed::atom))
        .route("/sitemap.xml", get(feed::sitemap))
        .route("/robots.txt", get(feed::robots))
        .nest("/api/v1/auth", auth::auth_routes())
        .nest("/api/v1/posts", posts::post_routes())
        .nest("/api/v1/pages", posts::page_routes())
        .nest("/api/v1/comments", comments::comment_routes())
        .nest("/api/v1/terms", terms::term_routes())
        .nest("/api/v1/users", users::user_routes())
        .nest("/api/v1/plugins", plugins::plugin_routes())
        .nest("/api/v1/themes", themes::theme_routes())
        .nest("/api/v1/settings", settings::settings_routes())
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub checked_at: DateTime<Utc>,
}

#[utoipa::path(get, path = "/health", tag = "system", responses((status = 200, description = "Health", body = HealthResponse)))]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        checked_at: Utc::now(),
    })
}
