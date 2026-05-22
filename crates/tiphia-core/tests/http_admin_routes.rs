mod support;

use async_trait::async_trait;
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
    Json, Router,
    routing::get,
};
use serde_json::{Value, json};
use tiphia_core::{
    AppResult, Config, build_router_with_plugins, connect_database,
    plugins::PluginRegistryBuilder,
    plugins::{
        AdminMenuItem, Plugin, PluginConfigField, PluginConfigFieldType, PluginConfigSchema,
        PluginManifest,
    },
    AppState,
};
use tower::ServiceExt;

static MANIFEST: PluginManifest = PluginManifest {
    name: "test-admin-plugin",
    version: "0.1.0",
    description: "Admin route test plugin.",
    author: "Tiphia Tests",
};

struct AdminTestPlugin;

#[async_trait]
impl Plugin for AdminTestPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    fn admin_menu(&self) -> Vec<AdminMenuItem> {
        vec![AdminMenuItem {
            label: "Test Plugin",
            path: "/admin/test-plugin",
            icon: Some("plug"),
            order: 10,
        }]
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![PluginConfigField {
                key: "enabled",
                label: "Enabled",
                field_type: PluginConfigFieldType::Boolean,
                required: true,
                default: Some(json!(true)),
                help: None,
            }],
        })
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1/test-admin-plugin")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(Router::new().route("/status", get(test_plugin_status)))
    }
}

async fn test_plugin_status() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

#[tokio::test]
async fn admin_can_manage_users_settings_terms_and_plugins() {
    let app = router_with_plugins(support::config(), |plugins| {
        plugins.register(AdminTestPlugin);
        Ok(())
    })
    .await;
    let token = bootstrap_admin(&app).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            json!({
                "username": "editor",
                "email": "editor@example.com",
                "password": "long-enough-password",
                "display_name": "Editor",
                "role": "editor"
            }),
            Some(&token),
        ))
        .await
        .expect("create user response");
    let response = assert_status(response, StatusCode::CREATED).await;
    let editor = response_json(response).await;
    assert_eq!(editor["role"], "editor");

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/users",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("list users response");
    let response = assert_status(response, StatusCode::OK).await;
    let users = response_json(response).await;
    assert_eq!(users["meta"]["total"], 2);

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/settings",
            json!({
                "title": "Tiphia Test",
                "description": "Testing settings",
                "base_url": "https://example.com",
                "timezone": "UTC",
                "default_page_size": 25,
                "comments_enabled": true,
                "comment_moderation": true,
                "permalink_format": "/posts/{slug}",
                "seo": {
                    "meta_title_suffix": "Tiphia",
                    "meta_description": "A test site"
                }
            }),
            Some(&token),
        ))
        .await
        .expect("update settings response");
    let response = assert_status(response, StatusCode::OK).await;
    let settings = response_json(response).await;
    assert_eq!(settings["title"], "Tiphia Test");

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/terms",
            json!({
                "slug": "rust",
                "name": "Rust",
                "description": "Rust posts",
                "term_type": "category",
                "parent_id": null,
                "sort_order": 1
            }),
            Some(&token),
        ))
        .await
        .expect("create term response");
    let response = assert_status(response, StatusCode::OK).await;
    let term = response_json(response).await;
    assert_eq!(term["slug"], "rust");

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/plugins/admin-menu",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("admin menu response");
    let response = assert_status(response, StatusCode::OK).await;
    let menu = response_json(response).await;
    assert_eq!(menu.as_array().expect("menu").len(), 0);

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/plugins/test-admin-plugin/state",
            json!({ "enabled": true }),
            Some(&token),
        ))
        .await
        .expect("plugin state response");
    let response = assert_status(response, StatusCode::OK).await;
    let state = response_json(response).await;
    assert_eq!(state["enabled"], true);

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/plugins/admin-menu",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("admin menu response");
    let response = assert_status(response, StatusCode::OK).await;
    let menu = response_json(response).await;
    assert_eq!(menu[0]["label"], "Test Plugin");

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/plugins/test-admin-plugin/config",
            json!({
                "config": {
                    "enabled": true
                }
            }),
            Some(&token),
        ))
        .await
        .expect("plugin config response");
    let response = assert_status(response, StatusCode::OK).await;
    let plugin_config = response_json(response).await;
    assert_eq!(plugin_config["config"]["enabled"], true);
}

#[tokio::test]
async fn disabled_plugin_routes_and_menu_are_hidden() {
    let app = router_with_plugins(support::config(), |plugins| {
        plugins.register(AdminTestPlugin);
        Ok(())
    })
    .await;
    let token = bootstrap_admin(&app).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/test-admin-plugin/status",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("default disabled plugin route response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/plugins/test-admin-plugin/state",
            json!({ "enabled": true }),
            Some(&token),
        ))
        .await
        .expect("plugin state response");
    let response = assert_status(response, StatusCode::OK).await;
    let body = response_json(response).await;
    assert_eq!(body["enabled"], true);

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/test-admin-plugin/status",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("enabled plugin route response");
    assert_status(response, StatusCode::OK).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            "/api/v1/plugins/test-admin-plugin/state",
            json!({ "enabled": false }),
            Some(&token),
        ))
        .await
        .expect("plugin state response");
    let response = assert_status(response, StatusCode::OK).await;
    let body = response_json(response).await;
    assert_eq!(body["enabled"], false);

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/test-admin-plugin/status",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("disabled plugin route response");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/plugins/admin-menu",
            json!(null),
            Some(&token),
        ))
        .await
        .expect("admin menu response");
    let response = assert_status(response, StatusCode::OK).await;
    let menu = response_json(response).await;
    assert_eq!(menu.as_array().expect("menu").len(), 0);
}

#[tokio::test]
async fn author_cannot_access_admin_user_routes() {
    let app = router_with_plugins(support::config(), |_| Ok(())).await;
    let admin_token = bootstrap_admin(&app).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            json!({
                "username": "author",
                "email": "author@example.com",
                "password": "long-enough-password",
                "display_name": "Author",
                "role": "author"
            }),
            Some(&admin_token),
        ))
        .await
        .expect("create author response");
    assert_status(response, StatusCode::CREATED).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            json!({
                "account": "author@example.com",
                "password": "long-enough-password"
            }),
            None,
        ))
        .await
        .expect("author login response");
    let response = assert_status(response, StatusCode::OK).await;
    let author_token = response_json(response).await["access_token"]
        .as_str()
        .expect("author token")
        .to_owned();

    let response = app
        .oneshot(json_request(
            "GET",
            "/api/v1/users",
            json!(null),
            Some(&author_token),
        ))
        .await
        .expect("list users response");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn root_and_admin_user_management_rules_are_enforced() {
    let app = router_with_plugins(support::config(), |_| Ok(())).await;
    let root_token = bootstrap_admin(&app).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            json!({
                "username": "admin2",
                "email": "admin2@example.com",
                "password": "long-enough-password",
                "display_name": "Admin Two",
                "role": "admin"
            }),
            Some(&root_token),
        ))
        .await
        .expect("create admin response");
    let response = assert_status(response, StatusCode::CREATED).await;
    let admin = response_json(response).await;
    let admin_id = admin["id"].as_i64().expect("admin id");

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/auth/login",
            json!({
                "account": "admin2@example.com",
                "password": "long-enough-password"
            }),
            None,
        ))
        .await
        .expect("admin login response");
    let response = assert_status(response, StatusCode::OK).await;
    let admin_token = response_json(response).await["access_token"]
        .as_str()
        .expect("admin token")
        .to_owned();

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            json!({
                "username": "admin3",
                "email": "admin3@example.com",
                "password": "long-enough-password",
                "display_name": "Admin Three",
                "role": "admin"
            }),
            Some(&admin_token),
        ))
        .await
        .expect("admin creates peer response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            json!({
                "username": "editor2",
                "email": "editor2@example.com",
                "password": "long-enough-password",
                "display_name": "Editor Two",
                "role": "editor"
            }),
            Some(&admin_token),
        ))
        .await
        .expect("admin creates editor response");
    assert_status(response, StatusCode::CREATED).await;

    let response = app
        .clone()
        .oneshot(json_request(
            "PUT",
            &format!("/api/v1/users/{admin_id}"),
            json!({ "status": "disabled" }),
            Some(&admin_token),
        ))
        .await
        .expect("admin disables self response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = app
        .clone()
        .oneshot(json_request(
            "GET",
            "/api/v1/users",
            json!(null),
            Some(&admin_token),
        ))
        .await
        .expect("list users response");
    let response = assert_status(response, StatusCode::OK).await;
    let users = response_json(response).await;
    let root_id = users["data"]
        .as_array()
        .expect("users array")
        .iter()
        .find(|user| user["role"] == "root")
        .and_then(|user| user["id"].as_i64())
        .expect("root id");

    let response = app
        .oneshot(json_request(
            "PUT",
            &format!("/api/v1/users/{root_id}"),
            json!({ "display_name": "Not Allowed" }),
            Some(&admin_token),
        ))
        .await
        .expect("admin updates root response");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn duplicate_user_email_returns_validation_error_response() {
    let app = router_with_plugins(support::config(), |_| Ok(())).await;
    let token = bootstrap_admin(&app).await;

    let payload = json!({
        "username": "editor",
        "email": "editor@example.com",
        "password": "long-enough-password",
        "display_name": "Editor",
        "role": "editor"
    });

    let response = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/api/v1/users",
            payload.clone(),
            Some(&token),
        ))
        .await
        .expect("create user response");
    assert_status(response, StatusCode::CREATED).await;

    let response = app
        .oneshot(json_request("POST", "/api/v1/users", payload, Some(&token)))
        .await
        .expect("duplicate user response");
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = response_json(response).await;
    assert_eq!(body["error"]["code"], "validation_error");
}

async fn bootstrap_admin(app: &axum::Router) -> String {
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
    let response = assert_status(response, StatusCode::CREATED).await;
    let body = response_json(response).await;
    body["access_token"].as_str().expect("token").to_owned()
}

async fn router_with_plugins<F>(config: Config, register_plugins: F) -> axum::Router
where
    F: FnOnce(&mut PluginRegistryBuilder) -> AppResult<()>,
{
    let db = connect_database(&config.database)
        .await
        .expect("connect database");
    build_router_with_plugins(db, config, register_plugins)
        .await
        .expect("router")
}

fn json_request(method: &str, uri: &str, body: Value, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);

    if !body.is_null() {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }

    let body = if body.is_null() {
        Body::empty()
    } else {
        Body::from(body.to_string())
    };

    builder.body(body).expect("json request")
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("response body");
    serde_json::from_slice(&bytes).expect("json response")
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
