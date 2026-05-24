use crate::{
    app::AppState,
    entities::posts::Model,
    error::AppResult,
    routes::auth::CurrentUser,
    services::posts::{
        BulkPostActionInput, BulkPostActionResponse, ChangePostStatusInput, CreatePostInput,
        UpdatePostInput,
    },
};
use axum::{
    Json,
    extract::{Path, State},
};

#[utoipa::path(
    post,
    path = "/api/v1/posts",
    tag = "content",
    security(("bearerAuth" = [])),
    request_body = CreatePostInput,
    responses((status = 200, description = "Created content"))
)]
pub async fn create(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(input): Json<CreatePostInput>,
) -> AppResult<Json<Model>> {
    Ok(Json(
        crate::services::posts::create(
            &state,
            current_user.0.id,
            current_user.0.can_edit_all_content(),
            input,
        )
        .await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}",
    tag = "content",
    security(("bearerAuth" = [])),
    request_body = UpdatePostInput,
    responses((status = 200, description = "Updated content"))
)]
pub async fn update(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<UpdatePostInput>,
) -> AppResult<Json<Model>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(
        crate::services::posts::update(&state, id, current_user.0.can_edit_all_content(), input)
            .await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/{id}/status",
    tag = "content",
    security(("bearerAuth" = [])),
    request_body = ChangePostStatusInput,
    responses((status = 200, description = "Updated status"))
)]
pub async fn change_status(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<ChangePostStatusInput>,
) -> AppResult<Json<Model>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(
        crate::services::posts::change_status(
            &state,
            id,
            current_user.0.can_edit_all_content(),
            input,
        )
        .await?,
    ))
}

#[utoipa::path(
    put,
    path = "/api/v1/posts/bulk",
    tag = "content",
    security(("bearerAuth" = [])),
    request_body = BulkPostActionInput,
    responses((status = 200, description = "Bulk content action", body = BulkPostActionResponse))
)]
pub async fn bulk_action(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Json(input): Json<BulkPostActionInput>,
) -> AppResult<Json<BulkPostActionResponse>> {
    Ok(Json(
        crate::services::posts::bulk_action(&state, &current_user.0, input).await?,
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/posts/{id}",
    tag = "content",
    security(("bearerAuth" = [])),
    responses((status = 200, description = "Deleted content"))
)]
pub async fn delete_post(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Model>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(crate::services::posts::delete(&state, id).await?))
}
