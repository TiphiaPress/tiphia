use crate::{
    app::AppState,
    entities::users::{self, UserRole, UserStatus},
    error::{AppError, AppResult, validation_on_unique},
    pagination::{Page, PaginationQuery},
    services::{
        auth::{PublicUser, hash_password, validate_password, validate_required},
        validation,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, QueryOrder, Set};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, IntoParams)]
pub struct ListUserQuery {
    #[serde(flatten)]
    pub pagination: PaginationQuery,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub role: UserRole,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<UserRole>,
    pub status: Option<UserStatus>,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub struct ChangePasswordInput {
    pub password: String,
}

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
    ensure_can_create_user(current_user, &input.role)?;
    validate_required(&input.username, "username")?;
    validate_required(&input.email, "email")?;
    validation::email(&input.email, "email")?;
    validate_required(&input.display_name, "display_name")?;
    validate_password(&input.password)?;

    let now = Utc::now();
    let user = users::ActiveModel {
        username: Set(input.username),
        email: Set(input.email),
        password_hash: Set(hash_password(&input.password)?),
        display_name: Set(input.display_name),
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
    ensure_can_manage_user(current_user, &existing)?;
    if matches!(input.status, Some(UserStatus::Disabled)) && current_user.id == existing.id {
        return Err(AppError::Validation(
            "cannot disable current user".to_owned(),
        ));
    }
    if let Some(role) = &input.role {
        ensure_can_assign_role(current_user, role)?;
    }
    let mut model: users::ActiveModel = existing.into();

    if let Some(email) = input.email {
        validate_required(&email, "email")?;
        validation::email(&email, "email")?;
        model.email = Set(email);
    }
    if let Some(display_name) = input.display_name {
        validate_required(&display_name, "display_name")?;
        model.display_name = Set(display_name);
    }
    if let Some(role) = input.role {
        model.role = Set(role);
    }
    if let Some(status) = input.status {
        model.status = Set(status);
    }
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
    ensure_can_manage_user(current_user, &existing)?;
    let mut model: users::ActiveModel = existing.into();
    model.password_hash = Set(hash_password(&input.password)?);
    model.updated_at = Set(Utc::now());

    Ok(model.update(&state.db).await?.into())
}

fn ensure_can_create_user(current_user: &PublicUser, role: &UserRole) -> AppResult<()> {
    ensure_can_assign_role(current_user, role)
}

fn ensure_can_assign_role(current_user: &PublicUser, role: &UserRole) -> AppResult<()> {
    match (&current_user.role, role) {
        (UserRole::Root, _) => Ok(()),
        (UserRole::Admin, UserRole::Editor | UserRole::Author) => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}

fn ensure_can_manage_user(current_user: &PublicUser, target: &users::Model) -> AppResult<()> {
    match (&current_user.role, &target.role) {
        (UserRole::Root, _) => Ok(()),
        (UserRole::Admin, UserRole::Editor | UserRole::Author) => Ok(()),
        _ if current_user.id == target.id => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}
