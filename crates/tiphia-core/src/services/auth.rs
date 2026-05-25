use crate::{
    app::AppState,
    entities::users::{self, UserRole, UserStatus},
    error::{AppError, AppResult, validation_on_unique},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};

#[path = "auth/input.rs"]
mod input;
#[path = "auth/model.rs"]
mod model;
#[path = "auth/password.rs"]
mod password;
#[path = "auth/token.rs"]
mod token;

pub use input::{
    AuthStatus, BootstrapAdminInput, ExtensionMap, LoginInput, RegisterInput, plugin_extension,
};
pub use model::{Claims, PublicUser, TokenResponse};
pub use password::{hash_password, validate_password, validate_required};
pub use token::decode_token;

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

    token::issue_token(&state.config.auth, user)
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

    token::issue_token(&state.config.auth, user)
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

    if !password::verify_password(&input.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }
    if !matches!(user.status, UserStatus::Active) {
        return Err(AppError::Forbidden);
    }

    token::issue_token(&state.config.auth, user)
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
