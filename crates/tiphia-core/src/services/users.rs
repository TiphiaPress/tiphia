use crate::{
    app::AppState,
    entities::users::{self, UserStatus},
    error::{AppError, AppResult, validation_on_unique},
    pagination::Page,
    services::{
        auth::{PublicUser, hash_password, validate_password, validate_required},
        validation,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, QueryOrder, Set};

#[path = "users/input.rs"]
mod input;
#[path = "users/normalization.rs"]
mod normalization;
#[path = "users/permissions.rs"]
mod permissions;

pub use input::{ChangePasswordInput, CreateUserInput, ListUserQuery, UpdateUserInput};

pub async fn list(state: &AppState, query: ListUserQuery) -> AppResult<Page<PublicUser>> {
    let page = query.pagination.page();
    let per_page = query.pagination.per_page();
    let paginator = users::Entity::find()
        .order_by_desc(users::Column::CreatedAt)
        .paginate(&state.db, per_page);
    let total = paginator.num_items().await?;
    let total_pages = paginator.num_pages().await?;
    let items = paginator
        .fetch_page(page - 1)
        .await?
        .into_iter()
        .map(PublicUser::from)
        .collect();

    Ok(Page::new(items, page, per_page, total, total_pages))
}

pub async fn create(
    state: &AppState,
    current_user: &PublicUser,
    input: CreateUserInput,
) -> AppResult<PublicUser> {
    permissions::ensure_can_create_user(current_user, &input.role)?;
    let username = input.username.trim().to_owned();
    let email = normalization::normalize_email(input.email);
    let display_name = input.display_name.trim().to_owned();
    validate_user_identity(&username, &email, &display_name)?;
    validate_password(&input.password)?;

    let now = Utc::now();
    let user = users::ActiveModel {
        username: Set(username),
        email: Set(email),
        password_hash: Set(hash_password(&input.password)?),
        display_name: Set(display_name),
        role: Set(input.role),
        status: Set(UserStatus::Active),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&state.db)
    .await
    .map_err(|err| validation_on_unique(err, "username or email already exists"))?;

    Ok(user.into())
}

pub async fn show(state: &AppState, id: i32) -> AppResult<PublicUser> {
    users::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .map(PublicUser::from)
        .ok_or(AppError::NotFound("user"))
}

pub async fn update(
    state: &AppState,
    current_user: &PublicUser,
    id: i32,
    input: UpdateUserInput,
) -> AppResult<PublicUser> {
    let existing = users::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("user"))?;
    permissions::ensure_can_manage_user(current_user, &existing)?;
    permissions::ensure_not_last_active_root(state, &existing, &input).await?;
    ensure_not_disabling_self(current_user, &existing, &input)?;
    if let Some(role) = &input.role {
        permissions::ensure_can_assign_role(current_user, role)?;
    }

    let mut model: users::ActiveModel = existing.into();
    apply_update_input(&mut model, input)?;
    model.updated_at = Set(Utc::now());

    Ok(model
        .update(&state.db)
        .await
        .map_err(|err| validation_on_unique(err, "username or email already exists"))?
        .into())
}

pub async fn change_password(
    state: &AppState,
    current_user: &PublicUser,
    id: i32,
    input: ChangePasswordInput,
) -> AppResult<PublicUser> {
    validate_password(&input.password)?;

    let existing = users::Entity::find_by_id(id)
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("user"))?;
    permissions::ensure_can_manage_user(current_user, &existing)?;
    let mut model: users::ActiveModel = existing.into();
    model.password_hash = Set(hash_password(&input.password)?);
    model.updated_at = Set(Utc::now());

    Ok(model.update(&state.db).await?.into())
}

fn apply_update_input(model: &mut users::ActiveModel, input: UpdateUserInput) -> AppResult<()> {
    if let Some(email) = input.email {
        let email = normalization::normalize_email(email);
        validate_required(&email, "email")?;
        validation::email(&email, "email")?;
        model.email = Set(email);
    }
    if let Some(display_name) = input.display_name {
        let display_name = display_name.trim().to_owned();
        validate_required(&display_name, "display_name")?;
        model.display_name = Set(display_name);
    }
    if let Some(role) = input.role {
        model.role = Set(role);
    }
    if let Some(status) = input.status {
        model.status = Set(status);
    }

    Ok(())
}

fn validate_user_identity(username: &str, email: &str, display_name: &str) -> AppResult<()> {
    validate_required(username, "username")?;
    validate_required(email, "email")?;
    validation::email(email, "email")?;
    validate_required(display_name, "display_name")?;
    Ok(())
}

fn ensure_not_disabling_self(
    current_user: &PublicUser,
    target: &users::Model,
    input: &UpdateUserInput,
) -> AppResult<()> {
    if matches!(input.status, Some(UserStatus::Disabled)) && current_user.id == target.id {
        return Err(AppError::Validation(
            "cannot disable current user".to_owned(),
        ));
    }

    Ok(())
}
