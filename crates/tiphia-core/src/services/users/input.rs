use crate::{
    entities::users::{UserRole, UserStatus},
    pagination::PaginationQuery,
};
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
