use crate::{
    config::Config,
    error::AppResult,
    plugins::{Hook, HookContext, PluginRegistry, PluginRegistryBuilder},
    rate_limit::RateLimiter,
    routes,
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{HeaderValue, Method, StatusCode, header},
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tower_http::{
    compression::CompressionLayer,
    cors::{AllowOrigin, CorsLayer},
    set_header::SetResponseHeaderLayer,
    timeout::TimeoutLayer,
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};
use tracing::Level;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub plugins: Arc<PluginRegistry>,
    pub config: Arc<Config>,
    pub rate_limiter: RateLimiter,
}

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

    let mut boot_context = HookContext::default();
    plugins
        .dispatch(Hook::AppBooting, &mut boot_context)
        .await?;

    let router = plugins.mount_routes(routes::router());
    let router = router
        .layer(DefaultBodyLimit::max(config.http.max_body_bytes))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            config.http.request_timeout,
        ))
        .layer(CompressionLayer::new())
        .layer(cors_layer(&config))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::X_FRAME_OPTIONS,
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::REFERRER_POLICY,
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::overriding(
            header::CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self' 'unsafe-inline' https:; connect-src 'self' https:; frame-src https:; style-src 'self' 'unsafe-inline' https:; img-src 'self' http: https: data: blob:; frame-ancestors 'none'",
            ),
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    let router = router.with_state(state);

    let mut booted_context = HookContext::default();
    plugins
        .dispatch(Hook::AppBooted, &mut booted_context)
        .await?;

    Ok(router)
}

fn cors_layer(config: &Config) -> CorsLayer {
    if config.cors.allowed_origins.is_empty() {
        return CorsLayer::permissive();
    }

    let origins = config
        .cors
        .allowed_origins
        .iter()
        .filter_map(|origin| origin.parse::<HeaderValue>().ok())
        .collect::<Vec<_>>();

    CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
}
