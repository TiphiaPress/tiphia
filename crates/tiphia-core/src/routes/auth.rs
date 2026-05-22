use crate::{
    app::AppState,
    error::{AppError, AppResult},
    services::auth::{
        AuthStatus, BootstrapAdminInput, LoginInput, PublicUser, RegisterInput, TokenResponse,
    },
};
use axum::{
    Json, Router,
    extract::{FromRequestParts, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    routing::{get, post},
};

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/status", get(status))
        .route("/bootstrap", post(bootstrap_admin))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(me))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/status",
    tag = "auth",
    responses((status = 200, description = "Authentication status", body = AuthStatus))
)]
pub async fn status(State(state): State<AppState>) -> AppResult<Json<AuthStatus>> {
    Ok(Json(crate::services::auth::status(&state).await?))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    tag = "auth",
    request_body = RegisterInput,
    responses((status = 201, description = "Registered", body = TokenResponse), (status = 403, description = "Registration disabled"))
)]
pub async fn register(
    State(state): State<AppState>,
    Json(input): Json<RegisterInput>,
) -> AppResult<(StatusCode, Json<TokenResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(crate::services::auth::register(&state, input).await?),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/bootstrap",
    tag = "auth",
    request_body = BootstrapAdminInput,
    responses((status = 201, description = "Created", body = TokenResponse))
)]
pub async fn bootstrap_admin(
    State(state): State<AppState>,
    Json(input): Json<BootstrapAdminInput>,
) -> AppResult<(StatusCode, Json<TokenResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(crate::services::auth::bootstrap_admin(&state, input).await?),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    tag = "auth",
    request_body = LoginInput,
    responses((status = 200, description = "Token", body = TokenResponse), (status = 429, description = "Rate limited"))
)]
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(input): Json<LoginInput>,
) -> AppResult<Json<TokenResponse>> {
    crate::rate_limit::check_login(&state, &headers).await?;
    Ok(Json(crate::services::auth::login(&state, input).await?))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    tag = "auth",
    security(("bearerAuth" = [])),
    responses((status = 200, description = "Current user", body = PublicUser))
)]
pub async fn me(current_user: CurrentUser) -> AppResult<Json<PublicUser>> {
    Ok(Json(current_user.0))
}

pub struct BearerToken(pub String);

impl<S> FromRequestParts<S> for BearerToken
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header_value = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = header_value
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?
            .trim();

        if token.is_empty() {
            return Err(AppError::Unauthorized);
        }

        Ok(Self(token.to_owned()))
    }
}

pub struct CurrentUser(pub PublicUser);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let bearer = BearerToken::from_request_parts(parts, state).await?;
        let user = crate::services::auth::current_user(state, &bearer.0).await?;
        Ok(Self(user))
    }
}
