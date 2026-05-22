use crate::{
    app::AppState,
    config::AuthConfig,
    entities::users::{self, UserRole, UserStatus},
    error::{AppError, AppResult, validation_on_unique},
};
use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct AuthStatus {
    pub initialized: bool,
    pub registration_enabled: bool,
}

#[derive(Clone, Debug, Deserialize, ToSchema)]
pub struct BootstrapAdminInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct RegisterInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub captcha: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct LoginInput {
    pub account: String,
    pub password: String,
    #[serde(default)]
    pub captcha: Option<Value>,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: &'static str,
    pub expires_at: i64,
    pub user: PublicUser,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct PublicUser {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub display_name: String,
    pub role: UserRole,
    pub status: UserStatus,
}

impl PublicUser {
    pub fn is_admin(&self) -> bool {
        matches!(self.role, UserRole::Admin | UserRole::Root)
    }

    pub fn is_root(&self) -> bool {
        matches!(self.role, UserRole::Root)
    }

    pub fn can_edit_all_content(&self) -> bool {
        matches!(
            self.role,
            UserRole::Root | UserRole::Admin | UserRole::Editor
        )
    }

    pub fn require_admin(&self) -> AppResult<()> {
        if self.is_admin() {
            return Ok(());
        }

        Err(AppError::Forbidden)
    }

    pub fn require_root(&self) -> AppResult<()> {
        if self.is_root() {
            return Ok(());
        }

        Err(AppError::Forbidden)
    }

    pub fn require_editor(&self) -> AppResult<()> {
        if self.can_edit_all_content() {
            return Ok(());
        }

        Err(AppError::Forbidden)
    }

    pub fn require_content_owner_or_editor(&self, owner_id: i32) -> AppResult<()> {
        if self.can_edit_all_content() || self.id == owner_id {
            return Ok(());
        }

        Err(AppError::Forbidden)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct Claims {
    pub sub: i32,
    pub username: String,
    pub role: UserRole,
    pub exp: usize,
    pub iat: usize,
}

pub async fn bootstrap_admin(
    state: &AppState,
    input: BootstrapAdminInput,
) -> AppResult<TokenResponse> {
    validate_required(&input.username, "username")?;
    validate_required(&input.email, "email")?;
    validate_password(&input.password)?;

    let user_count = users::Entity::find().count(&state.db).await?;
    if user_count > 0 {
        return Err(AppError::Forbidden);
    }

    let now = Utc::now();
    let password_hash = hash_password(&input.password)?;
    let user = users::ActiveModel {
        username: Set(input.username),
        email: Set(input.email),
        password_hash: Set(password_hash),
        display_name: Set(input
            .display_name
            .unwrap_or_else(|| "Administrator".to_owned())),
        role: Set(UserRole::Root),
        status: Set(UserStatus::Active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|err| validation_on_unique(err, "username or email already exists"))?;

    issue_token(&state.config.auth, user)
}

pub async fn status(state: &AppState) -> AppResult<AuthStatus> {
    let user_count = users::Entity::find().count(&state.db).await?;
    let settings = crate::services::settings::get(state).await?;

    Ok(AuthStatus {
        initialized: user_count > 0,
        registration_enabled: settings.registration_enabled,
    })
}

pub async fn register(state: &AppState, input: RegisterInput) -> AppResult<TokenResponse> {
    let settings = crate::services::settings::get(state).await?;
    if !settings.registration_enabled {
        return Err(AppError::Forbidden);
    }

    let mut context = crate::plugins::HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(crate::plugins::Hook::BeforeAuthRegister, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    validate_required(&input.username, "username")?;
    validate_required(&input.email, "email")?;
    crate::services::validation::email(&input.email, "email")?;
    validate_password(&input.password)?;

    let now = Utc::now();
    let user = users::ActiveModel {
        username: Set(input.username),
        email: Set(input.email),
        password_hash: Set(hash_password(&input.password)?),
        display_name: Set(input.display_name.unwrap_or_else(|| "Reader".to_owned())),
        role: Set(UserRole::Author),
        status: Set(UserStatus::Active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|err| validation_on_unique(err, "username or email already exists"))?;

    issue_token(&state.config.auth, user)
}

pub async fn login(state: &AppState, input: LoginInput) -> AppResult<TokenResponse> {
    let mut context = crate::plugins::HookContext::with_subject(&input)?;
    state
        .plugins
        .dispatch(crate::plugins::Hook::BeforeAuthLogin, &mut context)
        .await?;
    context.ensure_not_stopped()?;

    validate_required(&input.account, "account")?;
    validate_required(&input.password, "password")?;

    let user = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(input.account.clone())
                .or(users::Column::Email.eq(input.account)),
        )
        .one(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }
    if !matches!(user.status, UserStatus::Active) {
        return Err(AppError::Forbidden);
    }

    issue_token(&state.config.auth, user)
}

pub async fn current_user(state: &AppState, token: &str) -> AppResult<PublicUser> {
    let claims = decode_token(&state.config.auth, token)?;
    let user = users::Entity::find_by_id(claims.sub)
        .one(&state.db)
        .await?
        .ok_or(AppError::Unauthorized)?;

    if !matches!(user.status, UserStatus::Active) {
        return Err(AppError::Unauthorized);
    }

    Ok(user.into())
}

pub fn decode_token(config: &AuthConfig, token: &str) -> AppResult<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &validation,
    )?;
    Ok(data.claims)
}

fn issue_token(config: &AuthConfig, user: users::Model) -> AppResult<TokenResponse> {
    let now = Utc::now();
    let expires_at = now + Duration::seconds(config.token_ttl_seconds);
    let claims = Claims {
        sub: user.id,
        username: user.username.clone(),
        role: user.role.clone(),
        iat: now.timestamp() as usize,
        exp: expires_at.timestamp() as usize,
    };
    let access_token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )?;

    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_at: expires_at.timestamp(),
        user: user.into(),
    })
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| AppError::PasswordHash(err.to_string()))
}

fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed = PasswordHash::new(hash).map_err(|err| AppError::PasswordHash(err.to_string()))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

pub fn validate_required(value: &str, field: &'static str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(AppError::Validation(format!("{field} is required")));
    }

    Ok(())
}

pub fn validate_password(value: &str) -> AppResult<()> {
    if value.len() < 12 {
        return Err(AppError::Validation(
            "password must be at least 12 characters".to_owned(),
        ));
    }

    Ok(())
}

impl From<users::Model> for PublicUser {
    fn from(user: users::Model) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
            status: user.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuthConfig;

    #[test]
    fn password_policy_requires_minimum_length() {
        assert!(validate_password("short").is_err());
        assert!(validate_password("long-enough-password").is_ok());
    }

    #[test]
    fn password_hash_round_trips() {
        let hash = hash_password("long-enough-password").expect("hash");
        assert!(verify_password("long-enough-password", &hash).expect("verify"));
        assert!(!verify_password("different-password", &hash).expect("verify"));
    }

    #[test]
    fn token_round_trips() {
        let config = AuthConfig {
            jwt_secret: "test-secret-with-enough-entropy".to_owned(),
            token_ttl_seconds: 60,
        };
        let now = Utc::now();
        let token = encode(
            &Header::new(Algorithm::HS256),
            &Claims {
                sub: 42,
                username: "alice".to_owned(),
                role: UserRole::Admin,
                iat: now.timestamp() as usize,
                exp: (now + Duration::seconds(60)).timestamp() as usize,
            },
            &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
        )
        .expect("token");

        assert_eq!(decode_token(&config, &token).expect("claims").sub, 42);
    }
}
