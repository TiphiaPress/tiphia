use axum::Json;
use serde_json::Value;
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
        crate::routes::posts::read::list,
        crate::routes::posts::read::admin_list,
        crate::routes::posts::read::admin_show,
        crate::routes::posts::read::popular,
        crate::routes::posts::read::show,
        crate::routes::posts::read::show_by_slug,
        crate::routes::posts::write::create,
        crate::routes::posts::write::update,
        crate::routes::posts::write::delete_post,
        crate::routes::posts::write::change_status,
        crate::routes::posts::write::bulk_action,
        crate::routes::posts::revisions::revisions,
        crate::routes::posts::revisions::restore_revision,
        crate::routes::posts::relations::post_terms,
        crate::routes::posts::relations::sync_post_terms,
        crate::routes::posts::relations::comment_tree,
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

#[path = "openapi/normalize.rs"]
mod normalize;

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
    normalize::normalize_for_swagger_editor(&mut value);
    Json(value)
}
