use axum::{Json, Router, extract::State, routing::get};
use tiphia_core::{AppResult, AppState};

use crate::{
    LINKS_PLUGIN_NAME,
    config::{LinkItem, load_config},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/links", get(list_links))
        .route("/links/", get(list_links))
}

async fn list_links(State(state): State<AppState>) -> AppResult<Json<Vec<LinkItem>>> {
    Ok(Json(load_config(&state.db, LINKS_PLUGIN_NAME).await?.links))
}
