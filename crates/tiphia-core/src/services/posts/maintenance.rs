use crate::{
    app::AppState,
    entities::{comments, post_revisions, post_terms, posts},
    error::{AppError, AppResult},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub fn normalize_bulk_ids(mut ids: Vec<i32>) -> AppResult<Vec<i32>> {
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

    Ok(ids)
}

pub async fn delete_record_and_dependents(state: &AppState, post_id: i32) -> AppResult<()> {
    comments::Entity::delete_many()
        .filter(comments::Column::PostId.eq(post_id))
        .exec(&state.db)
        .await?;
    post_terms::Entity::delete_many()
        .filter(post_terms::Column::PostId.eq(post_id))
        .exec(&state.db)
        .await?;
    post_revisions::Entity::delete_many()
        .filter(post_revisions::Column::PostId.eq(post_id))
        .exec(&state.db)
        .await?;
    posts::Entity::delete_by_id(post_id).exec(&state.db).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_bulk_ids_sorts_and_deduplicates() {
        assert_eq!(normalize_bulk_ids(vec![3, 1, 3, 2]).unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn normalize_bulk_ids_rejects_empty_input() {
        assert!(matches!(
            normalize_bulk_ids(Vec::new()),
            Err(AppError::Validation(message)) if message == "ids is required"
        ));
    }

    #[test]
    fn normalize_bulk_ids_rejects_large_batches() {
        let ids = (1..=101).collect::<Vec<_>>();
        assert!(matches!(
            normalize_bulk_ids(ids),
            Err(AppError::Validation(message)) if message == "bulk action supports at most 100 ids"
        ));
    }
}
