use crate::{
    app::AppState,
    entities::{post_terms, terms},
    error::AppResult,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

pub fn normalize_term_ids(mut term_ids: Vec<i32>) -> Vec<i32> {
    term_ids.sort_unstable();
    term_ids.dedup();
    term_ids
}

pub async fn terms_for_post(state: &AppState, post_id: i32) -> AppResult<Vec<terms::Model>> {
    let relations = post_terms::Entity::find()
        .filter(post_terms::Column::PostId.eq(post_id))
        .all(&state.db)
        .await?;

    let mut items = Vec::with_capacity(relations.len());
    for relation in relations {
        if let Some(term) = terms::Entity::find_by_id(relation.term_id)
            .one(&state.db)
            .await?
        {
            items.push(term);
        }
    }

    Ok(items)
}

pub async fn replace_post_terms(
    state: &AppState,
    post_id: i32,
    term_ids: Vec<i32>,
) -> AppResult<()> {
    post_terms::Entity::delete_many()
        .filter(post_terms::Column::PostId.eq(post_id))
        .exec(&state.db)
        .await?;

    let now = Utc::now();
    for term_id in term_ids {
        post_terms::ActiveModel {
            post_id: Set(post_id),
            term_id: Set(term_id),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&state.db)
        .await?;
    }

    Ok(())
}

pub async fn delete_term_relations(state: &AppState, term_id: i32) -> AppResult<()> {
    post_terms::Entity::delete_many()
        .filter(post_terms::Column::TermId.eq(term_id))
        .exec(&state.db)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_term_ids_sorts_and_deduplicates() {
        assert_eq!(normalize_term_ids(vec![3, 1, 3, 2]), vec![1, 2, 3]);
    }
}
