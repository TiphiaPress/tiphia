use crate::{app::AppState, entities::posts::PostType, routes::auth::CurrentUser};
use axum::{
    Router,
    routing::{get, put},
};

use super::read::{admin_list, admin_show, list, popular, show, show_by_slug};
use super::{
    bulk_action, change_status, comment_tree, create, delete_post, post_terms, restore_revision,
    revisions, sync_post_terms, update,
};

pub fn resource_routes(post_type: PostType) -> Router<AppState> {
    let list_post_type = post_type.clone();
    let admin_list_post_type = post_type.clone();
    let slug_post_type = post_type;

    Router::new()
        .route(
            "/",
            get(move |state, query| list(state, query, list_post_type.clone())).post(create),
        )
        .route(
            "/admin",
            get(move |state, current_user: CurrentUser, query| {
                admin_list(state, current_user, query, admin_list_post_type.clone())
            }),
        )
        .route("/popular", get(popular))
        .route("/bulk", put(bulk_action))
        .route("/admin/{id}", get(admin_show))
        .route(
            "/slug/{slug}",
            get(move |state, path| show_by_slug(state, path, slug_post_type.clone())),
        )
        .route("/{id}", get(show).put(update).delete(delete_post))
        .route("/{id}/comments/tree", get(comment_tree))
        .route("/{id}/revisions", get(revisions))
        .route(
            "/{id}/revisions/{revision_id}/restore",
            put(restore_revision),
        )
        .route("/{id}/status", put(change_status))
        .route("/{id}/terms", get(post_terms).put(sync_post_terms))
}
