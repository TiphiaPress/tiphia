use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tiphia_core::{
    AppResult, AppState,
    entities::users,
    error::AppError,
    plugins::load_plugin_config,
    services::{auth::hash_password, options},
};
use utoipa::ToSchema;

use crate::{
    COMMENT_MAIL_PUSH_PLUGIN_NAME,
    config::CommentMailPushConfig,
    mailer::{escape_html, send_mail},
    password_reset::{
        PasswordResetRecord, append_token, expires_at, generate_token, hash_token, reset_option_key,
    },
    schema::PublicConfigResponse,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/comment-mail-push/config", get(public_config))
        .route(
            "/comment-mail-push/password/forgot",
            post(request_password_reset),
        )
        .route("/comment-mail-push/password/reset", post(reset_password))
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct ForgotPasswordInput {
    pub account: String,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ForgotPasswordResponse {
    pub accepted: bool,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct ResetPasswordInput {
    pub token: String,
    pub password: String,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ResetPasswordResponse {
    pub reset: bool,
}

pub async fn public_config(State(state): State<AppState>) -> AppResult<Json<PublicConfigResponse>> {
    let config = load_config(&state).await?;
    Ok(Json(PublicConfigResponse {
        enabled: config.enabled,
        comment_push_enabled: config.enabled && config.comment_push_enabled,
        password_reset_enabled: config.password_reset_ready(),
    }))
}

pub async fn request_password_reset(
    State(state): State<AppState>,
    Json(input): Json<ForgotPasswordInput>,
) -> AppResult<Json<ForgotPasswordResponse>> {
    let config = load_config(&state).await?;
    if !config.password_reset_ready() {
        return Ok(Json(ForgotPasswordResponse { accepted: false }));
    }

    let account = input.account.trim().to_owned();
    if account.is_empty() {
        return Ok(Json(ForgotPasswordResponse { accepted: true }));
    }

    let Some(user) = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(account.clone())
                .or(users::Column::Email.eq(account)),
        )
        .one(&state.db)
        .await?
    else {
        // 不暴露账号是否存在，避免枚举用户。
        return Ok(Json(ForgotPasswordResponse { accepted: true }));
    };

    let token = generate_token();
    let token_hash = hash_token(&token);
    let record = PasswordResetRecord {
        user_id: user.id,
        expires_at: expires_at(config.reset_token_ttl_minutes),
    };
    options::upsert_json(
        &state,
        &reset_option_key(&token_hash),
        serde_json::to_value(record).map_err(|err| AppError::Plugin(err.to_string()))?,
        false,
    )
    .await?;

    let reset_url = append_token(&config.recovery_base_url, &token);
    let html = format!(
        "<h2>找回密码</h2><p>你好，{}：</p><p>请点击下面的链接重置密码。该链接将在 {} 分钟后过期。</p><p><a href=\"{}\">重置密码</a></p><p>如果不是你本人操作，可以忽略这封邮件。</p>",
        escape_html(&user.display_name),
        config.reset_token_ttl_minutes.clamp(1, 1440),
        escape_html(&reset_url),
    );
    send_mail(&config, &user.email, "找回密码", &html).await?;

    Ok(Json(ForgotPasswordResponse { accepted: true }))
}

pub async fn reset_password(
    State(state): State<AppState>,
    Json(input): Json<ResetPasswordInput>,
) -> AppResult<Json<ResetPasswordResponse>> {
    let config = load_config(&state).await?;
    if !config.password_reset_ready() {
        return Err(AppError::Forbidden);
    }
    if input.password.len() < 8 {
        return Err(AppError::Validation(
            "password must be at least 8 characters".to_owned(),
        ));
    }

    let token_hash = hash_token(input.token.trim());
    let key = reset_option_key(&token_hash);
    let Some(value) = options::get_json(&state, &key).await? else {
        return Err(AppError::Unauthorized);
    };
    let record: PasswordResetRecord = serde_json::from_value(value)
        .map_err(|err| AppError::Plugin(format!("invalid password reset record: {err}")))?;
    if record.expires_at < Utc::now() {
        options::upsert_json(&state, &key, Value::Null, false).await?;
        return Err(AppError::Unauthorized);
    }

    let user = users::Entity::find_by_id(record.user_id)
        .one(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;
    let mut model: users::ActiveModel = user.into();
    model.password_hash = Set(hash_password(&input.password)?);
    model.updated_at = Set(Utc::now());
    model.update(&state.db).await?;
    options::upsert_json(&state, &key, Value::Null, false).await?;

    Ok(Json(ResetPasswordResponse { reset: true }))
}

async fn load_config(state: &AppState) -> AppResult<CommentMailPushConfig> {
    load_plugin_config(
        &state.db,
        COMMENT_MAIL_PUSH_PLUGIN_NAME,
        CommentMailPushConfig::default(),
    )
    .await
}
