use crate::{
    app::AppState, entities::terms::Model as TermModel, error::AppResult,
    routes::auth::CurrentUser, services::terms::SyncPostTermsInput,
};
use axum::{
    Json,
    extract::{Path, State},
};

#[utoipa::path(get, path = "/api/v1/posts/{id}/terms", tag = "content", responses((status = 200, description = "Terms")))]
pub async fn post_terms(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<TermModel>>> {
    Ok(Json(
        crate::services::terms::terms_for_post(&state, id).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/posts/{id}/terms", tag = "content", security(("bearerAuth" = [])), request_body = SyncPostTermsInput, responses((status = 200, description = "Terms")))]
pub async fn sync_post_terms(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<SyncPostTermsInput>,
) -> AppResult<Json<Vec<TermModel>>> {
    let post = crate::services::posts::show(&state, id).await?;
    current_user
        .0
        .require_content_owner_or_editor(post.author_id)?;
    Ok(Json(
        crate::services::terms::sync_post_terms(&state, id, input).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/posts/{id}/comments/tree", tag = "comments", responses((status = 200, description = "Comment tree")))]
pub async fn comment_tree(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<crate::services::comments::CommentNode>>> {
    Ok(Json(
        crate::services::comments::tree_for_post(&state, id, None).await?,
    ))
}
