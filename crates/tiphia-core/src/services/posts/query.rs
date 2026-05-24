use crate::{
    app::AppState,
    entities::{post_terms, posts},
    error::AppResult,
    services::{auth::PublicUser, posts::ListPostQuery},
};
use chrono::Utc;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Select};

use super::visibility;

pub async fn build_list_select(
    state: &AppState,
    query: &ListPostQuery,
    current_user: Option<&PublicUser>,
) -> AppResult<Select<posts::Entity>> {
    let mut select = posts::Entity::find().order_by_desc(posts::Column::CreatedAt);

    select = apply_visibility(select, current_user);
    select = apply_status_filter(select, query);
    select = apply_post_type_filter(select, query);
    select = apply_search_filter(select, query);
    select = apply_term_filter(state, select, query).await?;

    Ok(select)
}

fn apply_visibility(
    select: Select<posts::Entity>,
    current_user: Option<&PublicUser>,
) -> Select<posts::Entity> {
    if let Some(current_user) = current_user {
        if current_user.can_edit_all_content() {
            return select;
        }
        return select.filter(posts::Column::AuthorId.eq(current_user.id));
    }

    select.filter(visibility::public_visible_condition(Utc::now()))
}

fn apply_status_filter(
    select: Select<posts::Entity>,
    query: &ListPostQuery,
) -> Select<posts::Entity> {
    if let Some(status) = query.status.clone() {
        return select.filter(posts::Column::Status.eq(status));
    }
    select
}

fn apply_post_type_filter(
    select: Select<posts::Entity>,
    query: &ListPostQuery,
) -> Select<posts::Entity> {
    if let Some(post_type) = query.post_type.clone() {
        return select.filter(posts::Column::PostType.eq(post_type));
    }
    select
}

fn apply_search_filter(
    select: Select<posts::Entity>,
    query: &ListPostQuery,
) -> Select<posts::Entity> {
    let Some(q) = query
        .q
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    else {
        return select;
    };

    let pattern = format!("%{q}%");
    select.filter(
        posts::Column::Title
            .contains(q)
            .or(posts::Column::Markdown.like(pattern.clone()))
            .or(posts::Column::Excerpt.like(pattern)),
    )
}

async fn apply_term_filter(
    state: &AppState,
    select: Select<posts::Entity>,
    query: &ListPostQuery,
) -> AppResult<Select<posts::Entity>> {
    let Some(term_id) = query.term_id else {
        return Ok(select);
    };

    let relations = post_terms::Entity::find()
        .filter(post_terms::Column::TermId.eq(term_id))
        .all(&state.db)
        .await?;
    let post_ids = relations
        .into_iter()
        .map(|relation| relation.post_id)
        .collect::<Vec<_>>();

    Ok(select.filter(posts::Column::Id.is_in(post_ids)))
}
