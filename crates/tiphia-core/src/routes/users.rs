use crate::{
    app::AppState,
    error::AppResult,
    pagination::Page,
    routes::auth::CurrentUser,
    services::{
        auth::PublicUser,
        users::{ChangePasswordInput, CreateUserInput, ListUserQuery, UpdateUserInput},
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, put},
};

pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(show).put(update))
        .route("/{id}/password", put(change_password))
}

#[utoipa::path(get, path = "/api/v1/users", tag = "users", params(ListUserQuery), security(("bearerAuth" = [])), responses((status = 200, description = "Users")))]
pub async fn list(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(query): Query<ListUserQuery>,
) -> AppResult<Json<Page<PublicUser>>> {
    current_user.0.require_admin()?;
    Ok(Json(crate::services::users::list(&state, query).await?))
}

#[utoipa::path(post, path = "/api/v1/users", tag = "users", security(("bearerAuth" = [])), request_body = CreateUserInput, responses((status = 201, description = "Created user", body = PublicUser)))]
pub async fn create(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(input): Json<CreateUserInput>,
) -> AppResult<(StatusCode, Json<PublicUser>)> {
    current_user.0.require_admin()?;
    Ok((
        StatusCode::CREATED,
        Json(crate::services::users::create(&state, &current_user.0, input).await?),
    ))
}

#[utoipa::path(get, path = "/api/v1/users/{id}", tag = "users", security(("bearerAuth" = [])), responses((status = 200, description = "User", body = PublicUser)))]
pub async fn show(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<PublicUser>> {
    current_user.0.require_admin()?;
    Ok(Json(crate::services::users::show(&state, id).await?))
}

#[utoipa::path(put, path = "/api/v1/users/{id}", tag = "users", security(("bearerAuth" = [])), request_body = UpdateUserInput, responses((status = 200, description = "User", body = PublicUser)))]
pub async fn update(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<UpdateUserInput>,
) -> AppResult<Json<PublicUser>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::users::update(&state, &current_user.0, id, input).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/users/{id}/password", tag = "users", security(("bearerAuth" = [])), request_body = ChangePasswordInput, responses((status = 200, description = "User", body = PublicUser)))]
pub async fn change_password(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<ChangePasswordInput>,
) -> AppResult<Json<PublicUser>> {
    current_user.0.require_admin()?;
    Ok(Json(
        crate::services::users::change_password(&state, &current_user.0, id, input).await?,
    ))
}
