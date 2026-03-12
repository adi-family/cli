//! ACME challenge handlers for HTTP-01 and TLS-ALPN-01
//!
//! HTTP-01: Serves challenge tokens at `/.well-known/acme-challenge/{token}`
//! TLS-ALPN-01: Serves challenge certificate via ALPN protocol negotiation

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use dashmap::DashMap;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    server::{ClientHello, ResolvesServerCert},
    sign::CertifiedKey,
};
use std::sync::Arc;
use tracing::{debug, info, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeType {
    Http01,
    TlsAlpn01,
}

impl std::fmt::Display for ChallengeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Http01 => write!(f, "http-01"),
            Self::TlsAlpn01 => write!(f, "tls-alpn-01"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PendingChallenge {
    pub domain: String,
    pub token: String,
    pub key_authorization: String,
    pub challenge_type: ChallengeType,
}

pub struct ChallengeManager {
    http_challenges: DashMap<String, String>,
    alpn_challenges: DashMap<String, Arc<CertifiedKey>>,
    redirect_https: bool,
    https_port: u16,
}

impl ChallengeManager {
    pub fn new(redirect_https: bool, https_port: u16) -> Self {
        Self {
            http_challenges: DashMap::new(),
            alpn_challenges: DashMap::new(),
            redirect_https,
            https_port,
        }
    }

    pub fn add_challenge(&self, challenge: PendingChallenge) -> Result<()> {
        match challenge.challenge_type {
            ChallengeType::Http01 => {
                info!(
                    "Adding HTTP-01 challenge for {} (token: {})",
                    challenge.domain, challenge.token
                );
                self.http_challenges
                    .insert(challenge.token, challenge.key_authorization);
            }
            ChallengeType::TlsAlpn01 => {
                info!(
                    "Adding TLS-ALPN-01 challenge for {} (token: {})",
                    challenge.domain, challenge.token
                );
                let cert_key =
                    generate_alpn_challenge_cert(&challenge.domain, &challenge.key_authorization)?;
                self.alpn_challenges
                    .insert(challenge.domain, Arc::new(cert_key));
            }
        }
        Ok(())
    }

    pub fn remove_challenge(&self, domain: &str, token: &str) {
        self.http_challenges.remove(token);
        self.alpn_challenges.remove(domain);
        debug!("Removed challenge for {} (token: {})", domain, token);
    }

    pub fn get_http_challenge(&self, token: &str) -> Option<String> {
        self.http_challenges.get(token).map(|r| r.clone())
    }

    pub fn get_alpn_challenge(&self, domain: &str) -> Option<Arc<CertifiedKey>> {
        self.alpn_challenges.get(domain).map(|r| r.clone())
    }

    pub fn has_pending_challenges(&self) -> bool {
        !self.http_challenges.is_empty() || !self.alpn_challenges.is_empty()
    }

    pub fn http_router(self: Arc<Self>) -> Router {
        Router::new()
            .route(
                "/.well-known/acme-challenge/{token}",
                get(http_challenge_handler),
            )
            .route("/", get(http_root_handler))
            .fallback(http_fallback_handler)
            .with_state(self)
    }
}

async fn http_challenge_handler(
    State(manager): State<Arc<ChallengeManager>>,
    Path(token): Path<String>,
) -> Response {
    debug!("HTTP-01 challenge request for token: {}", token);

    match manager.get_http_challenge(&token) {
        Some(key_auth) => {
            info!("Serving HTTP-01 challenge for token: {}", token);
            (StatusCode::OK, key_auth).into_response()
        }
        None => {
            warn!("Unknown HTTP-01 challenge token: {}", token);
            (StatusCode::NOT_FOUND, "Challenge not found").into_response()
        }
    }
}

async fn http_root_handler(State(manager): State<Arc<ChallengeManager>>) -> Response {
    if manager.redirect_https && !manager.has_pending_challenges() {
        // Get Host header would require request, simplified here
        Redirect::permanent(&format!("https://localhost:{}/", manager.https_port)).into_response()
    } else {
        (StatusCode::OK, "Hive HTTP Challenge Server").into_response()
    }
}

async fn http_fallback_handler(
    State(manager): State<Arc<ChallengeManager>>,
    req: axum::extract::Request,
) -> Response {
    if manager.redirect_https && !manager.has_pending_challenges() {
        let path = req.uri().path();
        let query = req
            .uri()
            .query()
            .map(|q| format!("?{}", q))
            .unwrap_or_default();
        Redirect::permanent(&format!(
            "https://localhost:{}{}{}",
            manager.https_port, path, query
        ))
        .into_response()
    } else {
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

pub struct AlpnChallengeResolver {
    manager: Arc<ChallengeManager>,
    default_resolver: Arc<dyn ResolvesServerCert>,
}

impl std::fmt::Debug for AlpnChallengeResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlpnChallengeResolver")
            .field("manager", &"ChallengeManager")
            .field("default_resolver", &"dyn ResolvesServerCert")
            .finish()
    }
}

impl AlpnChallengeResolver {
    pub fn new(
        manager: Arc<ChallengeManager>,
        default_resolver: Arc<dyn ResolvesServerCert>,
    ) -> Self {
        Self {
            manager,
            default_resolver,
        }
    }
}

impl ResolvesServerCert for AlpnChallengeResolver {
    fn resolve(&self, client_hello: ClientHello<'_>) -> Option<Arc<CertifiedKey>> {
        let is_acme_alpn = client_hello
            .alpn()
            .map(|mut alpn| alpn.any(|p| p == b"acme-tls/1"))
            .unwrap_or(false);

        if is_acme_alpn {
            if let Some(sni) = client_hello.server_name() {
                debug!("TLS-ALPN-01 challenge request for domain: {}", sni);

                if let Some(cert) = self.manager.get_alpn_challenge(sni) {
                    info!("Serving TLS-ALPN-01 challenge for domain: {}", sni);
                    return Some(cert);
                }

                warn!("No TLS-ALPN-01 challenge found for domain: {}", sni);
            }
        }

        self.default_resolver.resolve(client_hello)
    }
}

/// Generate a self-signed certificate for TLS-ALPN-01 challenge.
/// The certificate embeds the key authorization hash in the acmeIdentifier extension (OID 1.3.6.1.5.5.7.1.31).
fn generate_alpn_challenge_cert(domain: &str, key_authorization: &str) -> Result<CertifiedKey> {
    use rcgen::{CertificateParams, CustomExtension, DistinguishedName, DnType, KeyPair};
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(key_authorization.as_bytes());
    let hash = hasher.finalize();

    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, domain.to_string());

    params.subject_alt_names = vec![rcgen::SanType::DnsName(domain.try_into().unwrap())];

    // acmeIdentifier OID: 1.3.6.1.5.5.7.1.31; value is SHA-256 hash as DER OCTET STRING
    let acme_identifier_oid = vec![1, 3, 6, 1, 5, 5, 7, 1, 31];
    let mut extension_value = vec![0x04, 0x20]; // Tag: OCTET STRING, length: 32
    extension_value.extend_from_slice(&hash);

    let extension = CustomExtension::from_oid_content(&acme_identifier_oid, extension_value);
    params.custom_extensions = vec![extension];

    let key_pair = KeyPair::generate().context("Failed to generate key pair")?;
    let cert = params
        .self_signed(&key_pair)
        .context("Failed to generate self-signed certificate")?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    let signing_key = rustls::crypto::aws_lc_rs::sign::any_supported_type(&key_der)
        .map_err(|e| anyhow::anyhow!("Failed to create signing key: {}", e))?;

    Ok(CertifiedKey::new(vec![cert_der], signing_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenge_manager_http() {
        let manager = ChallengeManager::new(false, 443);

        let challenge = PendingChallenge {
            domain: "example.com".to_string(),
            token: "test-token-123".to_string(),
            key_authorization: "test-key-auth".to_string(),
            challenge_type: ChallengeType::Http01,
        };

        manager.add_challenge(challenge).unwrap();
        assert!(manager.has_pending_challenges());

        let response = manager.get_http_challenge("test-token-123");
        assert_eq!(response, Some("test-key-auth".to_string()));

        manager.remove_challenge("example.com", "test-token-123");
        assert!(!manager.has_pending_challenges());
    }
}
