use crate::{
    config::Config,
    error::AppResult,
    plugins::{Hook, HookContext, PluginRegistry, PluginRegistryBuilder},
    rate_limit::RateLimiter,
    routes,
};
use axum::{Router, extract::DefaultBodyLimit, http::StatusCode};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

#[path = "app/cors.rs"]
mod cors;
#[path = "app/security.rs"]
mod security;
#[path = "app/state.rs"]
mod state;

pub use state::AppState;

pub async fn build_router(db: DatabaseConnection) -> AppResult<Router> {
    build_router_with_plugins(db, Config::load()?, |_| Ok(())).await
}

pub async fn build_router_with_plugins<F>(
    db: DatabaseConnection,
    config: Config,
    register_plugins: F,
) -> AppResult<Router>
where
    F: FnOnce(&mut PluginRegistryBuilder) -> AppResult<()>,
{
    let plugins = Arc::new(PluginRegistry::boot(db.clone(), register_plugins).await?);
    let rate_limiter = RateLimiter::new(&config.rate_limit).await?;
    let state = AppState {
        db,
        plugins: plugins.clone(),
        config: Arc::new(config.clone()),
        rate_limiter,
    };

    dispatch_boot_hook(&plugins, Hook::AppBooting).await?;

    let router = plugins.mount_routes(routes::router());
    let router = apply_base_layers(router, &config);
    let router = router.with_state(state);

    dispatch_boot_hook(&plugins, Hook::AppBooted).await?;

    Ok(router)
}

fn apply_base_layers(router: Router<AppState>, config: &Config) -> Router<AppState> {
    let router = router
        .layer(DefaultBodyLimit::max(config.http.max_body_bytes))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            config.http.request_timeout,
        ))
        .layer(CompressionLayer::new())
        .layer(cors::cors_layer(config))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    security::apply_security_headers(router)
}

async fn dispatch_boot_hook(plugins: &PluginRegistry, hook: Hook) -> AppResult<()> {
    let mut context = HookContext::default();
    plugins.dispatch(hook, &mut context).await
}
