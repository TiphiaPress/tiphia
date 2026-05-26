use crate::config::{CommentMailPushConfig, SmtpEncryption};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
    transport::smtp::{authentication::Credentials, client::Tls},
};
use tiphia_core::{AppResult, error::AppError};

pub async fn send_mail(
    config: &CommentMailPushConfig,
    to: &str,
    subject: &str,
    html: &str,
) -> AppResult<()> {
    if !config.smtp_ready() {
        return Err(AppError::Plugin("SMTP config is incomplete".to_owned()));
    }

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

    let message = Message::builder()
        .from(from)
        .to(to)
        .subject(subject)
        .header(ContentType::TEXT_HTML)
        .body(html.to_owned())
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

    builder
        .build()
        .send(message)
        .await
        .map_err(|err| AppError::Plugin(format!("failed to send email: {err}")))?;
    Ok(())
}

pub fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
