use crate::{
    app::AppState,
    entities::comments::Model,
    error::AppResult,
    pagination::Page,
    routes::auth::CurrentUser,
    services::comments::{
        CommentNode, CommentRequestMeta, CreateCommentInput, ListCommentQuery,
        ModerateCommentInput, RecentCommentQuery,
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    routing::{get, put},
};

pub fn comment_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/recent", get(recent))
        .route("/post/{post_id}/tree", get(tree_for_post))
        .route("/{id}/moderation", put(moderate))
}

#[utoipa::path(get, path = "/api/v1/comments", tag = "comments", security(("bearerAuth" = [])), params(ListCommentQuery), responses((status = 200, description = "Comments")))]
pub async fn list(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(query): Query<ListCommentQuery>,
) -> AppResult<Json<Page<Model>>> {
    current_user.0.require_editor()?;
    Ok(Json(crate::services::comments::list(&state, query).await?))
}

#[utoipa::path(get, path = "/api/v1/comments/recent", tag = "comments", params(RecentCommentQuery), responses((status = 200, description = "Recent approved comments")))]
pub async fn recent(
    State(state): State<AppState>,
    Query(query): Query<RecentCommentQuery>,
) -> AppResult<Json<Vec<crate::services::comments::RecentCommentResponse>>> {
    Ok(Json(
        crate::services::comments::recent(&state, query.limit.unwrap_or(5)).await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/comments", tag = "comments", request_body = CreateCommentInput, responses((status = 200, description = "Created comment"), (status = 429, description = "Rate limited")))]
pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<CreateCommentInput>,
) -> AppResult<Json<Model>> {
    crate::rate_limit::check_comment(&state, &headers).await?;
    Ok(Json(
        crate::services::comments::create_with_meta(&state, input, comment_meta(&headers)).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/comments/post/{post_id}/tree", tag = "comments", responses((status = 200, description = "Comment tree")))]
pub async fn tree_for_post(
    State(state): State<AppState>,
    Path(post_id): Path<i32>,
) -> AppResult<Json<Vec<CommentNode>>> {
    Ok(Json(
        crate::services::comments::tree_for_post(&state, post_id, None).await?,
    ))
}

#[utoipa::path(put, path = "/api/v1/comments/{id}/moderation", tag = "comments", security(("bearerAuth" = [])), request_body = ModerateCommentInput, responses((status = 200, description = "Moderated comment")))]
pub async fn moderate(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
    Json(input): Json<ModerateCommentInput>,
) -> AppResult<Json<Model>> {
    current_user.0.require_editor()?;
    Ok(Json(
        crate::services::comments::moderate(&state, id, input).await?,
    ))
}

fn comment_meta(headers: &HeaderMap) -> CommentRequestMeta {
    CommentRequestMeta {
        client_ip: headers
            .get("x-forwarded-for")
            .or_else(|| headers.get("x-real-ip"))
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(',').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned),
    }
}
