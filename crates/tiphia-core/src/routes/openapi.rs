use axum::Json;
use serde_json::{Map, Value, json};
use utoipa::{
    OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::pagination::PaginationQuery;
use crate::services::{
    auth::{AuthStatus, BootstrapAdminInput, LoginInput, PublicUser, RegisterInput, TokenResponse},
    comments::{
        CommentNode, CreateCommentInput, ModerateCommentInput, RecentCommentQuery,
        RecentCommentResponse,
    },
    plugins::{PluginStateResponse, UpdatePluginStateInput},
    posts::{
        BulkPostActionInput, BulkPostActionResponse, ChangePostStatusInput, CreatePostInput,
        PopularPostQuery, PostResponse, UpdatePostInput,
    },
    settings::{SeoSettings, SiteSettings},
    terms::{CreateTermInput, SyncPostTermsInput, TermResponse, UpdateTermInput},
    themes::ThemeInfo,
    users::{ChangePasswordInput, CreateUserInput, UpdateUserInput},
};
use crate::{error::ErrorBody, routes::HealthResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::auth::bootstrap_admin,
        crate::routes::auth::status,
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::me,
        crate::routes::posts::list,
        crate::routes::posts::admin_list,
        crate::routes::posts::admin_show,
        crate::routes::posts::popular,
        crate::routes::posts::show,
        crate::routes::posts::show_by_slug,
        crate::routes::posts::create,
        crate::routes::posts::update,
        crate::routes::posts::delete_post,
        crate::routes::posts::change_status,
        crate::routes::posts::bulk_action,
        crate::routes::posts::revisions,
        crate::routes::posts::restore_revision,
        crate::routes::posts::post_terms,
        crate::routes::posts::sync_post_terms,
        crate::routes::posts::comment_tree,
        crate::routes::comments::list,
        crate::routes::comments::recent,
        crate::routes::comments::create,
        crate::routes::comments::tree_for_post,
        crate::routes::comments::moderate,
        crate::routes::terms::list,
        crate::routes::terms::show,
        crate::routes::terms::create,
        crate::routes::terms::update,
        crate::routes::terms::delete_term,
        crate::routes::settings::get_settings,
        crate::routes::settings::update_settings,
        crate::routes::users::list,
        crate::routes::users::create,
        crate::routes::users::show,
        crate::routes::users::update,
        crate::routes::users::change_password,
        crate::routes::plugins::list,
        crate::routes::plugins::admin_menu,
        crate::routes::plugins::get_config,
        crate::routes::plugins::update_config,
        crate::routes::plugins::get_state,
        crate::routes::plugins::update_state,
        crate::routes::themes::list,
        crate::routes::health
    ),
    components(schemas(
        BootstrapAdminInput,
        AuthStatus,
        LoginInput,
        RegisterInput,
        TokenResponse,
        PublicUser,
        CreatePostInput,
        UpdatePostInput,
        ChangePostStatusInput,
        BulkPostActionInput,
        BulkPostActionResponse,
        PopularPostQuery,
        PostResponse,
        CreateCommentInput,
        ModerateCommentInput,
        RecentCommentQuery,
        RecentCommentResponse,
        CommentNode,
        CreateTermInput,
        UpdateTermInput,
        SyncPostTermsInput,
        TermResponse,
        SiteSettings,
        SeoSettings,
        CreateUserInput,
        UpdateUserInput,
        ChangePasswordInput,
        PluginStateResponse,
        UpdatePluginStateInput,
        ThemeInfo,
        ErrorBody,
        HealthResponse,
        PaginationQuery
    )),
    tags(
        (name = "auth", description = "Authentication"),
        (name = "content", description = "Posts and pages"),
        (name = "comments", description = "Comments"),
        (name = "terms", description = "Categories and tags"),
        (name = "settings", description = "Site settings"),
        (name = "users", description = "Users"),
        (name = "plugins", description = "Plugins"),
        (name = "themes", description = "Themes"),
        (name = "system", description = "System")
    )
)]
pub struct ApiDoc;

pub async fn openapi() -> Json<Value> {
    let mut openapi = ApiDoc::openapi();
    openapi
        .components
        .as_mut()
        .expect("components")
        .add_security_scheme(
            "bearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );

    let mut value = serde_json::to_value(openapi).expect("openapi document should serialize");
    normalize_for_swagger_editor(&mut value);
    Json(value)
}

fn normalize_for_swagger_editor(value: &mut Value) {
    value["openapi"] = Value::String("3.0.3".to_owned());
    assign_unique_operation_ids(value);
    normalize_schema(value);
}

fn assign_unique_operation_ids(value: &mut Value) {
    let Some(paths) = value.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };

    for (path, item) in paths {
        let Some(methods) = item.as_object_mut() else {
            continue;
        };

        for (method, operation) in methods {
            if !matches!(
                method.as_str(),
                "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "trace"
            ) {
                continue;
            }

            let operation_id = format!(
                "{}_{}",
                method,
                path.trim_start_matches('/')
                    .replace(['/', '-', '{', '}'], "_")
                    .trim_matches('_')
            );
            operation["operationId"] = Value::String(operation_id);
        }
    }
}

fn normalize_schema(value: &mut Value) {
    match value {
        Value::Object(object) => {
            normalize_type_array(object);
            normalize_one_of_null(object);

            for value in object.values_mut() {
                normalize_schema(value);
            }
        }
        Value::Array(items) => {
            for value in items {
                normalize_schema(value);
            }
        }
        _ => {}
    }
}

fn normalize_type_array(object: &mut Map<String, Value>) {
    let Some(types) = object.get("type").and_then(Value::as_array) else {
        return;
    };
    let non_null_types = types
        .iter()
        .filter_map(Value::as_str)
        .filter(|schema_type| *schema_type != "null")
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let has_null = types
        .iter()
        .filter_map(Value::as_str)
        .any(|schema_type| schema_type == "null");

    if has_null {
        object.insert("nullable".to_owned(), Value::Bool(true));
    }

    match non_null_types.as_slice() {
        [schema_type] => {
            object.insert("type".to_owned(), Value::String(schema_type.clone()));
        }
        [] => {
            object.remove("type");
        }
        _ => {
            object.insert(
                "oneOf".to_owned(),
                Value::Array(
                    non_null_types
                        .into_iter()
                        .map(|schema_type| json!({ "type": schema_type }))
                        .collect(),
                ),
            );
            object.remove("type");
        }
    }
}

fn normalize_one_of_null(object: &mut Map<String, Value>) {
    let Some(Value::Array(mut one_of)) = object.remove("oneOf") else {
        return;
    };

    let original_len = one_of.len();
    one_of.retain(|schema| !is_null_schema(schema));
    if one_of.len() == original_len {
        object.insert("oneOf".to_owned(), Value::Array(one_of));
        return;
    }

    object.insert("nullable".to_owned(), Value::Bool(true));
    if one_of.len() == 1 {
        if let Some(schema) = one_of.pop() {
            object.remove("oneOf");
            if let Some(schema) = schema.as_object() {
                for (key, value) in schema {
                    object.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
        }
    } else if !one_of.is_empty() {
        object.insert("oneOf".to_owned(), Value::Array(one_of));
    }
}

fn is_null_schema(value: &Value) -> bool {
    value
        .get("type")
        .and_then(Value::as_str)
        .map(|schema_type| schema_type == "null")
        .unwrap_or(false)
}
