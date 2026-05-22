mod support;

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use sea_orm::EntityTrait;
use serde_json::{Value, json};
use tiphia_core::{Config, build_router_with_plugins, connect_database, entities::comments};
use tower::ServiceExt;

#[tokio::test]
async fn health_returns_json_status() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["status"], "ok");
    assert!(body["version"].as_str().is_some());
    assert!(body["checked_at"].as_str().is_some());
}

#[tokio::test]
async fn openapi_document_is_generated_without_recursing_comment_tree() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/openapi.json")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("openapi response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = response_json(response).await;
    assert_eq!(body["openapi"], "3.0.3");
    assert!(body["components"]["schemas"]["CommentNode"].is_object());
    assert_no_type_arrays(&body);
}

#[tokio::test]
async fn openapi_keeps_frontend_contract_paths() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/openapi.json")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("openapi response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    let paths = body["paths"].as_object().expect("paths object");

    for path in [
        "/api/v1/posts/admin",
        "/api/v1/posts/bulk",
        "/api/v1/posts/popular",
        "/api/v1/posts/{id}/revisions",
        "/api/v1/posts/{id}/revisions/{revision_id}/restore",
        "/api/v1/comments",
        "/api/v1/comments/recent",
        "/api/v1/comments/{id}/moderation",
        "/api/v1/terms",
        "/api/v1/plugins/{name}/state",
        "/api/v1/themes",
        "/api/v1/settings",
    ] {
        assert!(
            paths.contains_key(path),
            "missing frontend contract path {path}"
        );
    }
}

#[tokio::test]
async fn theme_list_is_empty_by_default_because_themes_are_external() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/themes")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("themes response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    let themes = body.as_array().expect("theme array");
    assert!(themes.is_empty());
}

#[tokio::test]
async fn auth_and_post_routes_work_end_to_end() {
    let config = support::config();
    let (app, db) = router_with_db(config).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = response_json(response).await;
    let token = body["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "slug": "http-post",
                "title": "HTTP Post",
                "markdown": "# Hello",
                "status": "published",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create post response");
    let response = assert_status(response, StatusCode::OK).await;

    let body = response_json(response).await;
    assert_eq!(body["slug"], "http-post");
    let post_id = body["id"].as_i64().expect("post id");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts/slug/http-post")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("show post response");
    let response = assert_status(response, StatusCode::OK).await;

    let body = response_json(response).await;
    assert_eq!(body["slug"], "http-post");

    let response = app
        .clone()
        .oneshot(json_request_with_headers(
            "POST",
            "/api/v1/comments",
            json!({
                "post_id": post_id,
                "parent_id": null,
                "author_name": "Alice",
                "author_email": "alice@example.com",
                "author_url": null,
                "content": "Nice post"
            }),
            None,
            &[
                ("x-forwarded-for", "203.0.113.10, 10.0.0.1"),
                (header::USER_AGENT.as_str(), "TiphiaTest/1.0"),
            ],
        ))
        .await
        .expect("create comment response");
    let response = assert_status(response, StatusCode::OK).await;
    let body = response_json(response).await;
    let comment_id = body["id"].as_i64().unwrap() as i32;
    assert!(body.get("ip_hash").is_none());
    assert!(body.get("user_agent").is_none());

    let stored = comments::Entity::find_by_id(comment_id)
        .one(&db)
        .await
        .expect("query comment")
        .expect("comment exists");
    assert_eq!(stored.ip_hash.as_deref().map(str::len), Some(64));
    assert_eq!(stored.user_agent.as_deref(), Some("TiphiaTest/1.0"));

    app.clone()
        .oneshot(json_request(
            "PUT",
            &format!("/api/v1/comments/{comment_id}/moderation"),
            json!({ "status": "approved" }),
            Some(&token),
        ))
        .await
        .expect("approve comment");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts/slug/http-post")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("show post response with stats");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body["comment_count"], 1);
    assert_eq!(body["view_count"], 2);

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts/popular?limit=5")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("popular posts response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body[0]["slug"], "http-post");
    assert_eq!(body[0]["comment_count"], 1);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/comments/recent?limit=5")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("recent comments response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body[0]["post_slug"], "http-post");
    assert_eq!(body[0]["author_name"], "Alice");
    assert!(body[0].get("author_email").is_none());
    assert!(body[0].get("ip_hash").is_none());
    assert!(body[0].get("user_agent").is_none());
}

#[tokio::test]
async fn feeds_and_sitemap_only_include_public_content() {
    let (app, _) = router_with_db(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    app.clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "slug": "feed-public",
                "title": "Feed Public",
                "markdown": "public",
                "status": "published",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create public post");
    app.clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "slug": "feed-draft",
                "title": "Feed Draft",
                "markdown": "draft",
                "status": "draft",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create draft post");

    for path in ["/feed.xml", "/atom.xml", "/sitemap.xml"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(path)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("feed response");
        let response = assert_status(response, StatusCode::OK).await;
        let body = response_text(response).await;
        let public_marker = if path == "/sitemap.xml" {
            "feed-public"
        } else {
            "Feed Public"
        };
        let draft_marker = if path == "/sitemap.xml" {
            "feed-draft"
        } else {
            "Feed Draft"
        };
        assert!(
            body.contains(public_marker),
            "{path} should include public post"
        );
        assert!(
            !body.contains(draft_marker),
            "{path} should not include draft post"
        );
    }

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/robots.txt")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("robots response");
    let response = assert_status(response, StatusCode::OK).await;
    let body = response_text(response).await;
    assert!(body.contains("User-agent: *"));
}

#[tokio::test]
async fn request_body_limit_rejects_large_json() {
    let mut config = support::config();
    config.http.max_body_bytes = 64;
    let app = router(config).await;

    let response = app
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "this body is intentionally too large for the configured limit"
            }),
            None,
        ))
        .await
        .expect("limited response");

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn flat_comment_list_requires_editor_auth() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/comments?status=pending")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("comment list response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn comment_moderation_returns_updated_status() {
    let (app, _) = router_with_db(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let post = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "title": "Moderation Post",
                "markdown": "content",
                "status": "published",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create post");
    let post_id = response_json(assert_status(post, StatusCode::OK).await).await["id"]
        .as_i64()
        .expect("post id");

    let comment = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/comments",
            json!({
                "post_id": post_id,
                "parent_id": null,
                "author_name": "Reader",
                "author_email": "reader@example.com",
                "author_url": null,
                "content": "Pending comment"
            }),
            None,
        ))
        .await
        .expect("create comment");
    let comment_id = response_json(assert_status(comment, StatusCode::OK).await).await["id"]
        .as_i64()
        .expect("comment id");

    let moderated = app
        .oneshot(json_request(
            "PUT",
            &format!("/api/v1/comments/{comment_id}/moderation"),
            json!({ "status": "approved" }),
            Some(&token),
        ))
        .await
        .expect("moderate comment");
    let moderated = assert_status(moderated, StatusCode::OK).await;
    let body = response_json(moderated).await;
    assert_eq!(body["id"], comment_id);
    assert_eq!(body["status"], "approved");
}

#[tokio::test]
async fn post_list_accepts_pagination_query() {
    let app = router(support::config()).await;

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts?page=1&per_page=30")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("post list response");

    let response = assert_status(response, StatusCode::OK).await;
    let body = response_json(response).await;
    assert!(body["data"].is_array());
    assert_eq!(body["meta"]["page"], 1);
    assert_eq!(body["meta"]["per_page"], 30);
}

#[tokio::test]
async fn create_post_can_omit_slug_and_backend_generates_unique_slug() {
    let app = router(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "title": "Generated Slug Post",
                "markdown": "content",
                "status": "draft",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create first post");
    let first = assert_status(first, StatusCode::OK).await;
    assert_eq!(response_json(first).await["slug"], "generated-slug-post");

    let second = app
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "slug": "",
                "title": "Generated Slug Post",
                "markdown": "content",
                "status": "draft",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create second post");
    let second = assert_status(second, StatusCode::OK).await;
    assert_eq!(response_json(second).await["slug"], "generated-slug-post-2");
}

#[tokio::test]
async fn admin_post_list_includes_drafts_while_public_list_hides_them() {
    let app = router(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    app.clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "title": "Admin Visible Draft",
                "markdown": "draft",
                "status": "draft",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create draft");

    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts?page=1&per_page=30")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("public list");
    let public = assert_status(public, StatusCode::OK).await;
    assert_eq!(response_json(public).await["meta"]["total"], 0);

    let admin = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/posts/admin?page=1&per_page=30")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("admin list");
    let admin = assert_status(admin, StatusCode::OK).await;
    let body = response_json(admin).await;
    assert_eq!(body["meta"]["total"], 1);
    assert_eq!(body["data"][0]["title"], "Admin Visible Draft");
}

#[tokio::test]
async fn admin_post_detail_can_load_draft_while_public_detail_hides_it() {
    let app = router(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let created = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "title": "Admin Detail Draft",
                "markdown": "draft",
                "status": "draft",
                "post_type": "post"
            }),
            Some(&token),
        ))
        .await
        .expect("create draft");
    let post_id = response_json(assert_status(created, StatusCode::OK).await).await["id"]
        .as_i64()
        .expect("post id");

    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/posts/{post_id}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("public detail");
    assert_eq!(public.status(), StatusCode::NOT_FOUND);

    let admin = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/posts/admin/{post_id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("admin detail");
    let admin = assert_status(admin, StatusCode::OK).await;
    let body = response_json(admin).await;
    assert_eq!(body["id"], post_id);
    assert_eq!(body["status"], "draft");
}

#[tokio::test]
async fn create_term_can_omit_slug_and_backend_generates_unique_slug() {
    let app = router(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let first = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/terms",
            json!({
                "name": "Generated Term",
                "description": null,
                "term_type": "category",
                "parent_id": null,
                "sort_order": 0
            }),
            Some(&token),
        ))
        .await
        .expect("create first term");
    let first = assert_status(first, StatusCode::OK).await;
    assert_eq!(response_json(first).await["slug"], "generated-term");

    let second = app
        .oneshot(json_request(
            "POST",
            "/api/v1/terms",
            json!({
                "slug": "",
                "name": "Generated Term",
                "description": null,
                "term_type": "category",
                "parent_id": null,
                "sort_order": 0
            }),
            Some(&token),
        ))
        .await
        .expect("create second term");
    let second = assert_status(second, StatusCode::OK).await;
    assert_eq!(response_json(second).await["slug"], "generated-term-2");
}

#[tokio::test]
async fn bulk_post_actions_publish_archive_and_delete() {
    let app = router(support::config()).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/bootstrap",
            json!({
                "username": "admin",
                "email": "admin@example.com",
                "password": "long-enough-password",
                "display_name": "Admin"
            }),
            None,
        ))
        .await
        .expect("bootstrap response");
    let token = response_json(response).await["access_token"]
        .as_str()
        .expect("access token")
        .to_owned();

    let first = create_draft_post(&app, &token, "bulk-one").await;
    let second = create_draft_post(&app, &token, "bulk-two").await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/posts/bulk",
            json!({
                "ids": [first, second],
                "action": "publish",
                "published_at": null
            }),
            Some(&token),
        ))
        .await
        .expect("bulk publish response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body["affected"], 2);
    assert_eq!(body["posts"][0]["status"], "published");

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/posts/bulk",
            json!({
                "ids": [first],
                "action": "archive",
                "published_at": null
            }),
            Some(&token),
        ))
        .await
        .expect("bulk archive response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body["posts"][0]["status"], "archived");

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/posts/bulk",
            json!({
                "ids": [second],
                "action": "delete",
                "published_at": null
            }),
            Some(&token),
        ))
        .await
        .expect("bulk delete response");
    let body = response_json(assert_status(response, StatusCode::OK).await).await;
    assert_eq!(body["affected"], 1);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/posts/admin/{second}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("deleted post detail response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

async fn router(config: Config) -> axum::Router {
    router_with_db(config).await.0
}

async fn create_draft_post(app: &axum::Router, token: &str, slug: &str) -> i64 {
    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/posts",
            json!({
                "slug": slug,
                "title": slug,
                "markdown": "content",
                "status": "draft",
                "post_type": "post"
            }),
            Some(token),
        ))
        .await
        .expect("create draft post");
    response_json(assert_status(response, StatusCode::OK).await).await["id"]
        .as_i64()
        .expect("post id")
}

async fn router_with_db(config: Config) -> (axum::Router, sea_orm::DatabaseConnection) {
    let db = connect_database(&config.database)
        .await
        .expect("connect database");
    let app = build_router_with_plugins(db.clone(), config, |_| Ok(()))
        .await
        .expect("router");
    (app, db)
}

fn json_request(method: &str, uri: &str, body: Value, token: Option<&str>) -> Request<Body> {
    json_request_with_headers(method, uri, body, token, &[])
}

fn json_request_with_headers(
    method: &str,
    uri: &str,
    body: Value,
    token: Option<&str>,
    headers: &[(&str, &str)],
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json");

    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }

    builder
        .body(Body::from(body.to_string()))
        .expect("json request")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("json response")
}

async fn response_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    String::from_utf8(bytes.to_vec()).expect("utf8 response")
}

async fn assert_status(
    response: axum::response::Response,
    expected: StatusCode,
) -> axum::response::Response {
    let actual = response.status();
    if actual != expected {
        let body = to_bytes(response.into_body(), 1024 * 1024)
            .await
            .expect("response body");
        panic!(
            "expected status {expected}, got {actual}; body={}",
            String::from_utf8_lossy(&body)
        );
    }

    response
}

fn assert_no_type_arrays(value: &Value) {
    match value {
        Value::Object(object) => {
            if let Some(schema_type) = object.get("type") {
                assert!(
                    schema_type.is_string(),
                    "OpenAPI 3.0 does not support array-valued schema type: {object:?}"
                );
            }
            for value in object.values() {
                assert_no_type_arrays(value);
            }
        }
        Value::Array(items) => {
            for value in items {
                assert_no_type_arrays(value);
            }
        }
        _ => {}
    }
}
