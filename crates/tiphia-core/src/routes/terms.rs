use crate::{
    app::AppState,
    entities::terms::Model,
    error::AppResult,
    pagination::Page,
    routes::auth::CurrentUser,
    services::terms::{CreateTermInput, ListTermQuery, TermResponse, UpdateTermInput},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};

pub fn term_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(show).put(update).delete(delete_term))
}

#[utoipa::path(get, path = "/api/v1/terms", tag = "terms", params(ListTermQuery), responses((status = 200, description = "Terms")))]
pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListTermQuery>,
) -> AppResult<Json<Page<TermResponse>>> {
    Ok(Json(crate::services::terms::list(&state, query).await?))
}

#[utoipa::path(get, path = "/api/v1/terms/{id}", tag = "terms", responses((status = 200, description = "Term")))]
pub async fn show(State(state): State<AppState>, Path(id): Path<i32>) -> AppResult<Json<Model>> {
    Ok(Json(crate::services::terms::show(&state, id).await?))
}

#[utoipa::path(post, path = "/api/v1/terms", tag = "terms", security(("bearerAuth" = [])), request_body = CreateTermInput, responses((status = 200, description = "Created term")))]
pub async fn create(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(input): Json<CreateTermInput>,
) -> AppResult<Json<Model>> {
    current_user.0.require_editor()?;
    Ok(Json(crate::services::terms::create(&state, input).await?))
}

#[utoipa::path(put, path = "/api/v1/terms/{id}", tag = "terms", security(("bearerAuth" = [])), request_body = UpdateTermInput, responses((status = 200, description = "Updated term")))]
pub async fn update(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<UpdateTermInput>,
) -> AppResult<Json<Model>> {
    current_user.0.require_editor()?;
    Ok(Json(
        crate::services::terms::update(&state, id, input).await?,
    ))
}

#[utoipa::path(delete, path = "/api/v1/terms/{id}", tag = "terms", security(("bearerAuth" = [])), responses((status = 200, description = "Deleted term")))]
pub async fn delete_term(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Model>> {
    current_user.0.require_editor()?;
    Ok(Json(crate::services::terms::delete(&state, id).await?))
}
