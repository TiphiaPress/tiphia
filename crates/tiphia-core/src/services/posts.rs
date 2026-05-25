use crate::{
    app::AppState,
    entities::{
        post_revisions,
        posts::{self, PostStatus, PostType},
    },
    error::{AppError, AppResult, validation_on_unique},
    pagination::{Page, PaginationQuery},
    plugins::{Hook, HookContext},
    services::{
        auth::{ExtensionMap, PublicUser},
        validation,
    },
};
use chrono::{DateTime, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[path = "posts/content.rs"]
mod content;
#[path = "posts/maintenance.rs"]
mod maintenance;
#[path = "posts/hooks.rs"]
mod post_hooks;
#[path = "posts/query.rs"]
mod query_builder;
#[path = "posts/response.rs"]
mod response;
#[path = "posts/revisions.rs"]
mod revisions_service;
#[path = "posts/slug.rs"]
mod slug;
#[path = "posts/visibility.rs"]
mod visibility;
pub use response::PostResponse;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ListPostQuery {
    pub q: Option<String>,
    pub status: Option<PostStatus>,
    pub post_type: Option<PostType>,
    pub term_id: Option<i32>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreatePostInput {
    #[serde(default)]
    pub slug: String,
    pub title: String,
    pub markdown: String,
    pub html: Option<String>,
    pub excerpt: Option<String>,
    pub status: Option<PostStatus>,
    pub post_type: Option<PostType>,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub extensions: ExtensionMap,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdatePostInput {
    pub slug: Option<String>,
    pub title: Option<String>,
    pub markdown: Option<String>,
    pub html: Option<String>,
    pub excerpt: Option<String>,
    pub status: Option<PostStatus>,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub extensions: ExtensionMap,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangePostStatusInput {
    pub status: PostStatus,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub extensions: ExtensionMap,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkPostActionInput {
    pub ids: Vec<i32>,
    pub action: BulkPostAction,
    pub published_at: Option<DateTime<Utc>>,
    #[serde(default)]
    #[schema(value_type = Object)]
    pub extensions: ExtensionMap,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum BulkPostAction {
    Publish,
    Archive,
    Delete,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct PopularPostQuery {
    pub limit: Option<u64>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct BulkPostActionResponse {
    pub action: BulkPostAction,
    pub affected: usize,
    pub posts: Vec<posts::Model>,
}

pub async fn list(
    state: &AppState,
    query: ListPostQuery,
) -> AppResult<Page<response::PostResponse>> {
    list_with_visibility(state, query, None).await
}

pub async fn list_admin(
    state: &AppState,
    query: ListPostQuery,
    current_user: &PublicUser,
) -> AppResult<Page<response::PostResponse>> {
    list_with_visibility(state, query, Some(current_user)).await
}

async fn list_with_visibility(
    state: &AppState,
    query: ListPostQuery,
    current_user: Option<&PublicUser>,
) -> AppResult<Page<response::PostResponse>> {
    let hook = post_hooks::before_list(&query);
    let mut context = HookContext::with_subject(&query)?;
    state.plugins.dispatch(hook, &mut context).await?;

    let select = query_builder::build_list_select(state, &query, current_user).await?;

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = select.paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let settings = crate::services::settings::get(state).await?;
    let page_posts = paginator.fetch_page(page - 1).await?;
    let mut items = Vec::with_capacity(page_posts.len());
    for post in page_posts {
        items.push(response::for_post_with_settings(state, &settings, post).await?);
    }
    let after_hook = post_hooks::after_list(&query);
    let mut context = HookContext::with_subject(&items)?;
    state.plugins.dispatch(after_hook, &mut context).await?;

    Ok(Page::new(items, page, per_page, total, total_pages))
}

pub async fn show(state: &AppState, id: i32) -> AppResult<posts::Model> {
    posts::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("post"))
}

pub async fn show_response(state: &AppState, id: i32) -> AppResult<response::PostResponse> {
    let post = show(state, id).await?;
    visibility::ensure_publicly_visible(&post)?;
    response::increment_view_count(state, post.id).await?;
    response::for_post(state, post).await
}

pub async fn show_admin_response(
    state: &AppState,
    id: i32,
    current_user: &PublicUser,
) -> AppResult<response::PostResponse> {
    let post = show(state, id).await?;
    current_user.require_content_owner_or_editor(post.author_id)?;
    response::for_post(state, post).await
}

pub async fn show_by_slug(
    state: &AppState,
    post_type: PostType,
    slug: &str,
) -> AppResult<response::PostResponse> {
    let post = posts::Entity::find()
        .filter(posts::Column::Slug.eq(slug.to_owned()))
        .filter(posts::Column::PostType.eq(post_type))
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("post"))?;

    visibility::ensure_publicly_visible(&post)?;
    response::increment_view_count(state, post.id).await?;
    response::for_post(state, post).await
}

pub async fn popular(state: &AppState, limit: u64) -> AppResult<Vec<response::PostResponse>> {
    let limit = limit.clamp(1, 20);
    let posts = posts::Entity::find()
        .filter(visibility::public_visible_condition(Utc::now()))
        .filter(posts::Column::PostType.eq(PostType::Post))
        .all(&state.db)
        .await?;
    let settings = crate::services::settings::get(state).await?;
    let mut responses = Vec::with_capacity(posts.len());
    for post in posts {
        responses.push(response::for_post_with_settings(state, &settings, post).await?);
    }
    responses.sort_by(|left, right| {
        right
            .view_count
            .cmp(&left.view_count)
            .then_with(|| right.comment_count.cmp(&left.comment_count))
            .then_with(|| right.post.created_at.cmp(&left.post.created_at))
    });
    responses.truncate(limit as usize);
    Ok(responses)
}

pub async fn create(
    state: &AppState,
    author_id: i32,
    can_publish: bool,
    mut input: CreatePostInput,
) -> AppResult<posts::Model> {
    validation::required(&input.title, "title")?;

    let mut post_type = input.post_type.clone().unwrap_or(PostType::Post);
    let hook = post_hooks::before_create(&post_type);
    let mut context = HookContext::with_subject(&input)?;
    context.insert_meta("author_id", author_id)?;
    context.insert_meta("can_publish", can_publish)?;
    context.insert_meta("post_type", &post_type)?;
    state.plugins.dispatch(hook, &mut context).await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<CreatePostInput>()? {
        input = next_input;
        post_type = input.post_type.clone().unwrap_or(PostType::Post);
    }
    validation::required(&input.title, "title")?;
    let slug = slug::normalize_create_slug(state, &input.slug, &input.title, &post_type).await?;

    let now = Utc::now();
    let (status, published_at) = visibility::normalize_status(
        input.status.clone().unwrap_or(PostStatus::Draft),
        input.published_at,
        can_publish,
    )?;
    let rendered = content::render_create(state, &input).await?;
    let model = posts::ActiveModel {
        slug: Set(slug),
        title: Set(input.title),
        markdown: Set(rendered.markdown),
        html: Set(rendered.html),
        excerpt: Set(Some(rendered.excerpt)),
        status: Set(status.clone()),
        post_type: Set(post_type.clone()),
        author_id: Set(author_id),
        published_at: Set(published_at),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|err| validation_on_unique(err, "post slug already exists"))?;

    let after_hook = post_hooks::after_create(&post_type);
    let mut context = HookContext::with_subject(&model)?;
    state.plugins.dispatch(after_hook, &mut context).await?;

    Ok(model)
}

pub async fn update(
    state: &AppState,
    id: i32,
    can_publish: bool,
    mut input: UpdatePostInput,
) -> AppResult<posts::Model> {
    let mut context = HookContext::with_subject(&input)?;
    context.insert_meta("post_id", id)?;
    context.insert_meta("can_publish", can_publish)?;
    state
        .plugins
        .dispatch(Hook::BeforePostUpdate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<UpdatePostInput>()? {
        input = next_input;
    }

    let existing = show(state, id).await?;
    revisions_service::save(state, &existing).await?;
    let rendered = content::render_update(state, &existing, &input).await?;
    let mut model: posts::ActiveModel = existing.into();

    if let Some(slug) = input.slug {
        validation::required(&slug, "slug")?;
        validation::slug(&slug)?;
        model.slug = Set(slug);
    }
    if let Some(title) = input.title {
        validation::required(&title, "title")?;
        model.title = Set(title);
    }
    if let Some(rendered) = rendered {
        model.markdown = Set(rendered.markdown);
        model.html = Set(rendered.html);
        model.excerpt = Set(Some(rendered.excerpt));
    }
    if let Some(status) = input.status {
        let (status, published_at) =
            visibility::normalize_status(status, input.published_at, can_publish)?;
        model.status = Set(status);
        model.published_at = Set(published_at);
    } else if let Some(published_at) = input.published_at {
        model.published_at = Set(Some(published_at));
    }
    model.updated_at = Set(Utc::now());

    let updated = model
        .update(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "post slug already exists"))?;
    let mut context = HookContext::with_subject(&updated)?;
    context.insert_meta("post_id", id)?;
    state
        .plugins
        .dispatch(Hook::AfterPostUpdate, &mut context)
        .await?;

    Ok(updated)
}

pub async fn change_status(
    state: &AppState,
    id: i32,
    can_publish: bool,
    input: ChangePostStatusInput,
) -> AppResult<posts::Model> {
    let existing = show(state, id).await?;
    revisions_service::save(state, &existing).await?;

    let (status, published_at) =
        visibility::normalize_status(input.status, input.published_at, can_publish)?;
    let mut model: posts::ActiveModel = existing.into();
    model.status = Set(status);
    model.published_at = Set(published_at);
    model.updated_at = Set(Utc::now());

    Ok(model.update(&state.db).await?)
}

pub async fn revisions(state: &AppState, post_id: i32) -> AppResult<Vec<post_revisions::Model>> {
    revisions_service::list(state, post_id).await
}

pub async fn restore_revision(
    state: &AppState,
    post_id: i32,
    revision_id: i32,
) -> AppResult<posts::Model> {
    let existing = show(state, post_id).await?;
    revisions_service::restore(state, existing, revision_id).await
}
pub async fn delete(state: &AppState, id: i32) -> AppResult<posts::Model> {
    let model = show(state, id).await?;
    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::BeforePostDelete, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    maintenance::delete_record_and_dependents(state, id).await?;

    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::AfterPostDelete, &mut context)
        .await?;
    Ok(model)
}
pub async fn bulk_action(
    state: &AppState,
    current_user: &PublicUser,
    input: BulkPostActionInput,
) -> AppResult<BulkPostActionResponse> {
    let ids = maintenance::normalize_bulk_ids(input.ids)?;

    let mut posts = Vec::with_capacity(ids.len());
    for id in ids {
        let post = show(state, id).await?;
        current_user.require_content_owner_or_editor(post.author_id)?;
        let updated = match input.action {
            BulkPostAction::Publish => {
                change_status(
                    state,
                    id,
                    current_user.can_edit_all_content(),
                    ChangePostStatusInput {
                        status: PostStatus::Published,
                        published_at: input.published_at,
                        extensions: ExtensionMap::default(),
                    },
                )
                .await?
            }
            BulkPostAction::Archive => {
                change_status(
                    state,
                    id,
                    current_user.can_edit_all_content(),
                    ChangePostStatusInput {
                        status: PostStatus::Archived,
                        published_at: None,
                        extensions: ExtensionMap::default(),
                    },
                )
                .await?
            }
            BulkPostAction::Delete => delete(state, id).await?,
        };
        posts.push(updated);
    }

    Ok(BulkPostActionResponse {
        action: input.action,
        affected: posts.len(),
        posts,
    })
}
