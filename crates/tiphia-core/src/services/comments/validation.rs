use crate::{
    app::AppState,
    entities::{comments, posts},
    error::{AppError, AppResult},
    services::validation,
};
use sea_orm::EntityTrait;

use super::CreateCommentInput;

pub fn validate_create_input(input: &CreateCommentInput) -> AppResult<()> {
    validation::required(&input.author_name, "author_name")?;
    validation::max_len(&input.author_name, 120, "author_name")?;
    validation::required(&input.author_email, "author_email")?;
    validation::max_len(&input.author_email, 254, "author_email")?;
    validation::email(&input.author_email, "author_email")?;
    validation::optional_http_url(input.author_url.as_deref(), "author_url")?;
    validation::required(&input.content, "content")?;
    validation::max_len(&input.content, 10_000, "content")?;
    Ok(())
}

pub async fn ensure_post_exists(state: &AppState, post_id: i32) -> AppResult<()> {
    posts::Entity::find_by_id(post_id)
        .one(&state.db)
        .await?
        .ok_or(AppError::Validation("post not found".to_owned()))?;
    Ok(())
}

pub async fn ensure_parent_belongs_to_post(
    state: &AppState,
    post_id: i32,
    parent_id: Option<i32>,
) -> AppResult<()> {
    let Some(parent_id) = parent_id else {
        return Ok(());
    };

    let parent = comments::Entity::find_by_id(parent_id)
        .one(&state.db)
        .await?
        .ok_or(AppError::Validation("parent comment not found".to_owned()))?;

    if parent.post_id != post_id {
        return Err(AppError::Validation(
            "parent comment belongs to another post".to_owned(),
        ));
    }

    Ok(())
}
