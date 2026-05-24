use crate::{
    app::AppState,
    entities::terms::{self, TermType},
    error::{AppError, AppResult, validation_on_unique},
    pagination::{Page, PaginationQuery},
    plugins::{Hook, HookContext},
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
};
use serde::{Deserialize, Serialize};

#[path = "terms/relations.rs"]
mod relations;
#[path = "terms/response.rs"]
mod response;
#[path = "terms/slug.rs"]
mod slug;
pub use response::TermResponse;
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ListTermQuery {
    pub term_type: Option<TermType>,
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateTermInput {
    #[serde(default)]
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub term_type: TermType,
    pub parent_id: Option<i32>,
    pub sort_order: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateTermInput {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub sort_order: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct SyncPostTermsInput {
    pub term_ids: Vec<i32>,
}

pub async fn list(state: &AppState, query: ListTermQuery) -> AppResult<Page<TermResponse>> {
    let mut select = terms::Entity::find().order_by_asc(terms::Column::SortOrder);

    if let Some(term_type) = query.term_type {
        select = select.filter(terms::Column::TermType.eq(term_type));
    }

    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = select.paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let terms = paginator.fetch_page(page - 1).await?;
    let mut items = Vec::with_capacity(terms.len());
    for term in terms {
        items.push(response::for_term(state, term).await?);
    }

    Ok(Page::new(items, page, per_page, total, total_pages))
}

pub async fn show(state: &AppState, id: i32) -> AppResult<terms::Model> {
    terms::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("term"))
}

pub async fn create(state: &AppState, mut input: CreateTermInput) -> AppResult<terms::Model> {
    validate_required(&input.name, "name")?;

    let mut context = HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(Hook::BeforeTermCreate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<CreateTermInput>()? {
        input = next_input;
    }
    validate_required(&input.name, "name")?;
    let slug = slug::normalize_create_slug(state, &input.slug, &input.name).await?;

    let now = Utc::now();
    let model = terms::ActiveModel {
        slug: Set(slug),
        name: Set(input.name),
        description: Set(input.description),
        term_type: Set(input.term_type),
        parent_id: Set(input.parent_id),
        sort_order: Set(input.sort_order.unwrap_or(0)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|err| validation_on_unique(err, "term slug already exists"))?;

    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::AfterTermCreate, &mut context)
        .await?;

    Ok(model)
}

pub async fn update(
    state: &AppState,
    id: i32,
    mut input: UpdateTermInput,
) -> AppResult<terms::Model> {
    let mut context = HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(Hook::BeforeTermUpdate, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_input) = context.take_subject::<UpdateTermInput>()? {
        input = next_input;
    }

    let existing = show(state, id).await?;
    let mut model: terms::ActiveModel = existing.into();

    if let Some(slug) = input.slug {
        validate_required(&slug, "slug")?;
        crate::services::validation::slug(&slug)?;
        model.slug = Set(slug);
    }
    if let Some(name) = input.name {
        validate_required(&name, "name")?;
        model.name = Set(name);
    }
    if let Some(description) = input.description {
        model.description = Set(Some(description));
    }
    if let Some(parent_id) = input.parent_id {
        model.parent_id = Set(Some(parent_id));
    }
    if let Some(sort_order) = input.sort_order {
        model.sort_order = Set(sort_order);
    }
    model.updated_at = Set(Utc::now());

    let updated = model
        .update(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "term slug already exists"))?;
    let mut context = HookContext::with_subject(&updated)?;
    state
        .plugins
        .dispatch(Hook::AfterTermUpdate, &mut context)
        .await?;

    Ok(updated)
}

pub async fn delete(state: &AppState, id: i32) -> AppResult<terms::Model> {
    let model = show(state, id).await?;
    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::BeforeTermDelete, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    relations::delete_term_relations(state, id).await?;

    terms::Entity::delete_by_id(id).exec(&state.db).await?;

    let mut context = HookContext::with_subject(&model)?;
    state
        .plugins
        .dispatch(Hook::AfterTermDelete, &mut context)
        .await?;

    Ok(model)
}

pub async fn terms_for_post(state: &AppState, post_id: i32) -> AppResult<Vec<terms::Model>> {
    relations::terms_for_post(state, post_id).await
}
pub async fn sync_post_terms(
    state: &AppState,
    post_id: i32,
    input: SyncPostTermsInput,
) -> AppResult<Vec<terms::Model>> {
    let mut term_ids = relations::normalize_term_ids(input.term_ids);

    let mut context = HookContext::with_subject(&term_ids)?;
    context.insert_meta("post_id", post_id)?;
    state
        .plugins
        .dispatch(Hook::BeforePostTermsSync, &mut context)
        .await?;
    context.ensure_not_stopped()?;
    if let Some(next_term_ids) = context.take_subject::<Vec<i32>>()? {
        term_ids = relations::normalize_term_ids(next_term_ids);
    }

    relations::replace_post_terms(state, post_id, term_ids).await?;

    let terms = terms_for_post(state, post_id).await?;
    let mut context = HookContext::with_subject(&terms)?;
    context.insert_meta("post_id", post_id)?;
    state
        .plugins
        .dispatch(Hook::AfterPostTermsSync, &mut context)
        .await?;

    Ok(terms)
}

fn validate_required(value: &str, field: &'static str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }

    Ok(())
}
