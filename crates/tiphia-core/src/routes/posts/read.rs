use crate::{
    app::AppState,
    entities::posts::PostType,
    error::AppResult,
    pagination::Page,
    routes::auth::CurrentUser,
    services::posts::{ListPostQuery, PopularPostQuery, PostResponse},
};
use axum::{
    Json,
    extract::{Path, Query, State},
};

#[utoipa::path(
    get,
    path = "/api/v1/posts",
    tag = "content",
    params(ListPostQuery),
    responses((status = 200, description = "Content list"))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(mut query): Query<ListPostQuery>,
    post_type: PostType,
) -> AppResult<Json<Page<PostResponse>>> {
    query.post_type = Some(post_type);
    Ok(Json(crate::services::posts::list(&state, query).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/admin",
    tag = "content",
    security(("bearerAuth" = [])),
    params(ListPostQuery),
    responses((status = 200, description = "Admin content list"))
)]
pub async fn admin_list(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Query(mut query): Query<ListPostQuery>,
    post_type: PostType,
) -> AppResult<Json<Page<PostResponse>>> {
    query.post_type = Some(post_type);
    Ok(Json(
        crate::services::posts::list_admin(&state, query, &current_user.0).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/popular",
    tag = "content",
    params(PopularPostQuery),
    responses((status = 200, description = "Popular posts", body = Vec<PostResponse>))
)]
pub async fn popular(
    State(state): State<AppState>,
    Query(query): Query<PopularPostQuery>,
) -> AppResult<Json<Vec<PostResponse>>> {
    Ok(Json(
        crate::services::posts::popular(&state, query.limit.unwrap_or(5)).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/{id}",
    tag = "content",
    params(("id" = i32, Path, description = "Content id")),
    responses((status = 200, description = "Content", body = PostResponse))
)]
pub async fn show(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<PostResponse>> {
    Ok(Json(
        crate::services::posts::show_response(&state, id).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/admin/{id}",
    tag = "content",
    security(("bearerAuth" = [])),
    params(("id" = i32, Path, description = "Content id")),
    responses((status = 200, description = "Admin content", body = PostResponse))
)]
pub async fn admin_show(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<PostResponse>> {
    Ok(Json(
        crate::services::posts::show_admin_response(&state, id, &current_user.0).await?,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/posts/slug/{slug}",
    tag = "content",
    params(("slug" = String, Path, description = "Slug")),
    responses((status = 200, description = "Content", body = PostResponse))
)]
pub async fn show_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    post_type: PostType,
) -> AppResult<Json<PostResponse>> {
    Ok(Json(
        crate::services::posts::show_by_slug(&state, post_type, &slug).await?,
    ))
}
