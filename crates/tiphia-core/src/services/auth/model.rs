use crate::{
    entities::users::{self, UserRole, UserStatus},
    error::{AppError, AppResult},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
