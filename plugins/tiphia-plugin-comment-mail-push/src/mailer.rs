use crate::config::{CommentMailPushConfig, SmtpEncryption};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{authentication::Credentials, client::Tls},
};
use tiphia_core::{AppResult, error::AppError};
use tracing::{debug, info, warn};

pub async fn send_mail(
    config: &CommentMailPushConfig,
    to: &str,
    subject: &str,
    html: &str,
) -> AppResult<()> {
    if !config.smtp_ready() {
        return Err(AppError::Plugin("SMTP config is incomplete".to_owned()));
    }

    let recipient_email = to.trim().to_owned();
    let from_email = config.from_email.trim().to_owned();
    debug!(
        plugin = "tiphia-comment-mail-push",
        smtp_host = %config.smtp_host.trim(),
        smtp_port = config.smtp_port,
        smtp_encryption = ?config.smtp_encryption,
        smtp_auth_required = config.smtp_auth_required,
        from = %from_email,
        to = %recipient_email,
        subject = %subject,
        "preparing email"
    );

    let from = Mailbox::new(
        (!config.from_name.trim().is_empty()).then(|| config.from_name.trim().to_owned()),
        config
            .from_email
            .parse()
            .map_err(|err| AppError::Plugin(format!("invalid from email: {err}")))?,
    );
    let to = Mailbox::new(
        None,
        to.parse()
            .map_err(|err| AppError::Plugin(format!("invalid recipient email: {err}")))?,
    );

    let mut message_builder = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_HTML);
    if !config.reply_to_email.trim().is_empty() {
        let reply_to = Mailbox::new(
            None,
            config
                .reply_to_email
                .parse()
                .map_err(|err| AppError::Plugin(format!("invalid reply-to email: {err}")))?,
        );
        message_builder = message_builder.reply_to(reply_to);
    }

    let html = with_custom_css(config, html);
    let message = message_builder
        .body(html)
        .map_err(|err| AppError::Plugin(format!("failed to build email: {err}")))?;

    let mut builder = match config.smtp_encryption {
        SmtpEncryption::None => {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(config.smtp_host.trim())
        }
        SmtpEncryption::Ssl => AsyncSmtpTransport::<Tokio1Executor>::relay(config.smtp_host.trim())
            .map_err(|err| AppError::Plugin(format!("failed to build SSL SMTP client: {err}")))?,
        SmtpEncryption::Tls => AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(
            config.smtp_host.trim(),
        )
        .map_err(|err| AppError::Plugin(format!("failed to build TLS SMTP client: {err}")))?,
    }
    .port(config.smtp_port);

    if config.smtp_auth_required {
        builder = builder.credentials(Credentials::new(
            config.smtp_username.clone(),
            config.smtp_password.clone(),
        ));
    }

    if matches!(config.smtp_encryption, SmtpEncryption::None) {
        builder = builder.tls(Tls::None);
    }

    let mailer = builder.build();
    info!(
        plugin = "tiphia-comment-mail-push",
        to = %recipient_email,
        subject = %subject,
        smtp_host = %config.smtp_host.trim(),
        smtp_port = config.smtp_port,
        smtp_encryption = ?config.smtp_encryption,
        "sending email"
    );

    match mailer.send(message).await {
        Ok(_) => {
            info!(
                plugin = "tiphia-comment-mail-push",
                to = %recipient_email,
                subject = %subject,
                "email sent"
            );
            Ok(())
        }
        Err(err) => {
            warn!(
                plugin = "tiphia-comment-mail-push",
                to = %recipient_email,
                subject = %subject,
                error = %err,
                "failed to send email"
            );
            Err(AppError::Plugin(format!("failed to send email: {err}")))
        }
    }
}

fn with_custom_css(config: &CommentMailPushConfig, html: &str) -> String {
    let css = config.email_custom_css.trim();
    if css.is_empty() {
        return html.to_owned();
    }
    if html.contains("{{custom_css}}") {
        return html.replace("{{custom_css}}", css);
    }
    format!("<style>{css}</style>\n{html}")
}
pub fn render_email_template(template: &str, variables: &[(&str, String)]) -> String {
    let mut html = template.to_owned();
    for (key, value) in variables {
        html = html.replace(&format!("{{{{{key}}}}}"), value);
    }
    html
}
pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
