use crate::app::AppState;
use axum::{
    Router,
    http::{HeaderValue, header},
};
use tower_http::set_header::SetResponseHeaderLayer;

pub fn apply_security_headers(router: Router<AppState>) -> Router<AppState> {
    router
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
}
