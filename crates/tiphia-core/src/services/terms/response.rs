use crate::{
    app::AppState,
    entities::{post_terms, posts, terms},
    error::AppResult,
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TermResponse {
    #[serde(flatten)]
    pub term: terms::Model,
    pub post_count: u64,
}

pub async fn for_term(state: &AppState, term: terms::Model) -> AppResult<TermResponse> {
    let post_count = post_count(state, term.id).await?;
    Ok(TermResponse { term, post_count })
}

async fn post_count(state: &AppState, term_id: i32) -> AppResult<u64> {
    Ok(post_terms::Entity::find()
        .filter(post_terms::Column::TermId.eq(term_id))
        .find_also_related(posts::Entity)
        .all(&state.db)
        .await?
        .into_iter()
        .filter(|(_, post)| post.is_some())
        .count() as u64)
}
