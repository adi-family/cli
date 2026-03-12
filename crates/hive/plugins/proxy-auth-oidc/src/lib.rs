//! OpenID Connect Authentication Proxy Plugin for Hive
//!
//! Validates OIDC tokens using issuer's JWKS endpoint.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   auth-oidc:
//!     # Discovery URL (recommended)
//!     issuer: https://accounts.google.com
//!     
//!     # Or manual configuration
//!     jwks_uri: https://accounts.google.com/.well-known/jwks.json
//!     
//!     # Token validation
//!     audience: my-client-id      # Required audience claim
//!     required_claims:
//!       - email
//!       - email_verified
//!     
//!     # Where to find the token
//!     token_source: header        # header or cookie
//!     token_name: Authorization   # Header name or cookie name
//!     token_prefix: "Bearer "     # Prefix to strip (for header)
//!     
//!     # Forward claims to backend
//!     forward_claims:
//!       sub: X-User-ID
//!       email: X-User-Email
//!     
//!     # JWKS cache
//!     jwks_cache_ttl: 3600        # Seconds to cache JWKS
//! ```

use anyhow::{anyhow, Result};
use lib_plugin_abi_v3::{
    async_trait,
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

struct JwksCache {
    jwks: JwkSet,
    fetched_at: Instant,
    ttl: Duration,
}

impl JwksCache {
    fn is_expired(&self) -> bool {
        self.fetched_at.elapsed() > self.ttl
    }
}

pub struct OidcAuthPlugin {
    config: OidcConfig,
    http_client: reqwest::Client,
    jwks_cache: Arc<RwLock<Option<JwksCache>>>,
}

impl Default for OidcAuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl OidcAuthPlugin {
    pub fn new() -> Self {
        Self {
            config: OidcConfig::default(),
            http_client: reqwest::Client::new(),
            jwks_cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn get_jwks_uri(&self) -> Result<String> {
        if let Some(ref uri) = self.config.jwks_uri {
            return Ok(uri.clone());
        }

        let issuer = self
            .config
            .issuer
            .as_ref()
            .ok_or_else(|| anyhow!("Either 'issuer' or 'jwks_uri' must be configured"))?;

        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );

        debug!("Fetching OIDC discovery document from {}", discovery_url);

        let response = self
            .http_client
            .get(&discovery_url)
            .send()
            .await?
            .error_for_status()?;

        let discovery: OidcDiscovery = response.json().await?;
        Ok(discovery.jwks_uri)
    }

    async fn get_jwks(&self) -> Result<JwkSet> {
        {
            let cache = self.jwks_cache.read().await;
            if let Some(ref cached) = *cache {
                if !cached.is_expired() {
                    return Ok(cached.jwks.clone());
                }
            }
        }

        let jwks_uri = self.get_jwks_uri().await?;
        debug!("Fetching JWKS from {}", jwks_uri);

        let response = self
            .http_client
            .get(&jwks_uri)
            .send()
            .await?
            .error_for_status()?;

        let jwks: JwkSet = response.json().await?;

        {
            let mut cache = self.jwks_cache.write().await;
            *cache = Some(JwksCache {
                jwks: jwks.clone(),
                fetched_at: Instant::now(),
                ttl: Duration::from_secs(self.config.jwks_cache_ttl),
            });
        }

        Ok(jwks)
    }

    fn extract_token(&self, req: &ProxyRequest) -> Option<String> {
        match self.config.token_source.as_str() {
            "header" => {
                let header_value = req.headers.get(&self.config.token_name)?;
                let token = if let Some(ref prefix) = self.config.token_prefix {
                    header_value.strip_prefix(prefix)?.to_string()
                } else {
                    header_value.clone()
                };
                Some(token)
            }
            "cookie" => {
                let cookie_header = req.headers.get("cookie")?;
                for cookie in cookie_header.split(';') {
                    let cookie = cookie.trim();
                    let mut parts = cookie.splitn(2, '=');
                    let name = parts.next()?;
                    let value = parts.next().unwrap_or("");
                    if name == self.config.token_name {
                        return Some(value.to_string());
                    }
                }
                None
            }
            _ => None,
        }
    }

    async fn validate_token(&self, token: &str) -> Result<HashMap<String, serde_json::Value>> {
        let header = decode_header(token)?;
        let kid = header
            .kid
            .ok_or_else(|| anyhow!("Token missing 'kid' in header"))?;

        let jwks = self.get_jwks().await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| anyhow!("Key '{}' not found in JWKS", kid))?;

        let decoding_key = DecodingKey::from_jwk(jwk)?;

        let mut validation = Validation::new(match header.alg {
            Algorithm::RS256 => Algorithm::RS256,
            Algorithm::RS384 => Algorithm::RS384,
            Algorithm::RS512 => Algorithm::RS512,
            Algorithm::ES256 => Algorithm::ES256,
            Algorithm::ES384 => Algorithm::ES384,
            alg => return Err(anyhow!("Unsupported algorithm: {:?}", alg)),
        });

        if let Some(ref aud) = self.config.audience {
            validation.set_audience(&[aud]);
        } else {
            validation.validate_aud = false;
        }

        if let Some(ref iss) = self.config.issuer {
            validation.set_issuer(&[iss]);
        }

        let token_data = decode::<HashMap<String, serde_json::Value>>(token, &decoding_key, &validation)?;

        for claim in &self.config.required_claims {
            if !token_data.claims.contains_key(claim) {
                return Err(anyhow!("Missing required claim: {}", claim));
            }
        }

        Ok(token_data.claims)
    }
}

#[async_trait]
impl Plugin for OidcAuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.auth.oidc".to_string(),
            name: "OIDC Auth Proxy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("OpenID Connect authentication proxy middleware".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(auth_config) = ctx.config.get("auth-oidc") {
            self.config = serde_json::from_value(auth_config.clone())?;
        }

        if self.config.issuer.is_some() || self.config.jwks_uri.is_some() {
            match self.get_jwks().await {
                Ok(jwks) => debug!("Pre-fetched JWKS with {} keys", jwks.keys.len()),
                Err(e) => warn!("Failed to pre-fetch JWKS: {}", e),
            }
        }

        debug!(
            "OIDC auth plugin initialized: issuer={:?}, audience={:?}",
            self.config.issuer, self.config.audience
        );
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for OidcAuthPlugin {
    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        let token = match self.extract_token(&req) {
            Some(t) => t,
            None => {
                debug!("No token found in request");
                return Ok(ProxyResult::Response(
                    ProxyResponse::new(401)
                        .with_header("content-type", "application/json")
                        .with_header(
                            "www-authenticate",
                            "Bearer realm=\"API\", error=\"invalid_token\"",
                        )
                        .with_body(r#"{"error": "Authentication required"}"#),
                ));
            }
        };

        let claims = match self.validate_token(&token).await {
            Ok(claims) => claims,
            Err(e) => {
                error!("Token validation failed: {}", e);
                return Ok(ProxyResult::Response(
                    ProxyResponse::new(401)
                        .with_header("content-type", "application/json")
                        .with_header(
                            "www-authenticate",
                            "Bearer realm=\"API\", error=\"invalid_token\"",
                        )
                        .with_body(format!(r#"{{"error": "Invalid token: {}"}}"#, e)),
                ));
            }
        };

        debug!("Token validated, sub={:?}", claims.get("sub"));

        for (claim, header) in &self.config.forward_claims {
            if let Some(value) = claims.get(claim) {
                let value_str = match value {
                    serde_json::Value::String(s) => s.clone(),
                    _ => value.to_string(),
                };
                req.headers.insert(header.clone(), value_str);
            }
        }

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// OIDC issuer URL (used to discover JWKS via `/.well-known/openid-configuration`)
    pub issuer: Option<String>,
    /// Direct JWKS URI (skips discovery if set)
    pub jwks_uri: Option<String>,
    pub audience: Option<String>,
    #[serde(default)]
    pub required_claims: Vec<String>,
    /// Where to find token: "header" or "cookie"
    #[serde(default = "default_token_source")]
    pub token_source: String,
    #[serde(default = "default_token_name")]
    pub token_name: String,
    /// Prefix to strip from token value (e.g., "Bearer ")
    pub token_prefix: Option<String>,
    #[serde(default)]
    pub forward_claims: HashMap<String, String>,
    #[serde(default = "default_jwks_cache_ttl")]
    pub jwks_cache_ttl: u64,
}

fn default_token_source() -> String {
    "header".to_string()
}

fn default_token_name() -> String {
    "Authorization".to_string()
}

fn default_jwks_cache_ttl() -> u64 {
    3600
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            issuer: None,
            jwks_uri: None,
            audience: None,
            required_claims: Vec::new(),
            token_source: default_token_source(),
            token_name: default_token_name(),
            token_prefix: Some("Bearer ".to_string()),
            forward_claims: HashMap::new(),
            jwks_cache_ttl: default_jwks_cache_ttl(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OidcDiscovery {
    jwks_uri: String,
    #[allow(dead_code)]
    issuer: String,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(OidcAuthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = OidcAuthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.auth.oidc");
        assert_eq!(meta.name, "OIDC Auth Proxy");
        assert_eq!(meta.plugin_type, PluginType::Orchestration);
        assert_eq!(meta.category, Some(PluginCategory::Proxy));
    }

    #[test]
    fn test_extract_from_header() {
        let mut plugin = OidcAuthPlugin::new();
        plugin.config.token_source = "header".to_string();
        plugin.config.token_name = "Authorization".to_string();
        plugin.config.token_prefix = Some("Bearer ".to_string());

        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer my-token".to_string());

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers,
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_token(&req), Some("my-token".to_string()));
    }

    #[test]
    fn test_extract_from_cookie() {
        let mut plugin = OidcAuthPlugin::new();
        plugin.config.token_source = "cookie".to_string();
        plugin.config.token_name = "access_token".to_string();
        plugin.config.token_prefix = None;

        let mut headers = HashMap::new();
        headers.insert(
            "cookie".to_string(),
            "session=abc; access_token=my-token".to_string(),
        );

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/api/test".to_string(),
            headers,
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_token(&req), Some("my-token".to_string()));
    }

    #[test]
    fn test_config_parse() {
        let config = serde_json::json!({
            "issuer": "https://accounts.google.com",
            "audience": "my-client-id",
            "required_claims": ["email"],
            "forward_claims": {
                "sub": "X-User-ID",
                "email": "X-User-Email"
            }
        });

        let oidc_config: OidcConfig = serde_json::from_value(config).unwrap();
        assert_eq!(oidc_config.issuer, Some("https://accounts.google.com".to_string()));
        assert_eq!(oidc_config.audience, Some("my-client-id".to_string()));
        assert_eq!(oidc_config.required_claims, vec!["email"]);
        assert_eq!(
            oidc_config.forward_claims.get("sub"),
            Some(&"X-User-ID".to_string())
        );
    }
}
