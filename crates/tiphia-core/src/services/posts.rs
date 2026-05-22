use crate::{
    app::AppState,
    entities::{
        comments, options, post_revisions, post_terms,
        posts::{self, PostStatus, PostType},
    },
    error::{AppError, AppResult, validation_on_unique},
    pagination::{Page, PaginationQuery},
    plugins::{Hook, HookContext},
    services::{
        auth::PublicUser,
        render::{RenderInput, render_content},
        validation,
    },
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ListPostQuery {
    pub q: Option<String>,
    pub status: Option<PostStatus>,
    pub post_type: Option<PostType>,
    pub term_id: Option<i32>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PostResponse {
    #[serde(flatten)]
    pub post: posts::Model,
    pub permalink: String,
    pub view_count: u64,
    pub comment_count: u64,
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
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangePostStatusInput {
    pub status: PostStatus,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct BulkPostActionInput {
    pub ids: Vec<i32>,
    pub action: BulkPostAction,
    pub published_at: Option<DateTime<Utc>>,
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

pub async fn list(state: &AppState, query: ListPostQuery) -> AppResult<Page<PostResponse>> {
    list_with_visibility(state, query, None).await
}

pub async fn list_admin(
    state: &AppState,
    query: ListPostQuery,
    current_user: &PublicUser,
) -> AppResult<Page<PostResponse>> {
    list_with_visibility(state, query, Some(current_user)).await
}

async fn list_with_visibility(
    state: &AppState,
    query: ListPostQuery,
    current_user: Option<&PublicUser>,
) -> AppResult<Page<PostResponse>> {
    let hook = match query.post_type {
        Some(PostType::Page) => Hook::BeforePageList,
        _ => Hook::BeforePostList,
    };
    let mut context = HookContext::with_subject(&query)?;
    state.plugins.dispatch(hook, &mut context).await?;

    let mut select = posts::Entity::find().order_by_desc(posts::Column::CreatedAt);
    let now = Utc::now();

    if let Some(current_user) = current_user {
        if !current_user.can_edit_all_content() {
            select = select.filter(posts::Column::AuthorId.eq(current_user.id));
        }
    } else {
        select = select.filter(public_visible_condition(now));
    }
    if let Some(status) = query.status.clone() {
        select = select.filter(posts::Column::Status.eq(status));
    }

    if let Some(post_type) = query.post_type.clone() {
        select = select.filter(posts::Column::PostType.eq(post_type));
    }

    if let Some(q) = query
        .q
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        let pattern = format!("%{q}%");
        select = select.filter(
            posts::Column::Title
                .contains(q)
                .or(posts::Column::Markdown.like(pattern.clone()))
                .or(posts::Column::Excerpt.like(pattern)),
        );
    }

    if let Some(term_id) = query.term_id {
        let relations = post_terms::Entity::find()
            .filter(post_terms::Column::TermId.eq(term_id))
            .all(&state.db)
            .await?;
        let post_ids = relations
            .into_iter()
            .map(|relation| relation.post_id)
            .collect::<Vec<_>>();
        select = select.filter(posts::Column::Id.is_in(post_ids));
    }

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = select.paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let settings = crate::services::settings::get(state).await?;
    let page_posts = paginator.fetch_page(page - 1).await?;
    let mut items = Vec::with_capacity(page_posts.len());
    for post in page_posts {
        items.push(response_for_with_settings(state, &settings, post).await?);
    }
    let after_hook = match query.post_type {
        Some(PostType::Page) => Hook::AfterPageList,
        _ => Hook::AfterPostList,
    };
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

pub async fn show_response(state: &AppState, id: i32) -> AppResult<PostResponse> {
    let post = show(state, id).await?;
    ensure_publicly_visible(&post)?;
    increment_view_count(state, post.id).await?;
    response_for(state, post).await
}

pub async fn show_admin_response(
    state: &AppState,
    id: i32,
    current_user: &PublicUser,
) -> AppResult<PostResponse> {
    let post = show(state, id).await?;
    current_user.require_content_owner_or_editor(post.author_id)?;
    response_for(state, post).await
}

pub async fn show_by_slug(
    state: &AppState,
    post_type: PostType,
    slug: &str,
) -> AppResult<PostResponse> {
    let post = posts::Entity::find()
        .filter(posts::Column::Slug.eq(slug.to_owned()))
        .filter(posts::Column::PostType.eq(post_type))
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("post"))?;

    ensure_publicly_visible(&post)?;
    increment_view_count(state, post.id).await?;
    response_for(state, post).await
}

pub async fn popular(state: &AppState, limit: u64) -> AppResult<Vec<PostResponse>> {
    let limit = limit.clamp(1, 20);
    let posts = posts::Entity::find()
        .filter(public_visible_condition(Utc::now()))
        .filter(posts::Column::PostType.eq(PostType::Post))
        .all(&state.db)
        .await?;
    let settings = crate::services::settings::get(state).await?;
    let mut responses = Vec::with_capacity(posts.len());
    for post in posts {
        responses.push(response_for_with_settings(state, &settings, post).await?);
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

    let post_type = input.post_type.clone().unwrap_or(PostType::Post);
    let hook = match post_type {
        PostType::Page => Hook::BeforePageCreate,
        PostType::Post => Hook::BeforePostCreate,
    };
    let mut context = HookContext::with_subject(&input)?;
    state.plugins.dispatch(hook, &mut context).await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<CreatePostInput>()? {
        input = next_input;
    }
    validation::required(&input.title, "title")?;
    let slug = normalize_create_slug(state, &input.slug, &input.title, &post_type).await?;

    let now = Utc::now();
    let (status, published_at) = normalize_status(
        input.status.unwrap_or(PostStatus::Draft),
        input.published_at,
        can_publish,
    )?;
    let rendered = render_content(
        state,
        RenderInput {
            markdown: input.markdown,
            html: input.html,
            excerpt: input.excerpt,
        },
    )
    .await?;
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

    let after_hook = match post_type {
        PostType::Page => Hook::AfterPageCreate,
        PostType::Post => Hook::AfterPostCreate,
    };
    let mut context = HookContext::with_subject(&model)?;
    state.plugins.dispatch(after_hook, &mut context).await?;

    Ok(model)
}

async fn normalize_create_slug(
    state: &AppState,
    raw_slug: &str,
    title: &str,
    post_type: &PostType,
) -> AppResult<String> {
    let slug = raw_slug.trim();
    if !slug.is_empty() {
        validation::slug(slug)?;
        return Ok(slug.to_owned());
    }

    let fallback = match post_type {
        PostType::Post => "post",
        PostType::Page => "page",
    };
    let base = slugify(title).unwrap_or_else(|| fallback.to_owned());
    unique_slug(state, &base).await
}

async fn unique_slug(state: &AppState, base: &str) -> AppResult<String> {
    let mut candidate = base.to_owned();
    let mut suffix = 2;

    loop {
        let exists = posts::Entity::find()
            .filter(posts::Column::Slug.eq(candidate.clone()))
            .one(&state.db)
            .await?
            .is_some();
        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}

fn slugify(value: &str) -> Option<String> {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in value.trim().chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() { None } else { Some(slug) }
}

pub async fn update(
    state: &AppState,
    id: i32,
    can_publish: bool,
    mut input: UpdatePostInput,
) -> AppResult<posts::Model> {
    let mut context = HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(Hook::BeforePostUpdate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<UpdatePostInput>()? {
        input = next_input;
    }

    let existing = show(state, id).await?;
    save_revision(state, &existing).await?;
    let next_markdown = input
        .markdown
        .clone()
        .unwrap_or_else(|| existing.markdown.clone());
    let should_render = input.markdown.is_some() || input.html.is_some() || input.excerpt.is_some();
    let rendered = if should_render {
        Some(
            render_content(
                state,
                RenderInput {
                    markdown: next_markdown,
                    html: input.html.clone(),
                    excerpt: input.excerpt.clone(),
                },
            )
            .await?,
        )
    } else {
        None
    };
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
        let (status, published_at) = normalize_status(status, input.published_at, can_publish)?;
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
    save_revision(state, &existing).await?;

    let (status, published_at) = normalize_status(input.status, input.published_at, can_publish)?;
    let mut model: posts::ActiveModel = existing.into();
    model.status = Set(status);
    model.published_at = Set(published_at);
    model.updated_at = Set(Utc::now());

    Ok(model.update(&state.db).await?)
}

pub async fn revisions(state: &AppState, post_id: i32) -> AppResult<Vec<post_revisions::Model>> {
    Ok(post_revisions::Entity::find()
        .filter(post_revisions::Column::PostId.eq(post_id))
        .order_by_desc(post_revisions::Column::CreatedAt)
        .all(&state.db)
        .await?)
}

pub async fn restore_revision(
    state: &AppState,
    post_id: i32,
    revision_id: i32,
) -> AppResult<posts::Model> {
    let existing = show(state, post_id).await?;
    save_revision(state, &existing).await?;

    let revision = post_revisions::Entity::find_by_id(revision_id)
        .one(&state.db)
        .await?
        .filter(|revision| revision.post_id == post_id)
        .ok_or(AppError::NotFound("revision"))?;

    let mut model: posts::ActiveModel = existing.into();
    model.title = Set(revision.title);
    model.markdown = Set(revision.markdown);
    model.html = Set(revision.html);
    model.excerpt = Set(revision.excerpt);
    model.status = Set(revision.status);
    model.updated_at = Set(Utc::now());

    Ok(model.update(&state.db).await?)
}

pub async fn delete(state: &AppState, id: i32) -> AppResult<posts::Model> {
    let model = show(state, id).await?;
    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::BeforePostDelete, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    comments::Entity::delete_many()
        .filter(comments::Column::PostId.eq(id))
        .exec(&state.db)
        .await?;
    post_terms::Entity::delete_many()
        .filter(post_terms::Column::PostId.eq(id))
        .exec(&state.db)
        .await?;
    post_revisions::Entity::delete_many()
        .filter(post_revisions::Column::PostId.eq(id))
        .exec(&state.db)
        .await?;
    posts::Entity::delete_by_id(id).exec(&state.db).await?;

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
    let mut ids = input.ids;
    ids.sort_unstable();
    ids.dedup();

    if ids.is_empty() {
        return Err(AppError::Validation("ids is required".to_owned()));
    }
    if ids.len() > 100 {
        return Err(AppError::Validation(
            "bulk action supports at most 100 ids".to_owned(),
        ));
    }

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

fn normalize_status(
    status: PostStatus,
    published_at: Option<DateTime<Utc>>,
    can_publish: bool,
) -> AppResult<(PostStatus, Option<DateTime<Utc>>)> {
    if matches!(status, PostStatus::Published | PostStatus::Scheduled) && !can_publish {
        return Err(AppError::Forbidden);
    }

    let now = Utc::now();
    match status {
        PostStatus::Published => Ok((PostStatus::Published, Some(published_at.unwrap_or(now)))),
        PostStatus::Scheduled => {
            let publish_at = published_at.ok_or_else(|| {
                AppError::Validation("published_at is required for scheduled posts".to_owned())
            })?;
            if publish_at <= now {
                return Ok((PostStatus::Published, Some(publish_at)));
            }
            Ok((PostStatus::Scheduled, Some(publish_at)))
        }
        PostStatus::PendingReview => Ok((PostStatus::PendingReview, None)),
        PostStatus::Draft | PostStatus::Archived => Ok((status, published_at)),
    }
}

fn ensure_publicly_visible(post: &posts::Model) -> AppResult<()> {
    let now = Utc::now();
    let visible = matches!(post.status, PostStatus::Published)
        || (matches!(post.status, PostStatus::Scheduled)
            && post
                .published_at
                .map(|published_at| published_at <= now)
                .unwrap_or(false));

    if visible {
        return Ok(());
    }

    Err(AppError::NotFound("post"))
}

fn public_visible_condition(now: DateTime<Utc>) -> Condition {
    Condition::any()
        .add(posts::Column::Status.eq(PostStatus::Published))
        .add(
            Condition::all()
                .add(posts::Column::Status.eq(PostStatus::Scheduled))
                .add(posts::Column::PublishedAt.lte(now)),
        )
}

async fn save_revision(state: &AppState, post: &posts::Model) -> AppResult<()> {
    post_revisions::ActiveModel {
        post_id: Set(post.id),
        title: Set(post.title.clone()),
        markdown: Set(post.markdown.clone()),
        html: Set(post.html.clone()),
        excerpt: Set(post.excerpt.clone()),
        status: Set(post.status.clone()),
        author_id: Set(post.author_id),
        created_at: Set(Utc::now()),
        ..Default::default()
    }
    .insert(&state.db)
    .await?;

    Ok(())
}

async fn response_for(state: &AppState, post: posts::Model) -> AppResult<PostResponse> {
    let settings = crate::services::settings::get(state).await?;
    response_for_with_settings(state, &settings, post).await
}

async fn response_for_with_settings(
    state: &AppState,
    settings: &crate::services::settings::SiteSettings,
    post: posts::Model,
) -> AppResult<PostResponse> {
    let view_count = view_count(state, post.id).await?;
    let comment_count = approved_comment_count(state, post.id).await?;
    Ok(PostResponse {
        permalink: permalink_for(&settings.permalink_format, &post),
        post,
        view_count,
        comment_count,
    })
}

async fn approved_comment_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    Ok(comments::Entity::find()
        .filter(comments::Column::PostId.eq(post_id))
        .filter(comments::Column::Status.eq(crate::entities::comments::CommentStatus::Approved))
        .count(&state.db)
        .await?)
}

async fn view_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    Ok(options::Entity::find()
        .filter(options::Column::Key.eq(view_count_key(post_id)))
        .one(&state.db)
        .await?
        .and_then(|option| {
            option
                .value
                .get("count")
                .and_then(serde_json::Value::as_u64)
        })
        .unwrap_or(0))
}

async fn increment_view_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    let key = view_count_key(post_id);
    let now = Utc::now();
    if let Some(existing) = options::Entity::find()
        .filter(options::Column::Key.eq(key.clone()))
        .one(&state.db)
        .await?
    {
        let next = existing
            .value
            .get("count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .saturating_add(1);
        let mut model: options::ActiveModel = existing.into();
        model.value = Set(json!({ "count": next }));
        model.updated_at = Set(now);
        model.update(&state.db).await?;
        Ok(next)
    } else {
        options::ActiveModel {
            key: Set(key),
            value: Set(json!({ "count": 1_u64 })),
            autoload: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await?;
        Ok(1)
    }
}

fn view_count_key(post_id: i32) -> String {
    format!("post:view:{post_id}")
}

fn permalink_for(format: &str, post: &posts::Model) -> String {
    let published_at = post.published_at.unwrap_or(post.created_at);
    format
        .replace("{id}", &post.id.to_string())
        .replace("{slug}", &post.slug)
        .replace("{year}", &published_at.format("%Y").to_string())
        .replace("{month}", &published_at.format("%m").to_string())
        .replace("{day}", &published_at.format("%d").to_string())
}
