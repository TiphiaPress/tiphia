use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use qrcode::{QrCode, render::svg};
use rand::{RngCore, rngs::OsRng};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::Sha1;
use tiphia_core::{
    AppError, AppResult, AppState,
    plugins::{
        Hook, HookContext, HookMap, Plugin, PluginConfigField, PluginConfigFieldType,
        PluginConfigSchema, PluginManifest, PluginRegistryBuilder, ensure_plugin_config,
        load_plugin_config,
    },
    routes::auth::CurrentUser,
    services::{
        auth::{LoginInput, plugin_extension},
        options,
    },
};
use tracing::info;

const PLUGIN_NAME: &str = "tiphia-authenticator";
const USER_SECRET_PREFIX: &str = "plugin:tiphia-authenticator:user:";
const ISSUER_DEFAULT: &str = "TiphiaPress";
const DIGITS: u32 = 6;
const PERIOD_SECONDS: i64 = 30;
const WINDOW: i64 = 1;

type HmacSha1 = Hmac<Sha1>;

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(AuthenticatorPlugin);
    Ok(())
}

pub struct AuthenticatorPlugin;

static MANIFEST: PluginManifest = PluginManifest {
    name: PLUGIN_NAME,
    version: "0.1.0",
    description: "Adds TOTP two-factor authentication compatible with Google and Microsoft Authenticator.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for AuthenticatorPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(
            db,
            self.manifest().name,
            json!(AuthenticatorConfig::default()),
        )
        .await
    }

    fn hooks(&self) -> HookMap {
        [(Hook::BeforeAuthLogin, 30)].into_iter().collect()
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "issuer",
                    label: "Issuer",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!(ISSUER_DEFAULT)),
                    help: Some("Name shown in authenticator apps."),
                },
                PluginConfigField {
                    key: "enforce_for_all_users",
                    label: "Require for bound users",
                    field_type: PluginConfigFieldType::Boolean,
                    required: false,
                    default: Some(json!(true)),
                    help: Some(
                        "When enabled, users who have bound a TOTP secret must provide a code on login.",
                    ),
                },
            ],
        })
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        if hook != Hook::BeforeAuthLogin {
            return Ok(());
        }

        let config = load_config(context.database()?).await?;
        if !config.enforce_for_all_users {
            return Ok(());
        }

        let Some(input) = context.subject_as::<LoginInput>()? else {
            return Ok(());
        };

        let Some(secret) = secret_for_account(context.database()?, &input.account).await? else {
            return Ok(());
        };

        let Some(code) = authenticator_code(&input)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            context.stop("authenticator code is required");
            return Ok(());
        };

        if !verify_totp_code(&secret, code, Utc::now().timestamp())? {
            context.stop("invalid authenticator code");
        }

        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(
            Router::new()
                .route("/authenticator/config", get(public_config))
                .route("/authenticator/setup", post(setup_current_user))
                .route("/authenticator/status", get(status_current_user))
                .route("/authenticator/disable", post(disable_current_user)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct AuthenticatorConfig {
    #[serde(default = "default_issuer")]
    issuer: String,
    #[serde(default = "default_true")]
    enforce_for_all_users: bool,
}

impl Default for AuthenticatorConfig {
    fn default() -> Self {
        Self {
            issuer: default_issuer(),
            enforce_for_all_users: true,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct PublicAuthenticatorConfig {
    enabled: bool,
    issuer: String,
}

#[derive(Clone, Debug, Serialize)]
struct AuthenticatorStatus {
    bound: bool,
}

#[derive(Clone, Debug, Serialize)]
struct AuthenticatorSetupResponse {
    secret: String,
    otpauth_url: String,
    qr_svg: String,
}

async fn public_config(
    State(state): State<AppState>,
) -> AppResult<Json<PublicAuthenticatorConfig>> {
    let config = load_config(&state.db).await?;
    Ok(Json(PublicAuthenticatorConfig {
        enabled: true,
        issuer: normalized_issuer(&config),
    }))
}

async fn status_current_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<AuthenticatorStatus>> {
    Ok(Json(AuthenticatorStatus {
        bound: user_secret(&state, user.id).await?.is_some(),
    }))
}

async fn setup_current_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<AuthenticatorSetupResponse>> {
    let config = load_config(&state.db).await?;
    let secret = generate_secret();
    options::upsert_json(&state, &user_secret_key(user.id), json!(secret), false).await?;
    let issuer = normalized_issuer(&config);
    let account = format!("{} ({})", user.username, user.email);
    let otpauth_url = otpauth_url(&issuer, &account, &secret);
    Ok(Json(AuthenticatorSetupResponse {
        qr_svg: qr_svg(&otpauth_url)?,
        otpauth_url,
        secret,
    }))
}

async fn disable_current_user(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<AuthenticatorStatus>> {
    options::upsert_json(&state, &user_secret_key(user.id), Value::Null, false).await?;
    Ok(Json(AuthenticatorStatus { bound: false }))
}

fn authenticator_code(input: &LoginInput) -> Option<&str> {
    plugin_extension(&input.extensions, PLUGIN_NAME)
        .and_then(|value| value.get("totp_code"))
        .and_then(Value::as_str)
}
async fn load_config(db: &DatabaseConnection) -> AppResult<AuthenticatorConfig> {
    load_plugin_config(db, PLUGIN_NAME, AuthenticatorConfig::default()).await
}

async fn user_secret(state: &AppState, user_id: i32) -> AppResult<Option<String>> {
    let value = options::get_json(state, &user_secret_key(user_id)).await?;
    Ok(value
        .and_then(|value| value.as_str().map(str::to_owned))
        .filter(|value| !value.is_empty()))
}

async fn secret_for_account(db: &DatabaseConnection, account: &str) -> AppResult<Option<String>> {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    use tiphia_core::entities::{options as options_entity, users};

    let Some(user) = users::Entity::find()
        .filter(
            users::Column::Username
                .eq(account.to_owned())
                .or(users::Column::Email.eq(account.to_owned())),
        )
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    let key = user_secret_key(user.id);
    let option = options_entity::Entity::find()
        .filter(options_entity::Column::Key.eq(key))
        .one(db)
        .await?;

    Ok(option
        .and_then(|option| option.value.as_str().map(str::to_owned))
        .filter(|value| !value.is_empty()))
}

fn user_secret_key(user_id: i32) -> String {
    format!("{USER_SECRET_PREFIX}{user_id}")
}

fn generate_secret() -> String {
    let mut bytes = [0u8; 20];
    OsRng.fill_bytes(&mut bytes);
    base32_encode(&bytes)
}

fn qr_svg(value: &str) -> AppResult<String> {
    let code = QrCode::new(value.as_bytes()).map_err(|err| AppError::Plugin(err.to_string()))?;
    Ok(code.render::<svg::Color>().min_dimensions(220, 220).build())
}

fn otpauth_url(issuer: &str, account: &str, secret: &str) -> String {
    format!(
        "otpauth://totp/{}:{}?secret={}&issuer={}&algorithm=SHA1&digits={}&period={}",
        urlencoding::encode(issuer),
        urlencoding::encode(account),
        secret,
        urlencoding::encode(issuer),
        DIGITS,
        PERIOD_SECONDS,
    )
}

fn verify_totp_code(secret: &str, code: &str, timestamp: i64) -> AppResult<bool> {
    if code.len() != DIGITS as usize || !code.chars().all(|ch| ch.is_ascii_digit()) {
        return Ok(false);
    }
    let Some(secret_bytes) = base32_decode(secret) else {
        return Err(AppError::Plugin("invalid authenticator secret".to_owned()));
    };
    let step = timestamp / PERIOD_SECONDS;
    for offset in -WINDOW..=WINDOW {
        if totp_at_step(&secret_bytes, step + offset)? == code {
            return Ok(true);
        }
    }
    Ok(false)
}

fn totp_at_step(secret: &[u8], step: i64) -> AppResult<String> {
    let counter = (step as u64).to_be_bytes();
    let mut mac =
        HmacSha1::new_from_slice(secret).map_err(|err| AppError::Plugin(err.to_string()))?;
    mac.update(&counter);
    let digest = mac.finalize().into_bytes();
    let offset = (digest[19] & 0x0f) as usize;
    let value = (((digest[offset] & 0x7f) as u32) << 24)
        | ((digest[offset + 1] as u32) << 16)
        | ((digest[offset + 2] as u32) << 8)
        | digest[offset + 3] as u32;
    Ok(format!("{:06}", value % 10u32.pow(DIGITS)))
}

fn base32_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut output = String::new();
    let mut buffer = 0u16;
    let mut bits = 0u8;
    for byte in bytes {
        buffer = (buffer << 8) | *byte as u16;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        output.push(ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    output
}

fn base32_decode(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0u32;
    let mut bits = 0u8;
    for ch in value.chars().filter(|ch| *ch != '=' && !ch.is_whitespace()) {
        let raw = match ch.to_ascii_uppercase() {
            'A'..='Z' => ch.to_ascii_uppercase() as u8 - b'A',
            '2'..='7' => ch as u8 - b'2' + 26,
            _ => return None,
        } as u32;
        buffer = (buffer << 5) | raw;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    Some(output)
}

fn normalized_issuer(config: &AuthenticatorConfig) -> String {
    let issuer = config.issuer.trim();
    if issuer.is_empty() {
        ISSUER_DEFAULT.to_owned()
    } else {
        issuer.to_owned()
    }
}

fn default_issuer() -> String {
    ISSUER_DEFAULT.to_owned()
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base32_roundtrip() {
        let raw = b"hello world";
        assert_eq!(base32_decode(&base32_encode(raw)).unwrap(), raw);
    }

    #[test]
    fn verifies_known_totp_vector() {
        let secret = base32_encode(b"12345678901234567890");
        assert!(verify_totp_code(&secret, "287082", 59).is_ok());
    }
}
