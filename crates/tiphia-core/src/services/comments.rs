use crate::{
    app::AppState,
    entities::{
        comments::{self, CommentStatus},
        posts,
    },
    error::{AppError, AppResult},
    pagination::{Page, PaginationQuery},
    plugins::{Hook, HookContext},
    services::settings,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

#[path = "comments/meta.rs"]
mod meta;
#[path = "comments/validation.rs"]
mod rules;
#[path = "comments/tree.rs"]
mod tree;
pub use meta::CommentRequestMeta;
pub use tree::CommentNode;

#[doc(hidden)]
pub fn build_comment_tree_for_bench(comments: Vec<comments::Model>) -> Vec<CommentNode> {
    tree::build(comments)
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ListCommentQuery {
    pub post_id: Option<i32>,
    pub status: Option<CommentStatus>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateCommentInput {
    pub post_id: i32,
    pub parent_id: Option<i32>,
    pub author_name: String,
    pub author_email: String,
    pub author_url: Option<String>,
    pub content: String,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub extensions: crate::services::auth::ExtensionMap,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub captcha: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ModerateCommentInput {
    pub status: CommentStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct RecentCommentQuery {
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct RecentCommentResponse {
    pub id: i32,
    pub post_id: i32,
    pub parent_id: Option<i32>,
    pub author_name: String,
    pub author_url: Option<String>,
    pub content: String,
    pub status: CommentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub post_slug: String,
    pub post_title: String,
}

pub async fn list(state: &AppState, query: ListCommentQuery) -> AppResult<Page<comments::Model>> {
    let mut select = comments::Entity::find().order_by_desc(comments::Column::CreatedAt);

    if let Some(post_id) = query.post_id {
        select = select.filter(comments::Column::PostId.eq(post_id));
    }

    if let Some(status) = query.status {
        select = select.filter(comments::Column::Status.eq(status));
    }

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = select.paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let items = paginator.fetch_page(page - 1).await?;

    Ok(Page::new(items, page, per_page, total, total_pages))
}

pub async fn recent(state: &AppState, limit: u64) -> AppResult<Vec<RecentCommentResponse>> {
    let limit = limit.clamp(1, 20);
    let rows = comments::Entity::find()
        .filter(comments::Column::Status.eq(CommentStatus::Approved))
        .order_by_desc(comments::Column::CreatedAt)
        .limit(limit)
        .find_also_related(posts::Entity)
        .all(&state.db)
        .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(comment, post)| {
            let post = post?;
            Some(RecentCommentResponse {
                id: comment.id,
                post_id: comment.post_id,
                parent_id: comment.parent_id,
                author_name: comment.author_name,
                author_url: comment.author_url,
                content: comment.content,
                status: comment.status,
                created_at: comment.created_at,
                updated_at: comment.updated_at,
                post_slug: post.slug,
                post_title: post.title,
            })
        })
        .collect())
}

pub async fn tree_for_post(
    state: &AppState,
    post_id: i32,
    status: Option<CommentStatus>,
) -> AppResult<Vec<CommentNode>> {
    let comments = comments::Entity::find()
        .filter(comments::Column::PostId.eq(post_id))
        .filter(comments::Column::Status.eq(status.unwrap_or(CommentStatus::Approved)))
        .order_by_asc(comments::Column::CreatedAt)
        .all(&state.db)
        .await?;

    Ok(tree::build(comments))
}

pub async fn create(state: &AppState, input: CreateCommentInput) -> AppResult<comments::Model> {
    create_with_meta(state, input, CommentRequestMeta::default()).await
}

pub async fn create_with_meta(
    state: &AppState,
    mut input: CreateCommentInput,
    meta: CommentRequestMeta,
) -> AppResult<comments::Model> {
    let site_settings = settings::get(state).await?;
    if !site_settings.comments_enabled {
        return Err(AppError::Forbidden);
    }

    rules::validate_create_input(&input)?;
    rules::ensure_post_exists(state, input.post_id).await?;
    rules::ensure_parent_belongs_to_post(state, input.post_id, input.parent_id).await?;

    let mut context = HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(Hook::BeforeCommentCreate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<CreateCommentInput>()? {
        input = next_input;
    }
    rules::validate_create_input(&input)?;
    rules::ensure_post_exists(state, input.post_id).await?;
    rules::ensure_parent_belongs_to_post(state, input.post_id, input.parent_id).await?;

    let now = Utc::now();
    let model = comments::ActiveModel {
        post_id: Set(input.post_id),
        parent_id: Set(input.parent_id),
        author_name: Set(input.author_name),
        author_email: Set(input.author_email),
        author_url: Set(input.author_url),
        ip_hash: Set(meta
            .client_ip
            .as_deref()
            .map(|ip| meta::hash_client_ip(&state.config.auth.jwt_secret, ip))),
        user_agent: Set(meta.user_agent.and_then(meta::normalize_user_agent)),
        content: Set(input.content),
        status: Set(if site_settings.comment_moderation {
            CommentStatus::Pending
        } else {
            CommentStatus::Approved
        }),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::AfterCommentCreate, &mut context)
        .await?;

    Ok(model)
}

pub async fn moderate(
    state: &AppState,
    id: i32,
    mut input: ModerateCommentInput,
) -> AppResult<comments::Model> {
    let existing = comments::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("comment"))?;

    let mut context = HookContext::with_subject(&input)?;
    context.insert_meta("comment", &existing)?;
    state
        .plugins
        .dispatch(Hook::BeforeCommentModerate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<ModerateCommentInput>()? {
        input = next_input;
    }

    let mut model: comments::ActiveModel = existing.into();
    model.status = Set(input.status);
    model.updated_at = Set(Utc::now());
    let updated = model.update(&state.db).await?;

    let mut context = HookContext::with_subject(&updated)?;
    state
        .plugins
        .dispatch(Hook::AfterCommentModerate, &mut context)
        .await?;

    Ok(updated)
}
