use crate::{
    app::AppState,
    entities::{
        posts::{Model, PostType},
        terms::Model as TermModel,
    },
    error::AppResult,
    pagination::Page,
    routes::auth::CurrentUser,
    services::{
        posts::{
            BulkPostActionInput, BulkPostActionResponse, ChangePostStatusInput, CreatePostInput,
            ListPostQuery, PopularPostQuery, PostResponse, UpdatePostInput,
        },
        terms::SyncPostTermsInput,
    },
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, put},
};

pub fn post_routes() -> Router<AppState> {
    resource_routes(PostType::Post)
}

pub fn page_routes() -> Router<AppState> {
    resource_routes(PostType::Page)
}

fn resource_routes(post_type: PostType) -> Router<AppState> {
    let list_post_type = post_type.clone();
    let admin_list_post_type = post_type.clone();
    let slug_post_type = post_type;

    Router::new()
        .route(
            "/",
            get(move |state, query| list(state, query, list_post_type.clone())).post(create),
        )
        .route(
            "/admin",
            get(move |state, current_user, query| {
                admin_list(state, current_user, query, admin_list_post_type.clone())
            }),
        )
        .route("/popular", get(popular))
        .route("/bulk", put(bulk_action))
        .route("/admin/{id}", get(admin_show))
        .route(
            "/slug/{slug}",
            get(move |state, path| show_by_slug(state, path, slug_post_type.clone())),
        )
        .route("/{id}", get(show).put(update).delete(delete_post))
        .route("/{id}/comments/tree", get(comment_tree))
        .route("/{id}/revisions", get(revisions))
        .route(
            "/{id}/revisions/{revision_id}/restore",
            put(restore_revision),
        )
        .route("/{id}/status", put(change_status))
        .route("/{id}/terms", get(post_terms).put(sync_post_terms))
}

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

#[utoipa::path(get, path = "/api/v1/posts/{id}/terms", tag = "content", responses((status = 200, description = "Terms")))]
pub async fn post_terms(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<TermModel>>> {
    Ok(Json(
        crate::services::terms::terms_for_post(&state, id).await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/posts/{id}/revisions", tag = "content", security(("bearerAuth" = [])), responses((status = 200, description = "Revisions")))]
pub async fn revisions(
    State(state): State<AppState>,
    current_user: CurrentUser,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<crate::entities::post_revisions::Model>>> {
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

#[utoipa::path(get, path = "/api/v1/posts/{id}/comments/tree", tag = "comments", responses((status = 200, description = "Comment tree")))]
pub async fn comment_tree(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> AppResult<Json<Vec<crate::services::comments::CommentNode>>> {
    Ok(Json(
        crate::services::comments::tree_for_post(&state, id, None).await?,
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
