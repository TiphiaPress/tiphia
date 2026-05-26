use serde::{Deserialize, Serialize};
use tiphia_core::plugins::{PluginConfigField, PluginConfigFieldType, PluginConfigSchema};

pub fn config_schema() -> PluginConfigSchema {
    use serde_json::json;
    PluginConfigSchema {
        fields: vec![
            bool_field(
                "enabled",
                "启用插件",
                false,
                "关闭时评论推送和找回密码都不会生效。",
            ),
            bool_field(
                "comment_push_enabled",
                "启用评论邮件推送",
                false,
                "有新评论时发送邮件通知。",
            ),
            bool_field(
                "password_reset_enabled",
                "启用找回密码",
                false,
                "启用后登录页可显示找回密码入口。",
            ),
            number_field(
                "reset_token_ttl_minutes",
                "邮件过期时间（分钟）",
                30,
                "找回密码链接有效期。",
            ),
            text_field(
                "comment_notify_email",
                "评论通知收件邮箱",
                false,
                "为空时默认发送到发件邮箱。",
            ),
            text_field("from_name", "发信人名称", false, "邮件中展示的发信人名称。"),
            text_field("from_email", "发件邮箱地址", true, "SMTP 发件邮箱。"),
            text_field(
                "recovery_base_url",
                "邮件恢复地址",
                false,
                "例如 https://blog.example.com/admin/login?reset=1。插件会追加 token 参数。",
            ),
            text_field("smtp_host", "SMTP 地址", true, "例如 smtp.example.com。"),
            number_field("smtp_port", "SMTP 端口", 587, "常见端口：25、465、587。"),
            text_field(
                "smtp_username",
                "SMTP 登录用户",
                false,
                "需要服务器验证时填写。",
            ),
            PluginConfigField {
                key: "smtp_password",
                label: "SMTP 登录密码",
                field_type: PluginConfigFieldType::Text,
                required: false,
                default: Some(json!("")),
                help: Some("需要服务器验证时填写。建议使用邮箱服务商的应用专用密码。"),
            },
            bool_field(
                "smtp_auth_required",
                "需要服务器验证",
                true,
                "勾选后会使用 SMTP 登录用户和密码认证。",
            ),
            PluginConfigField {
                key: "smtp_encryption",
                label: "SMTP 加密模式",
                field_type: PluginConfigFieldType::Text,
                required: true,
                default: Some(json!("tls")),
                help: Some(
                    "可选：none、ssl、tls。none 为无安全加密，ssl 为 465 隐式 SSL，tls 为 STARTTLS。",
                ),
            },
        ],
    }
}

fn text_field(
    key: &'static str,
    label: &'static str,
    required: bool,
    help: &'static str,
) -> PluginConfigField {
    PluginConfigField {
        key,
        label,
        field_type: PluginConfigFieldType::Text,
        required,
        default: Some(serde_json::json!("")),
        help: Some(help),
    }
}

fn number_field(
    key: &'static str,
    label: &'static str,
    default: u64,
    help: &'static str,
) -> PluginConfigField {
    PluginConfigField {
        key,
        label,
        field_type: PluginConfigFieldType::Number,
        required: true,
        default: Some(serde_json::json!(default)),
        help: Some(help),
    }
}

fn bool_field(
    key: &'static str,
    label: &'static str,
    default: bool,
    help: &'static str,
) -> PluginConfigField {
    PluginConfigField {
        key,
        label,
        field_type: PluginConfigFieldType::Boolean,
        required: false,
        default: Some(serde_json::json!(default)),
        help: Some(help),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicConfigResponse {
    pub enabled: bool,
    pub comment_push_enabled: bool,
    pub password_reset_enabled: bool,
}
