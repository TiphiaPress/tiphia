use crate::{
    app::AppState, error::AppResult, routes::auth::CurrentUser, services::settings::SiteSettings,
};
use axum::{Json, Router, extract::State, routing::get};

pub fn settings_routes() -> Router<AppState> {
    Router::new().route("/", get(get_settings).put(update_settings))
}

#[utoipa::path(get, path = "/api/v1/settings", tag = "settings", responses((status = 200, description = "Settings", body = SiteSettings)))]
pub async fn get_settings(State(state): State<AppState>) -> AppResult<Json<SiteSettings>> {
    Ok(Json(crate::services::settings::get(&state).await?))
}

#[utoipa::path(put, path = "/api/v1/settings", tag = "settings", security(("bearerAuth" = [])), request_body = SiteSettings, responses((status = 200, description = "Settings", body = SiteSettings)))]
pub async fn update_settings(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(settings): Json<SiteSettings>,
) -> AppResult<Json<SiteSettings>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::settings::update(&state, settings).await?,
    ))
}
