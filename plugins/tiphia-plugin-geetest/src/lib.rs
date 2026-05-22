use async_trait::async_trait;
use axum::{Json, Router, extract::State, routing::get};
use hmac::{Hmac, Mac};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::Sha256;
use tiphia_core::{
    AppError, AppResult, AppState,
    plugins::{
        Hook, HookContext, HookMap, Plugin, PluginConfigField, PluginConfigFieldType,
        PluginConfigSchema, PluginManifest, PluginRegistryBuilder, ensure_plugin_config,
        load_plugin_config,
    },
    services::{
        auth::{LoginInput, RegisterInput},
        comments::CreateCommentInput,
    },
};
use tracing::info;

type HmacSha256 = Hmac<Sha256>;

pub fn register(builder: &mut PluginRegistryBuilder) -> AppResult<()> {
    builder.register(GeetestPlugin);
    Ok(())
}

pub struct GeetestPlugin;

static GEETEST_MANIFEST: PluginManifest = PluginManifest {
    name: "tiphia-geetest",
    version: "0.1.0",
    description: "Adds GeeTest v4 captcha verification to login and public registration.",
    author: "Tiphia",
};

#[async_trait]
impl Plugin for GeetestPlugin {
    fn manifest(&self) -> &'static PluginManifest {
        &GEETEST_MANIFEST
    }

    async fn install(&self, db: &DatabaseConnection) -> AppResult<()> {
        ensure_plugin_config(db, self.manifest().name, json!(GeetestConfig::default())).await
    }

    fn hooks(&self) -> HookMap {
        [
            (Hook::BeforeAuthLogin, 20),
            (Hook::BeforeAuthRegister, 20),
            (Hook::BeforeCommentCreate, 20),
        ]
        .into_iter()
        .collect()
    }

    fn config_schema(&self) -> Option<PluginConfigSchema> {
        Some(PluginConfigSchema {
            fields: vec![
                PluginConfigField {
                    key: "captcha_id",
                    label: "GeeTest captcha ID",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("GeeTest v4 captcha_id from the GeeTest console."),
                },
                PluginConfigField {
                    key: "captcha_key",
                    label: "GeeTest captcha key",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("GeeTest v4 captcha_key used for server-side validation."),
                },
                PluginConfigField {
                    key: "verify_login",
                    label: "Verify login",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(json!(true)),
                    help: Some("Require captcha on admin login when configured."),
                },
                PluginConfigField {
                    key: "verify_register",
                    label: "Verify registration",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(json!(true)),
                    help: Some("Require captcha on public registration when configured."),
                },
                PluginConfigField {
                    key: "verify_comment",
                    label: "Verify comments",
                    field_type: PluginConfigFieldType::Boolean,
                    required: true,
                    default: Some(json!(true)),
                    help: Some("Require captcha when visitors submit blog comments."),
                },
                PluginConfigField {
                    key: "product",
                    label: "GeeTest product",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("float")),
                    help: Some("Display mode: float, popup, or bind."),
                },
                PluginConfigField {
                    key: "native_button_width",
                    label: "Button width",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("100%")),
                    help: Some("GeeTest native button width, e.g. 100%, 260px, 16rem."),
                },
                PluginConfigField {
                    key: "native_button_height",
                    label: "Button height",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("GeeTest native button height, e.g. 50px."),
                },
                PluginConfigField {
                    key: "rem",
                    label: "Scale ratio",
                    field_type: PluginConfigFieldType::Number,
                    required: false,
                    default: Some(json!(null)),
                    help: Some("GeeTest rem scale ratio."),
                },
                PluginConfigField {
                    key: "language",
                    label: "Language",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("GeeTest language, e.g. zho, eng, zho-tw, jpn."),
                },
                PluginConfigField {
                    key: "protocol",
                    label: "Protocol",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("Optional protocol override: http:// or https://."),
                },
                PluginConfigField {
                    key: "timeout",
                    label: "Timeout",
                    field_type: PluginConfigFieldType::Number,
                    required: false,
                    default: Some(json!(30000)),
                    help: Some("Single request timeout in milliseconds."),
                },
                PluginConfigField {
                    key: "next_width",
                    label: "Popup width",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("")),
                    help: Some("GeeTest next popup width."),
                },
                PluginConfigField {
                    key: "mask_outside",
                    label: "Close popup on outside click",
                    field_type: PluginConfigFieldType::Boolean,
                    required: false,
                    default: Some(json!(true)),
                    help: Some("Whether clicking outside captcha closes the popup."),
                },
                PluginConfigField {
                    key: "mask_bg_color",
                    label: "Mask background color",
                    field_type: PluginConfigFieldType::Text,
                    required: false,
                    default: Some(json!("#0000004d")),
                    help: Some("CSS color for GeeTest popup mask."),
                },
                PluginConfigField {
                    key: "hide_success",
                    label: "Hide bind success popup",
                    field_type: PluginConfigFieldType::Boolean,
                    required: false,
                    default: Some(json!(false)),
                    help: Some("Only works when product is bind."),
                },
            ],
        })
    }

    async fn activate(&self) -> AppResult<()> {
        info!(plugin = self.manifest().name, "plugin activated");
        Ok(())
    }

    async fn handle(&self, hook: Hook, context: &mut HookContext) -> AppResult<()> {
        let config = load_config(context.database()?).await?;
        if !config.ready() {
            return Ok(());
        }

        let captcha = match hook {
            Hook::BeforeAuthLogin if config.verify_login => context
                .subject_as::<LoginInput>()?
                .and_then(|input| input.captcha),
            Hook::BeforeAuthRegister if config.verify_register => context
                .subject_as::<RegisterInput>()?
                .and_then(|input| input.captcha),
            Hook::BeforeCommentCreate if config.verify_comment => context
                .subject_as::<CreateCommentInput>()?
                .and_then(|input| input.captcha),
            _ => None,
        };

        let Some(captcha) = captcha else {
            context.stop("captcha is required");
            return Ok(());
        };

        verify_geetest(&config, captcha).await?;
        Ok(())
    }

    fn route_prefix(&self) -> Option<&'static str> {
        Some("/api/v1")
    }

    fn route_router(&self) -> Option<Router<AppState>> {
        Some(
            Router::new()
                .route("/geetest/config", get(public_config))
                .route("/geetest/config/", get(public_config)),
        )
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GeetestConfig {
    #[serde(default)]
    captcha_id: String,
    #[serde(default)]
    captcha_key: String,
    #[serde(default = "default_true")]
    verify_login: bool,
    #[serde(default = "default_true")]
    verify_register: bool,
    #[serde(default = "default_true")]
    verify_comment: bool,
    #[serde(default = "default_product")]
    product: String,
    #[serde(default = "default_button_width")]
    native_button_width: String,
    #[serde(default)]
    native_button_height: String,
    #[serde(default)]
    rem: Option<f64>,
    #[serde(default)]
    language: String,
    #[serde(default)]
    protocol: String,
    #[serde(default = "default_timeout")]
    timeout: Option<u64>,
    #[serde(default)]
    next_width: String,
    #[serde(default = "default_true")]
    mask_outside: bool,
    #[serde(default = "default_mask_bg")]
    mask_bg_color: String,
    #[serde(default)]
    hide_success: bool,
}

impl Default for GeetestConfig {
    fn default() -> Self {
        Self {
            captcha_id: String::new(),
            captcha_key: String::new(),
            verify_login: true,
            verify_register: true,
            verify_comment: true,
            product: default_product(),
            native_button_width: default_button_width(),
            native_button_height: String::new(),
            rem: None,
            language: String::new(),
            protocol: String::new(),
            timeout: default_timeout(),
            next_width: String::new(),
            mask_outside: true,
            mask_bg_color: default_mask_bg(),
            hide_success: false,
        }
    }
}

impl GeetestConfig {
    fn ready(&self) -> bool {
        !self.captcha_id.trim().is_empty() && !self.captcha_key.trim().is_empty()
    }
}

#[derive(Clone, Debug, Serialize)]
struct PublicGeetestConfig {
    enabled: bool,
    captcha_id: Option<String>,
    verify_login: bool,
    verify_register: bool,
    verify_comment: bool,
    product: String,
    native_button_width: String,
    native_button_height: Option<String>,
    rem: Option<f64>,
    language: Option<String>,
    protocol: Option<String>,
    timeout: Option<u64>,
    next_width: Option<String>,
    mask_outside: bool,
    mask_bg_color: String,
    hide_success: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct GeetestProof {
    lot_number: String,
    captcha_output: String,
    pass_token: String,
    gen_time: String,
}

#[derive(Clone, Debug, Deserialize)]
struct GeetestValidateResponse {
    result: String,
    #[serde(default)]
    reason: String,
}

async fn public_config(State(state): State<AppState>) -> AppResult<Json<PublicGeetestConfig>> {
    let config = load_config(&state.db).await?;
    Ok(Json(PublicGeetestConfig {
        enabled: config.ready(),
        captcha_id: config.ready().then(|| config.captcha_id.trim().to_owned()),
        verify_login: config.verify_login,
        verify_register: config.verify_register,
        verify_comment: config.verify_comment,
        product: sanitize_product(&config.product).to_owned(),
        native_button_width: non_empty(&config.native_button_width)
            .unwrap_or_else(|| "100%".to_owned()),
        native_button_height: non_empty(&config.native_button_height),
        rem: config.rem.filter(|value| *value > 0.0),
        language: non_empty(&config.language),
        protocol: sanitize_protocol(&config.protocol).map(str::to_owned),
        timeout: config.timeout.filter(|value| *value > 0),
        next_width: non_empty(&config.next_width),
        mask_outside: config.mask_outside,
        mask_bg_color: non_empty(&config.mask_bg_color).unwrap_or_else(default_mask_bg),
        hide_success: config.hide_success,
    }))
}

async fn load_config(db: &DatabaseConnection) -> AppResult<GeetestConfig> {
    load_plugin_config(db, GEETEST_MANIFEST.name, GeetestConfig::default()).await
}

async fn verify_geetest(config: &GeetestConfig, value: Value) -> AppResult<()> {
    let proof: GeetestProof =
        serde_json::from_value(value).map_err(|err| AppError::Validation(err.to_string()))?;
    let captcha_id = config.captcha_id.trim();
    let captcha_key = config.captcha_key.trim();
    let sign_token = sign_lot_number(captcha_key, &proof.lot_number)?;
    let response = reqwest::Client::new()
        .post("https://gcaptcha4.geetest.com/validate")
        .query(&[("captcha_id", captcha_id)])
        .form(&[
            ("lot_number", proof.lot_number.as_str()),
            ("captcha_output", proof.captcha_output.as_str()),
            ("pass_token", proof.pass_token.as_str()),
            ("gen_time", proof.gen_time.as_str()),
            ("sign_token", sign_token.as_str()),
        ])
        .send()
        .await
        .map_err(|err| AppError::Plugin(format!("geetest request failed: {err}")))?
        .error_for_status()
        .map_err(|err| AppError::Plugin(format!("geetest status failed: {err}")))?
        .json::<GeetestValidateResponse>()
        .await
        .map_err(|err| AppError::Plugin(format!("geetest response failed: {err}")))?;

    if response.result == "success" {
        return Ok(());
    }

    Err(AppError::Validation(format!(
        "captcha verification failed: {}",
        response.reason
    )))
}

fn sign_lot_number(captcha_key: &str, lot_number: &str) -> AppResult<String> {
    let mut mac = HmacSha256::new_from_slice(captcha_key.as_bytes())
        .map_err(|err| AppError::Plugin(err.to_string()))?;
    mac.update(lot_number.as_bytes());
    Ok(hex_lower(&mac.finalize().into_bytes()))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn default_true() -> bool {
    true
}

fn default_product() -> String {
    "float".to_owned()
}

fn default_button_width() -> String {
    "100%".to_owned()
}

fn default_timeout() -> Option<u64> {
    Some(30000)
}

fn default_mask_bg() -> String {
    "#0000004d".to_owned()
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_owned())
}

fn sanitize_product(value: &str) -> &str {
    match value.trim() {
        "popup" => "popup",
        "bind" => "bind",
        _ => "float",
    }
}

fn sanitize_protocol(value: &str) -> Option<&str> {
    match value.trim() {
        "http://" => Some("http://"),
        "https://" => Some("https://"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_lot_number_as_lower_hex_hmac_sha256() {
        let signature = sign_lot_number("secret", "lot").expect("signature");
        assert_eq!(signature.len(), 64);
        assert!(signature.chars().all(|ch| ch.is_ascii_hexdigit()));
    }
}
