use crate::{
    entities::posts::{self, PostStatus},
    error::{AppError, AppResult},
};
use chrono::{DateTime, Utc};
use sea_orm::{ColumnTrait, Condition};

pub fn normalize_status(
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

pub fn ensure_publicly_visible(post: &posts::Model) -> AppResult<()> {
    if is_publicly_visible(post, Utc::now()) {
        return Ok(());
    }

    Err(AppError::NotFound("post"))
}

pub fn is_publicly_visible(post: &posts::Model, now: DateTime<Utc>) -> bool {
    matches!(post.status, PostStatus::Published)
        || (matches!(post.status, PostStatus::Scheduled)
            && post
                .published_at
                .map(|published_at| published_at <= now)
                .unwrap_or(false))
}

pub fn public_visible_condition(now: DateTime<Utc>) -> Condition {
    Condition::any()
        .add(posts::Column::Status.eq(PostStatus::Published))
        .add(
            Condition::all()
                .add(posts::Column::Status.eq(PostStatus::Scheduled))
                .add(posts::Column::PublishedAt.lte(now)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::posts::PostType;

    fn post(status: PostStatus, published_at: Option<DateTime<Utc>>) -> posts::Model {
        let now = Utc::now();
        posts::Model {
            id: 1,
            slug: "post".to_owned(),
            title: "Post".to_owned(),
            markdown: String::new(),
            html: String::new(),
            excerpt: None,
            status,
            post_type: PostType::Post,
            author_id: 1,
            published_at,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn scheduled_post_is_visible_after_publish_time() {
        let now = Utc::now();
        assert!(is_publicly_visible(
            &post(
                PostStatus::Scheduled,
                Some(now - chrono::Duration::seconds(1))
            ),
            now
        ));
        assert!(!is_publicly_visible(
            &post(
                PostStatus::Scheduled,
                Some(now + chrono::Duration::seconds(1))
            ),
            now
        ));
    }

    #[test]
    fn normalize_status_requires_permission_to_publish() {
        assert!(matches!(
            normalize_status(PostStatus::Published, None, false),
            Err(AppError::Forbidden)
        ));
    }
}
