use crate::{config::Config, plugins::PluginRegistry, rate_limit::RateLimiter};
use sea_orm::DatabaseConnection;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub plugins: Arc<PluginRegistry>,
    pub config: Arc<Config>,
    pub rate_limiter: RateLimiter,
}
