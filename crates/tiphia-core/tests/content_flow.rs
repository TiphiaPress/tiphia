mod support;

use chrono::{Duration, Utc};
use tiphia_core::{
    AppError,
    entities::{
        comments::CommentStatus,
        posts::{PostStatus, PostType},
    },
    services::{
        auth::{self, BootstrapAdminInput},
        comments::{self, CreateCommentInput},
        posts::{self, CreatePostInput, ListPostQuery},
    },
};

#[tokio::test]
async fn create_post_and_comment_tree() {
    let state = support::state().await;
    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    let post = posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "hello-world".to_owned(),
            title: "Hello World".to_owned(),
            markdown: "# Hello".to_owned(),
            html: None,
            excerpt: None,
            status: Some(PostStatus::Published),
            post_type: Some(PostType::Post),
            published_at: None,
        },
    )
    .await
    .expect("create post");

    let parent = comments::create(
        &state,
        CreateCommentInput {
            post_id: post.id,
            parent_id: None,
            author_name: "Alice".to_owned(),
            author_email: "alice@example.com".to_owned(),
            author_url: Some("https://example.com".to_owned()),
            content: "First".to_owned(),
            captcha: None,
        },
    )
    .await
    .expect("create parent comment");

    comments::create(
        &state,
        CreateCommentInput {
            post_id: post.id,
            parent_id: Some(parent.id),
            author_name: "Bob".to_owned(),
            author_email: "bob@example.com".to_owned(),
            author_url: None,
            content: "Reply".to_owned(),
            captcha: None,
        },
    )
    .await
    .expect("create child comment");

    let tree = comments::tree_for_post(&state, post.id, Some(CommentStatus::Pending))
        .await
        .expect("comment tree");

    assert_eq!(tree.len(), 1);
    assert_eq!(tree[0].children.len(), 1);
}

#[tokio::test]
async fn duplicate_post_slug_returns_validation_error() {
    let state = support::state().await;
    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    let input = CreatePostInput {
        slug: "duplicate".to_owned(),
        title: "Duplicate".to_owned(),
        markdown: "content".to_owned(),
        html: None,
        excerpt: None,
        status: None,
        post_type: None,
        published_at: None,
    };

    posts::create(&state, admin.id, true, input.clone())
        .await
        .expect("first post");
    let err = posts::create(&state, admin.id, true, input)
        .await
        .expect_err("duplicate slug should fail");

    assert!(matches!(err, AppError::Validation(_)));
}

#[tokio::test]
async fn scheduled_posts_are_hidden_until_due() {
    let state = support::state().await;
    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "future-post".to_owned(),
            title: "Future Post".to_owned(),
            markdown: "soon".to_owned(),
            html: None,
            excerpt: None,
            status: Some(PostStatus::Scheduled),
            post_type: Some(PostType::Post),
            published_at: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await
    .expect("create scheduled post");

    let page = posts::list(
        &state,
        ListPostQuery {
            q: None,
            status: None,
            post_type: Some(PostType::Post),
            term_id: None,
            pagination: Default::default(),
        },
    )
    .await
    .expect("list posts");

    assert_eq!(page.meta.total, 0);
}

#[tokio::test]
async fn unpublished_posts_are_hidden_from_public_detail_routes() {
    let state = support::state().await;
    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    let draft = posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "hidden-draft".to_owned(),
            title: "Hidden Draft".to_owned(),
            markdown: "draft".to_owned(),
            html: None,
            excerpt: None,
            status: Some(PostStatus::Draft),
            post_type: Some(PostType::Post),
            published_at: None,
        },
    )
    .await
    .expect("create draft");

    assert!(matches!(
        posts::show_response(&state, draft.id).await,
        Err(AppError::NotFound("post"))
    ));
    assert!(matches!(
        posts::show_by_slug(&state, PostType::Post, "hidden-draft").await,
        Err(AppError::NotFound("post"))
    ));
}

#[tokio::test]
async fn public_post_list_hides_unpublished_status_filters() {
    let state = support::state().await;
    let admin = auth::bootstrap_admin(
        &state,
        BootstrapAdminInput {
            username: "admin".to_owned(),
            email: "admin@example.com".to_owned(),
            password: "long-enough-password".to_owned(),
            display_name: None,
        },
    )
    .await
    .expect("bootstrap admin")
    .user;

    posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "filtered-draft".to_owned(),
            title: "Filtered Draft".to_owned(),
            markdown: "draft".to_owned(),
            html: None,
            excerpt: None,
            status: Some(PostStatus::Draft),
            post_type: Some(PostType::Post),
            published_at: None,
        },
    )
    .await
    .expect("create draft");
    posts::create(
        &state,
        admin.id,
        true,
        CreatePostInput {
            slug: "filtered-future".to_owned(),
            title: "Filtered Future".to_owned(),
            markdown: "future".to_owned(),
            html: None,
            excerpt: None,
            status: Some(PostStatus::Scheduled),
            post_type: Some(PostType::Post),
            published_at: Some(Utc::now() + Duration::days(1)),
        },
    )
    .await
    .expect("create scheduled post");

    let draft_page = posts::list(
        &state,
        ListPostQuery {
            q: None,
            status: Some(PostStatus::Draft),
            post_type: Some(PostType::Post),
            term_id: None,
            pagination: Default::default(),
        },
    )
    .await
    .expect("list draft posts");
    let scheduled_page = posts::list(
        &state,
        ListPostQuery {
            q: None,
            status: Some(PostStatus::Scheduled),
            post_type: Some(PostType::Post),
            term_id: None,
            pagination: Default::default(),
        },
    )
    .await
    .expect("list scheduled posts");

    assert_eq!(draft_page.meta.total, 0);
    assert_eq!(scheduled_page.meta.total, 0);
}
