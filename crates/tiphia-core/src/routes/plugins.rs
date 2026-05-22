use crate::{
    app::AppState,
    error::AppResult,
    plugins::AdminMenuItem,
    routes::auth::CurrentUser,
    services::plugins::{
        PluginConfigResponse, PluginInfo, PluginStateResponse, UpdatePluginConfigInput,
        UpdatePluginStateInput,
    },
};
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::get,
};

pub fn plugin_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list))
        .route("/admin-menu", get(admin_menu))
        .route("/{name}/config", get(get_config).put(update_config))
        .route("/{name}/state", get(get_state).put(update_state))
}

#[utoipa::path(get, path = "/api/v1/plugins", tag = "plugins", responses((status = 200, description = "Plugins")))]
pub async fn list(State(state): State<AppState>) -> AppResult<Json<Vec<PluginInfo>>> {
    Ok(Json(crate::services::plugins::list(&state).await?))
}

#[utoipa::path(get, path = "/api/v1/plugins/admin-menu", tag = "plugins", security(("bearerAuth" = [])), responses((status = 200, description = "Admin menu")))]
pub async fn admin_menu(
    State(state): State<AppState>,
    current_user: CurrentUser,
) -> AppResult<Json<Vec<AdminMenuItem>>> {
    current_user.0.require_editor()?;
    Ok(Json(crate::services::plugins::admin_menu(&state).await?))
}

#[utoipa::path(get, path = "/api/v1/plugins/{name}/config", tag = "plugins", security(("bearerAuth" = [])), responses((status = 200, description = "Plugin config")))]
pub async fn get_config(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(name): Path<String>,
) -> AppResult<Json<PluginConfigResponse>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::plugins::get_config(&state, &name).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/plugins/{name}/config", tag = "plugins", security(("bearerAuth" = [])), request_body = UpdatePluginConfigInput, responses((status = 200, description = "Plugin config")))]
pub async fn update_config(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(name): Path<String>,
    Json(input): Json<UpdatePluginConfigInput>,
) -> AppResult<Json<PluginConfigResponse>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::plugins::update_config(&state, &name, input).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/plugins/{name}/state", tag = "plugins", security(("bearerAuth" = [])), responses((status = 200, description = "Plugin state")))]
pub async fn get_state(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(name): Path<String>,
) -> AppResult<Json<PluginStateResponse>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::plugins::get_state(&state, &name).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/plugins/{name}/state", tag = "plugins", security(("bearerAuth" = [])), request_body = UpdatePluginStateInput, responses((status = 200, description = "Plugin state")))]
pub async fn update_state(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(name): Path<String>,
    Json(input): Json<UpdatePluginStateInput>,
) -> AppResult<Json<PluginStateResponse>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::plugins::update_state(&state, &name, input).await?,
    ))
}
