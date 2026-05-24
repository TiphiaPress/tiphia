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
