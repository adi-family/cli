use crate::error::{Error, Result};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};

#[derive(Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_email: String,
    pub from_name: String,
    pub tls: bool,
}

impl SmtpConfig {
    pub fn from_env() -> Option<Self> {
        use lib_env_parse::{env_bool_default_true, env_opt, env_or, env_vars};
        env_vars! {
            SmtpHost      => "SMTP_HOST",
            SmtpPort      => "SMTP_PORT",
            SmtpUsername  => "SMTP_USERNAME",
            SmtpPassword  => "SMTP_PASSWORD",
            SmtpFromEmail => "SMTP_FROM_EMAIL",
            SmtpFromName  => "SMTP_FROM_NAME",
            SmtpTls       => "SMTP_TLS",
        }

        Some(Self {
            host: env_opt(EnvVar::SmtpHost.as_str())?,
            port: env_opt(EnvVar::SmtpPort.as_str())
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            username: env_opt(EnvVar::SmtpUsername.as_str()),
            password: env_opt(EnvVar::SmtpPassword.as_str()),
            from_email: env_opt(EnvVar::SmtpFromEmail.as_str())?,
            from_name: env_or(EnvVar::SmtpFromName.as_str(), "ADI"),
            tls: env_bool_default_true(EnvVar::SmtpTls.as_str()),
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
        if lib_env_parse::env_opt("UNSAFE_PRINT_EMAIL_CODE_IN_CONSOLE").is_some() {
            tracing::warn!("VERIFICATION CODE for {}: {}", to_email, code);
        }

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

        // Quote the display name if it contains spaces (RFC 5322)
        let from = if config.from_name.contains(' ') {
            format!("\"{}\" <{}>", config.from_name, config.from_email)
        } else {
            format!("{} <{}>", config.from_name, config.from_email)
        };

        let email = Message::builder()
            .from(
                from.parse()
                    .map_err(|e| Error::EmailError(format!("{}", e)))?,
            )
            .to(to_email
                .parse()
                .map_err(|e| Error::EmailError(format!("{}", e)))?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| Error::EmailError(format!("{}", e)))?;

        let creds = match (&config.username, &config.password) {
            (Some(user), Some(pass)) if !user.is_empty() => {
                Some(Credentials::new(user.clone(), pass.clone()))
            }
            _ => None,
        };

        // TLS modes: disabled (plain), port 465 (implicit TLS), other (STARTTLS)
        let mailer = if !config.tls {
            let mut builder = SmtpTransport::builder_dangerous(&config.host).port(config.port);
            if let Some(creds) = creds {
                builder = builder.credentials(creds);
            }
            builder.build()
        } else if config.port == 465 {
            let mut builder = SmtpTransport::relay(&config.host)
                .map_err(|e| Error::EmailError(format!("{}", e)))?
                .port(465)
                .tls(lettre::transport::smtp::client::Tls::Wrapper(
                    lettre::transport::smtp::client::TlsParameters::new(config.host.clone())
                        .map_err(|e| Error::EmailError(format!("{}", e)))?,
                ));
            if let Some(creds) = creds {
                builder = builder.credentials(creds);
            }
            builder.build()
        } else {
            let mut builder = SmtpTransport::starttls_relay(&config.host)
                .map_err(|e| Error::EmailError(format!("{}", e)))?
                .port(config.port);
            if let Some(creds) = creds {
                builder = builder.credentials(creds);
            }
            builder.build()
        };

        mailer
            .send(&email)
            .map_err(|e| Error::EmailError(format!("{}", e)))?;

        tracing::info!("Sent verification email to {}", to_email);
        Ok(())
    }
}
