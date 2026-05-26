use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub const DEFAULT_COMMENT_EMAIL_TEMPLATE: &str = r#"<h2>收到新评论</h2>
<p><strong>文章：</strong><a href="{{post_url}}">{{post_title}}</a></p>
<p><strong>评论者：</strong>{{sender_name}} &lt;{{sender_email}}&gt;</p>
<p><strong>文章作者：</strong>{{post_author_name}}</p>
<p><strong>状态：</strong>{{comment_status}}</p>
<blockquote>{{comment_content}}</blockquote>"#;

pub const DEFAULT_COMMENT_REPLY_EMAIL_TEMPLATE: &str = r#"<h2>你的评论收到回复</h2>
<p>{{recipient_name}}，你好：</p>
<p>{{sender_name}} 回复了你在 <a href="{{post_url}}">{{post_title}}</a> 下的评论。</p>
<p><strong>你原来的评论：</strong></p>
<blockquote>{{replied_content}}</blockquote>
<p><strong>新的回复：</strong></p>
<blockquote>{{comment_content}}</blockquote>"#;

pub const DEFAULT_PASSWORD_RESET_EMAIL_TEMPLATE: &str = r#"<h2>找回密码</h2>
<p>你好，{{display_name}}：</p>
<p>请点击下面的链接重置密码。该链接将在 {{ttl_minutes}} 分钟后过期。</p>
<p><a href="{{reset_url}}">重置密码</a></p>
<p>如果不是你本人操作，可以忽略这封邮件。</p>"#;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
#[serde(default)]
pub struct CommentMailPushConfig {
    pub comment_push_enabled: bool,
    pub comment_reply_enabled: bool,
    pub notify_post_author_enabled: bool,
    pub password_reset_enabled: bool,
    pub reset_token_ttl_minutes: u32,
    pub comment_notify_email: String,
    pub from_name: String,
    pub from_email: String,
    pub reply_to_email: String,
    #[serde(alias = "recovery_base_url")]
    pub password_reset_base_url: String,
    pub comment_email_template: String,
    pub comment_reply_email_template: String,
    pub password_reset_email_template: String,
    pub email_custom_css: String,
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
            comment_push_enabled: false,
            comment_reply_enabled: false,
            notify_post_author_enabled: false,
            password_reset_enabled: false,
            reset_token_ttl_minutes: 30,
            comment_notify_email: String::new(),
            from_name: "TiphiaPress".to_owned(),
            from_email: String::new(),
            reply_to_email: String::new(),
            password_reset_base_url: String::new(),
            comment_email_template: DEFAULT_COMMENT_EMAIL_TEMPLATE.to_owned(),
            comment_reply_email_template: DEFAULT_COMMENT_REPLY_EMAIL_TEMPLATE.to_owned(),
            password_reset_email_template: DEFAULT_PASSWORD_RESET_EMAIL_TEMPLATE.to_owned(),
            email_custom_css: String::new(),
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
        !self.from_email.trim().is_empty()
            && !self.smtp_host.trim().is_empty()
            && self.smtp_port > 0
    }

    pub fn password_reset_ready(&self) -> bool {
        self.smtp_ready()
            && self.password_reset_enabled
            && !self.password_reset_base_url.trim().is_empty()
    }

    pub fn comment_push_ready(&self) -> bool {
        self.smtp_ready() && self.comment_push_enabled
    }

    pub fn comment_reply_ready(&self) -> bool {
        self.smtp_ready() && self.comment_reply_enabled
    }

    pub fn comment_recipient(&self) -> Option<String> {
        let email = if self.comment_notify_email.trim().is_empty() {
            self.from_email.trim()
        } else {
            self.comment_notify_email.trim()
        };
        (!email.is_empty()).then(|| email.to_owned())
    }

    pub fn comment_template(&self) -> &str {
        non_empty_or_default(&self.comment_email_template, DEFAULT_COMMENT_EMAIL_TEMPLATE)
    }

    pub fn comment_reply_template(&self) -> &str {
        non_empty_or_default(
            &self.comment_reply_email_template,
            DEFAULT_COMMENT_REPLY_EMAIL_TEMPLATE,
        )
    }

    pub fn password_reset_template(&self) -> &str {
        non_empty_or_default(
            &self.password_reset_email_template,
            DEFAULT_PASSWORD_RESET_EMAIL_TEMPLATE,
        )
    }
}

fn non_empty_or_default<'a>(value: &'a str, default: &'a str) -> &'a str {
    let value = value.trim();
    if value.is_empty() { default } else { value }
}
