use crate::{app::AppState, entities::posts::PostType};
use axum::Router;

#[path = "posts/read.rs"]
pub mod read;
#[path = "posts/relations.rs"]
pub mod relations;
#[path = "posts/revisions.rs"]
pub mod revisions;
#[path = "posts/router.rs"]
mod router;
#[path = "posts/write.rs"]
pub mod write;

pub use read::{admin_list, admin_show, list, popular, show, show_by_slug};
pub use relations::{comment_tree, post_terms, sync_post_terms};
pub use revisions::{restore_revision, revisions};
pub use write::{bulk_action, change_status, create, delete_post, update};

pub fn post_routes() -> Router<AppState> {
    router::resource_routes(PostType::Post)
}

pub fn page_routes() -> Router<AppState> {
    router::resource_routes(PostType::Page)
}
