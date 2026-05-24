use crate::{
    app::AppState,
    entities::{post_revisions, posts},
    error::{AppError, AppResult},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};

pub async fn list(state: &AppState, post_id: i32) -> AppResult<Vec<post_revisions::Model>> {
    Ok(post_revisions::Entity::find()
        .filter(post_revisions::Column::PostId.eq(post_id))
        .order_by_desc(post_revisions::Column::CreatedAt)
        .all(&state.db)
        .await?)
}

pub async fn save(state: &AppState, post: &posts::Model) -> AppResult<()> {
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

pub async fn restore(
    state: &AppState,
    existing: posts::Model,
    revision_id: i32,
) -> AppResult<posts::Model> {
    save(state, &existing).await?;

    let revision = post_revisions::Entity::find_by_id(revision_id)
        .one(&state.db)
        .await?
        .filter(|revision| revision.post_id == existing.id)
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
