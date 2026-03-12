//! ACME client wrapper for Let's Encrypt certificate management
//!
//! Uses `instant-acme` for the ACME protocol implementation.

use crate::cert_store::{AccountCredentials, CertStore, CertificateMetadata};
use crate::challenge::{ChallengeManager, ChallengeType, PendingChallenge};
use crate::config::SslConfig;
use anyhow::{Context, Result};
use chrono::Utc;
use instant_acme::{
    Account, AuthorizationStatus, ChallengeType as AcmeChallengeType, Identifier, NewAccount,
    NewOrder, OrderStatus,
};
use rcgen::{CertificateParams, DistinguishedName, KeyPair};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub struct AcmeClient {
    account: Account,
    config: SslConfig,
    cert_store: Arc<CertStore>,
}

impl AcmeClient {
    pub async fn new(config: SslConfig, cert_store: Arc<CertStore>) -> Result<Self> {
        let account = load_or_create_account(&config, &cert_store).await?;

        Ok(Self {
            account,
            config,
            cert_store,
        })
    }

    pub async fn obtain_certificate(
        &self,
        challenge_manager: &ChallengeManager,
    ) -> Result<ObtainedCertificate> {
        let domains = &self.config.domains;

        if domains.is_empty() {
            anyhow::bail!("No domains configured for certificate");
        }

        info!("Ordering certificate for domains: {:?}", domains);

        let identifiers: Vec<Identifier> =
            domains.iter().map(|d| Identifier::Dns(d.clone())).collect();

        let mut order = self
            .account
            .new_order(&NewOrder {
                identifiers: &identifiers,
            })
            .await
            .context("Failed to create ACME order")?;

        let state = order.state();
        info!("Order state: {:?}", state.status);

        let authorizations = order
            .authorizations()
            .await
            .context("Failed to get authorizations")?;

        let mut challenges_to_complete = Vec::new();

        for auth in &authorizations {
            match auth.status {
                AuthorizationStatus::Valid => {
                    debug!("Authorization already valid for {:?}", auth.identifier);
                    continue;
                }
                AuthorizationStatus::Pending => {
                    let (challenge, challenge_type, challenge_url) = self.select_challenge(auth)?;

                    let domain = match &auth.identifier {
                        Identifier::Dns(d) => d.clone(),
                    };

                    info!("Completing {} challenge for {}", challenge_type, domain);

                    let key_auth = order.key_authorization(&challenge).as_str().to_string();

                    let pending = PendingChallenge {
                        domain: domain.clone(),
                        token: challenge.token.clone(),
                        key_authorization: key_auth,
                        challenge_type,
                    };

                    challenge_manager.add_challenge(pending)?;
                    challenges_to_complete.push((domain, challenge_url));
                }
                AuthorizationStatus::Invalid => {
                    anyhow::bail!("Authorization invalid for {:?}", auth.identifier);
                }
                _ => {
                    warn!("Unexpected authorization status: {:?}", auth.status);
                }
            }
        }

        for (domain, challenge_url) in &challenges_to_complete {
            info!("Setting challenge ready for {}", domain);
            order
                .set_challenge_ready(challenge_url)
                .await
                .context("Failed to set challenge ready")?;
        }

        let mut tries = 0u8;
        let mut delay = Duration::from_millis(500);
        loop {
            tokio::time::sleep(delay).await;
            let state = order.refresh().await.context("Failed to refresh order")?;

            match state.status {
                OrderStatus::Ready => {
                    info!("Order is ready for finalization");
                    break;
                }
                OrderStatus::Invalid => {
                    for (domain, _) in &challenges_to_complete {
                        challenge_manager.remove_challenge(domain, "");
                    }
                    anyhow::bail!("Order became invalid");
                }
                _ => {
                    debug!("Order status: {:?}, waiting...", state.status);
                }
            }

            delay = std::cmp::min(delay * 2, Duration::from_secs(10));
            tries += 1;
            if tries >= 30 {
                for (domain, _) in &challenges_to_complete {
                    challenge_manager.remove_challenge(domain, "");
                }
                anyhow::bail!("Order did not become ready after {} attempts", tries);
            }
        }

        for (domain, _) in &challenges_to_complete {
            challenge_manager.remove_challenge(domain, "");
        }

        let mut params = CertificateParams::new(domains.clone())
            .context("Failed to create certificate params")?;
        params.distinguished_name = DistinguishedName::new();

        let private_key = KeyPair::generate().context("Failed to generate key pair")?;
        let csr = params
            .serialize_request(&private_key)
            .context("Failed to generate CSR")?;

        order
            .finalize(csr.der())
            .await
            .context("Failed to finalize order")?;

        let cert_chain_pem = loop {
            match order
                .certificate()
                .await
                .context("Failed to get certificate")?
            {
                Some(cert) => break cert,
                None => {
                    debug!("Certificate not ready yet, waiting...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        };

        let private_key_pem = private_key.serialize_pem();

        info!("Certificate obtained successfully for {:?}", domains);

        let expires_at = crate::cert_store::parse_certificate_expiry(&cert_chain_pem)
            .unwrap_or_else(|_| Utc::now() + chrono::Duration::days(90));

        let issuer = crate::cert_store::parse_certificate_issuer(&cert_chain_pem)
            .unwrap_or_else(|_| "Let's Encrypt".to_string());

        let serial = crate::cert_store::parse_certificate_serial(&cert_chain_pem)
            .unwrap_or_else(|_| "unknown".to_string());

        let metadata = CertificateMetadata {
            domains: domains.clone(),
            expires_at,
            issued_at: Utc::now(),
            renewed_at: None,
            issuer,
            serial,
        };

        let primary_domain = domains.first()
            .ok_or_else(|| anyhow::anyhow!("No domains configured for certificate"))?;
        self.cert_store
            .save_certificate(primary_domain, &cert_chain_pem, &private_key_pem, &metadata)
            .await?;

        Ok(ObtainedCertificate {
            cert_chain_pem,
            private_key_pem,
            metadata,
        })
    }

    fn select_challenge<'a>(
        &self,
        auth: &'a instant_acme::Authorization,
    ) -> Result<(&'a instant_acme::Challenge, ChallengeType, String)> {
        let priorities = match self.config.challenge_type {
            crate::config::AcmeChallengeType::Http01 => vec![AcmeChallengeType::Http01],
            crate::config::AcmeChallengeType::TlsAlpn01 => vec![AcmeChallengeType::TlsAlpn01],
            crate::config::AcmeChallengeType::Auto => {
                vec![AcmeChallengeType::Http01, AcmeChallengeType::TlsAlpn01]
            }
        };

        for acme_type in priorities {
            if let Some(challenge) = auth.challenges.iter().find(|c| c.r#type == acme_type) {
                let challenge_type = match acme_type {
                    AcmeChallengeType::Http01 => ChallengeType::Http01,
                    AcmeChallengeType::TlsAlpn01 => ChallengeType::TlsAlpn01,
                    _ => continue,
                };
                return Ok((challenge, challenge_type, challenge.url.clone()));
            }
        }

        anyhow::bail!(
            "No supported challenge type found. Available: {:?}",
            auth.challenges
                .iter()
                .map(|c| &c.r#type)
                .collect::<Vec<_>>()
        )
    }

    pub async fn needs_renewal(&self) -> Result<bool> {
        for domain in &self.config.domains {
            if self
                .cert_store
                .needs_renewal(domain, self.config.renew_before_days)
                .await?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn account(&self) -> &Account {
        &self.account
    }
}

pub struct ObtainedCertificate {
    pub cert_chain_pem: String,
    pub private_key_pem: String,
    pub metadata: CertificateMetadata,
}

async fn load_or_create_account(config: &SslConfig, cert_store: &CertStore) -> Result<Account> {
    if let Some(creds) = cert_store.load_account().await? {
        info!("Loading existing ACME account: {}", creds.email);

        let instant_creds: instant_acme::AccountCredentials =
            serde_json::from_str(&creds.private_key_pem)
                .context("Failed to deserialize account credentials")?;

        let account = Account::from_credentials(instant_creds)
            .await
            .context("Failed to load ACME account from credentials")?;

        return Ok(account);
    }

    info!("Creating new ACME account for: {}", config.email);

    let (account, credentials) = Account::create(
        &NewAccount {
            contact: &[&format!("mailto:{}", config.email)],
            terms_of_service_agreed: true,
            only_return_existing: false,
        },
        config.acme_directory_url(),
        None,
    )
    .await
    .context("Failed to create ACME account")?;

    let creds_json =
        serde_json::to_string(&credentials).context("Failed to serialize account credentials")?;

    let account_creds = AccountCredentials {
        account_url: account.id().to_string(),
        private_key_pem: creds_json, // Store serialized credentials here
        email: config.email.clone(),
        created_at: Utc::now(),
    };

    cert_store.save_account(&account_creds).await?;

    info!("ACME account created: {}", account.id());

    Ok(account)
}
