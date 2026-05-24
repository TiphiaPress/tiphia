use crate::{
    app::AppState,
    entities::users::{self, UserRole, UserStatus},
    error::{AppError, AppResult},
    services::{auth::PublicUser, users::UpdateUserInput},
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};

pub fn ensure_can_create_user(current_user: &PublicUser, role: &UserRole) -> AppResult<()> {
    ensure_can_assign_role(current_user, role)
}

pub fn ensure_can_assign_role(current_user: &PublicUser, role: &UserRole) -> AppResult<()> {
    match (&current_user.role, role) {
        (UserRole::Root, _) => Ok(()),
        (UserRole::Admin, UserRole::Editor | UserRole::Author) => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}

pub fn ensure_can_manage_user(current_user: &PublicUser, target: &users::Model) -> AppResult<()> {
    match (&current_user.role, &target.role) {
        (UserRole::Root, _) => Ok(()),
        (UserRole::Admin, UserRole::Editor | UserRole::Author) => Ok(()),
        _ if current_user.id == target.id => Ok(()),
        _ => Err(AppError::Forbidden),
    }
}

pub async fn ensure_not_last_active_root(
    state: &AppState,
    target: &users::Model,
    input: &UpdateUserInput,
) -> AppResult<()> {
    if !would_remove_active_root(target, input) {
        return Ok(());
    }

    let other_active_roots = users::Entity::find()
        .filter(users::Column::Role.eq(UserRole::Root))
        .filter(users::Column::Status.eq(UserStatus::Active))
        .filter(users::Column::Id.ne(target.id))
        .count(&state.db)
        .await?;

    if other_active_roots == 0 {
        return Err(AppError::Validation(
            "cannot remove the last active root user".to_owned(),
        ));
    }

    Ok(())
}

pub fn would_remove_active_root(target: &users::Model, input: &UpdateUserInput) -> bool {
    if target.role != UserRole::Root || target.status != UserStatus::Active {
        return false;
    }

    let role_removed = input
        .role
        .as_ref()
        .is_some_and(|role| !matches!(role, UserRole::Root));
    let status_removed = matches!(input.status, Some(UserStatus::Disabled));
    role_removed || status_removed
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn user_model(id: i32, role: UserRole, status: UserStatus) -> users::Model {
        let now = Utc::now();
        users::Model {
            id,
            username: format!("user{id}"),
            email: format!("user{id}@example.com"),
            password_hash: "hash".to_owned(),
            display_name: format!("User {id}"),
            role,
            status,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn detects_last_root_removal_when_root_is_disabled_or_demoted() {
        let root = user_model(1, UserRole::Root, UserStatus::Active);
        assert!(would_remove_active_root(
            &root,
            &UpdateUserInput {
                email: None,
                display_name: None,
                role: None,
                status: Some(UserStatus::Disabled),
            }
        ));
        assert!(would_remove_active_root(
            &root,
            &UpdateUserInput {
                email: None,
                display_name: None,
                role: Some(UserRole::Admin),
                status: None,
            }
        ));
    }

    #[test]
    fn ignores_non_root_or_noop_root_updates() {
        let root = user_model(1, UserRole::Root, UserStatus::Active);
        let admin = user_model(2, UserRole::Admin, UserStatus::Active);
        assert!(!would_remove_active_root(
            &root,
            &UpdateUserInput {
                email: Some("root@example.com".to_owned()),
                display_name: None,
                role: Some(UserRole::Root),
                status: Some(UserStatus::Active),
            }
        ));
        assert!(!would_remove_active_root(
            &admin,
            &UpdateUserInput {
                email: None,
                display_name: None,
                role: Some(UserRole::Author),
                status: Some(UserStatus::Disabled),
            }
        ));
    }
}
