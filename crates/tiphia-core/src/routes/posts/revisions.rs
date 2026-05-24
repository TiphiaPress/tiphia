use crate::{
    app::AppState,
    entities::{post_revisions, posts::Model},
    error::AppResult,
    routes::auth::CurrentUser,
};
use axum::{
    Json,
    extract::{Path, State},
};

#[utoipa::path(get, path = "/api/v1/posts/{id}/revisions", tag = "content", security(("bearerAuth" = [])), responses((status = 200, description = "Revisions")))]
pub async fn revisions(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<post_revisions::Model>>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(crate::services::posts::revisions(&state, id).await?))
}

#[utoipa::path(put, path = "/api/v1/posts/{id}/revisions/{revision_id}/restore", tag = "content", security(("bearerAuth" = [])), responses((status = 200, description = "Restored content")))]
pub async fn restore_revision(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path((id, revision_id)): Path<(i32, i32)>,
) -> AppResult<Json<Model>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(
        crate::services::posts::restore_revision(&state, id, revision_id).await?,
    ))
}
