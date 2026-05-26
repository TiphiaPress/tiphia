use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PasswordResetRecord {
    pub user_id: i32,
    pub expires_at: DateTime<Utc>,
}

pub fn generate_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

pub fn expires_at(ttl_minutes: u32) -> DateTime<Utc> {
    let ttl = ttl_minutes.clamp(1, 1440) as i64;
    Utc::now() + Duration::minutes(ttl)
}

pub fn append_token(base_url: &str, token: &str) -> String {
    let separator = if base_url.contains('?') { '&' } else { '?' };
    format!("{}{}token={}", base_url.trim(), separator, token)
}

pub fn reset_option_key(token_hash: &str) -> String {
    format!("plugin:tiphia-comment-mail-push:password-reset:{token_hash}")
}
