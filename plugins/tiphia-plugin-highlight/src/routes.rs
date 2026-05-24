use axum::{Json, Router, extract::State, routing::get};
use sea_orm::DatabaseConnection;
use tiphia_core::{AppResult, AppState, plugins::load_plugin_config};

use crate::{
    HIGHLIGHT_PLUGIN_NAME,
    config::{HighlightConfig, normalize_style},
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/highlight/config", get(config))
        .route("/highlight/config/", get(config))
}

async fn config(State(state): State<AppState>) -> AppResult<Json<HighlightConfig>> {
    let mut config = load_config(&state.db, HIGHLIGHT_PLUGIN_NAME).await?;
    config.style = normalize_style(&config.style).to_owned();
    Ok(Json(config))
}

async fn load_config(db: &DatabaseConnection, plugin_name: &str) -> AppResult<HighlightConfig> {
    load_plugin_config(db, plugin_name, HighlightConfig::default()).await
}
