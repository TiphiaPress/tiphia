use crate::{
    app::AppState,
    entities::{comments, posts},
    error::AppResult,
    services::{options, settings::SiteSettings},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PostResponse {
    #[serde(flatten)]
    pub post: posts::Model,
    pub permalink: String,
    pub view_count: u64,
    pub comment_count: u64,
}

pub async fn for_post(state: &AppState, post: posts::Model) -> AppResult<PostResponse> {
    let settings = crate::services::settings::get(state).await?;
    for_post_with_settings(state, &settings, post).await
}

pub async fn for_post_with_settings(
    state: &AppState,
    settings: &SiteSettings,
    post: posts::Model,
) -> AppResult<PostResponse> {
    let view_count = view_count(state, post.id).await?;
    let comment_count = approved_comment_count(state, post.id).await?;
    Ok(PostResponse {
        permalink: permalink_for(settings, &post),
        post,
        view_count,
        comment_count,
    })
}

pub async fn increment_view_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    let value = options::update_json(state, &view_count_key(post_id), false, |current| {
        let next = current
            .and_then(|value| {
                value
                    .get("count")
                    .and_then(serde_json::Value::as_u64)
                    .map(|count| count.saturating_add(1))
            })
            .unwrap_or(1);
        serde_json::json!({ "count": next })
    })
    .await?;

    Ok(value
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1))
}

pub fn view_count_key(post_id: i32) -> String {
    format!("post:view:{post_id}")
}

async fn approved_comment_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    Ok(comments::Entity::find()
        .filter(comments::Column::PostId.eq(post_id))
        .filter(comments::Column::Status.eq(comments::CommentStatus::Approved))
        .count(&state.db)
        .await?)
}

async fn view_count(state: &AppState, post_id: i32) -> AppResult<u64> {
    Ok(options::get_json(state, &view_count_key(post_id))
        .await?
        .and_then(|value| value.get("count").and_then(serde_json::Value::as_u64))
        .unwrap_or(0))
}

fn permalink_for(settings: &SiteSettings, post: &posts::Model) -> String {
    if matches!(&post.post_type, posts::PostType::Page) {
        return format!("/pages/{}", post.slug);
    }

    let published_at = post.published_at.unwrap_or(post.created_at);
    settings
        .permalink_format
        .replace("{id}", &post.id.to_string())
        .replace("{slug}", &post.slug)
        .replace("{year}", &published_at.format("%Y").to_string())
        .replace("{month}", &published_at.format("%m").to_string())
        .replace("{day}", &published_at.format("%d").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::posts::{PostStatus, PostType};
    use chrono::Utc;

    fn post() -> posts::Model {
        let now = Utc::now();
        posts::Model {
            id: 42,
            slug: "hello".to_owned(),
            title: "Hello".to_owned(),
            markdown: String::new(),
            html: String::new(),
            excerpt: None,
            status: PostStatus::Published,
            post_type: PostType::Post,
            author_id: 1,
            published_at: Some(now),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn view_count_key_uses_documented_option_contract() {
        assert_eq!(view_count_key(42), "post:view:42");
    }

    #[test]
    fn permalink_replaces_supported_tokens() {
        let post = post();
        let settings = SiteSettings {
            permalink_format: "/posts/{id}/{slug}/{year}/{month}/{day}".to_owned(),
            ..SiteSettings::default()
        };
        let permalink = permalink_for(&settings, &post);

        assert!(permalink.contains("/42/hello/"));
        assert!(!permalink.contains("{slug}"));
    }
}
