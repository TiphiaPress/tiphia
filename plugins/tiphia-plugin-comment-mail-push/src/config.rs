use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(default)]
pub struct CommentMailPushConfig {
    pub enabled: bool,
    pub comment_push_enabled: bool,
    pub password_reset_enabled: bool,
    pub reset_token_ttl_minutes: u32,
    pub comment_notify_email: String,
    pub from_name: String,
    pub from_email: String,
    pub recovery_base_url: String,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_auth_required: bool,
    pub smtp_encryption: SmtpEncryption,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SmtpEncryption {
    None,
    Ssl,
    Tls,
}

impl Default for CommentMailPushConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            comment_push_enabled: false,
            password_reset_enabled: false,
            reset_token_ttl_minutes: 30,
            comment_notify_email: String::new(),
            from_name: "TiphiaPress".to_owned(),
            from_email: String::new(),
            recovery_base_url: String::new(),
            smtp_host: String::new(),
            smtp_port: 587,
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_auth_required: true,
            smtp_encryption: SmtpEncryption::Tls,
        }
    }
}

impl CommentMailPushConfig {
    pub fn smtp_ready(&self) -> bool {
        self.enabled
            && !self.from_email.trim().is_empty()
            && !self.smtp_host.trim().is_empty()
            && self.smtp_port > 0
    }

    pub fn password_reset_ready(&self) -> bool {
        self.smtp_ready()
            && self.password_reset_enabled
            && !self.recovery_base_url.trim().is_empty()
    }

    pub fn comment_push_ready(&self) -> bool {
        self.smtp_ready() && self.comment_push_enabled
    }

    pub fn comment_recipient(&self) -> Option<String> {
        let email = if self.comment_notify_email.trim().is_empty() {
            self.from_email.trim()
        } else {
            self.comment_notify_email.trim()
        };
        (!email.is_empty()).then(|| email.to_owned())
    }
}
