use crate::error::{Error, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
}

impl SmtpConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            host: std::env::var("SMTP_HOST").ok()?,
            port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            username: std::env::var("SMTP_USERNAME").ok()?,
            password: std::env::var("SMTP_PASSWORD").ok()?,
            from_email: std::env::var("SMTP_FROM_EMAIL").ok()?,
            from_name: std::env::var("SMTP_FROM_NAME").unwrap_or_else(|_| "ADI".to_string()),
        })
    }
}

pub struct EmailSender {
    config: Option<SmtpConfig>,
}

impl EmailSender {
    pub fn new(config: Option<SmtpConfig>) -> Self {
        Self { config }
    }

    pub fn from_env() -> Self {
        Self::new(SmtpConfig::from_env())
    }

    pub fn send_verification_code(&self, to_email: &str, code: &str) -> Result<()> {
        let subject = "Your verification code";
        let body = format!(
            r#"Your verification code is: {}

This code will expire in 10 minutes.

If you didn't request this code, you can safely ignore this email."#,
            code
        );

        self.send(to_email, subject, &body)
    }

    fn send(&self, to_email: &str, subject: &str, body: &str) -> Result<()> {
        let Some(config) = &self.config else {
            tracing::warn!(
                "SMTP not configured - would send email to {}: {} - {}",
                to_email,
                subject,
                body
            );
            return Ok(());
        };

        let from = format!("{} <{}>", config.from_name, config.from_email);

        let email = Message::builder()
            .from(from.parse().map_err(|e| Error::EmailError(format!("{}", e)))?)
            .to(to_email.parse().map_err(|e| Error::EmailError(format!("{}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| Error::EmailError(format!("{}", e)))?;

        let creds = Credentials::new(config.username.clone(), config.password.clone());

        let mailer = SmtpTransport::relay(&config.host)
            .map_err(|e| Error::EmailError(format!("{}", e)))?
            .port(config.port)
            .credentials(creds)
            .build();

        mailer
            .send(&email)
            .map_err(|e| Error::EmailError(format!("{}", e)))?;

        tracing::info!("Sent verification email to {}", to_email);
        Ok(())
    }
}
