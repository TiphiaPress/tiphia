use crate::{app::AppState, error::AppResult};
use axum::{Json, Router, extract::State, routing::get};

pub fn theme_routes() -> Router<AppState> {
    Router::new().route("/", get(list))
}

#[utoipa::path(get, path = "/api/v1/themes", tag = "themes", responses((status = 200, description = "Themes")))]
pub async fn list(
    State(state): State<AppState>,
) -> AppResult<Json<Vec<crate::services::themes::ThemeInfo>>> {
    let settings = crate::services::settings::get(&state).await?;
    Ok(Json(crate::services::themes::list(&settings.theme.active)))
}
