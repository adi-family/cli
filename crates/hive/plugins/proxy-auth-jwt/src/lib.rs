//! JWT Authentication Proxy Middleware Plugin for Hive
//!
//! Validates JWT tokens in incoming requests.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: auth.jwt
//!       auth.jwt:
//!         jwks_url: https://auth.example.com/.well-known/jwks.json
//!         header: Authorization
//!         scheme: Bearer
//!         claims:
//!           iss: https://auth.example.com
//!         forward_claims:
//!           sub: X-User-ID
//!           email: X-User-Email
//! ```

use async_trait::async_trait;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

pub struct JwtAuthPlugin {
    config: JwtConfig,
    decoding_key: Arc<RwLock<Option<DecodingKey>>>,
}

impl Default for JwtAuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtAuthPlugin {
    pub fn new() -> Self {
        Self {
            config: JwtConfig::default(),
            decoding_key: Arc::new(RwLock::new(None)),
        }
    }

    fn extract_token(&self, req: &ProxyRequest) -> Option<String> {
        let header_name = self.config.header.to_lowercase();
        let auth_header = req.headers.get(&header_name)?;

        let scheme = &self.config.scheme;
        if auth_header.starts_with(scheme) {
            Some(auth_header[scheme.len()..].trim().to_string())
        } else {
            None
        }
    }

    async fn validate_token(&self, token: &str) -> anyhow::Result<HashMap<String, serde_json::Value>> {
        let key = self.decoding_key.read().await;
        let decoding_key = key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Decoding key not initialized"))?;

        let mut validation = Validation::new(Algorithm::RS256);
        
        if let Some(ref iss) = self.config.claims.as_ref().and_then(|c| c.get("iss")) {
            if let Some(iss_str) = iss.as_str() {
                validation.set_issuer(&[iss_str]);
            }
        }

        if let Some(ref aud) = self.config.claims.as_ref().and_then(|c| c.get("aud")) {
            if let Some(aud_str) = aud.as_str() {
                validation.set_audience(&[aud_str]);
            }
        }

        let token_data = decode::<HashMap<String, serde_json::Value>>(token, decoding_key, &validation)
            .map_err(|e| anyhow::anyhow!("Token validation failed: {}", e))?;

        Ok(token_data.claims)
    }

    async fn fetch_jwks(&self, url: &str) -> anyhow::Result<()> {
        debug!("Fetching JWKS from {}", url);

        let response = reqwest::get(url)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch JWKS: {}", e))?;

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse JWKS: {}", e))?;

        let key = jwks
            .keys
            .iter()
            .find(|k| k.kty == "RSA" && k.alg.as_deref() == Some("RS256"))
            .ok_or_else(|| anyhow::anyhow!("No suitable RSA key found in JWKS"))?;

        let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)
            .map_err(|e| anyhow::anyhow!("Failed to create decoding key: {}", e))?;

        let mut guard = self.decoding_key.write().await;
        *guard = Some(decoding_key);

        debug!("JWKS loaded successfully");
        Ok(())
    }

    async fn init_with_config(&mut self, config: &Value) -> anyhow::Result<()> {
        self.config = serde_json::from_value(config.clone())
            .map_err(|e| anyhow::anyhow!("Invalid JWT config: {}", e))?;

        if let Some(ref jwks_url) = self.config.jwks_url {
            self.fetch_jwks(jwks_url).await?;
        } else if let Some(ref secret) = self.config.secret {
            let key = DecodingKey::from_secret(secret.as_bytes());
            let mut guard = self.decoding_key.write().await;
            *guard = Some(key);
        } else {
            return Err(anyhow::anyhow!("Either jwks_url or secret must be provided"));
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for JwtAuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.auth.jwt".to_string(),
            name: "JWT Auth".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI".to_string()),
            description: Some("JWT token authentication middleware".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.init_with_config(&ctx.config).await?;
        Ok(())
    }

    async fn shutdown(&self) -> PluginResult<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_PROXY_MIDDLEWARE]
    }
}

#[async_trait]
impl ProxyMiddleware for JwtAuthPlugin {
    async fn init_middleware(&mut self, config: &Value) -> PluginResult<()> {
        self.init_with_config(config).await?;
        Ok(())
    }

    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        if self.config.skip.unwrap_or(false) {
            return Ok(ProxyResult::Continue(req));
        }

        let Some(token) = self.extract_token(&req) else {
            debug!("No token found in request");
            return Ok(ProxyResult::Response(
                ProxyResponse::new(401)
                    .with_header("content-type", "application/json")
                    .with_header("www-authenticate", &format!("{} realm=\"api\"", self.config.scheme))
                    .with_body(r#"{"error": "unauthorized", "message": "Missing authentication token"}"#),
            ));
        };

        match self.validate_token(&token).await {
            Ok(claims) => {
                if let Some(ref forward_claims) = self.config.forward_claims {
                    for (claim_name, header_name) in forward_claims {
                        if let Some(value) = claims.get(claim_name) {
                            let header_value = match value {
                                serde_json::Value::String(s) => s.clone(),
                                _ => value.to_string(),
                            };
                            req.headers.insert(header_name.to_lowercase(), header_value);
                        }
                    }
                }

                Ok(ProxyResult::Continue(req))
            }
            Err(e) => {
                warn!("Token validation failed: {}", e);
                Ok(ProxyResult::Response(
                    ProxyResponse::new(401)
                        .with_header("content-type", "application/json")
                        .with_header("www-authenticate", &format!("{} realm=\"api\"", self.config.scheme))
                        .with_body(r#"{"error": "unauthorized", "message": "Invalid authentication token"}"#),
                ))
            }
        }
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JwtConfig {
    pub jwks_url: Option<String>,
    /// Shared secret (for HS256)
    pub secret: Option<String>,
    #[serde(default = "default_header")]
    pub header: String,
    #[serde(default = "default_scheme")]
    pub scheme: String,
    pub claims: Option<HashMap<String, serde_json::Value>>,
    pub forward_claims: Option<HashMap<String, String>>,
    pub skip: Option<bool>,
}

fn default_header() -> String {
    "Authorization".to_string()
}

fn default_scheme() -> String {
    "Bearer ".to_string()
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JwkKey>,
}

#[derive(Debug, Deserialize)]
struct JwkKey {
    kty: String,
    alg: Option<String>,
    n: String,
    e: String,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(JwtAuthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = JwtAuthPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.auth.jwt");
    }

    #[test]
    fn test_extract_token() {
        let plugin = JwtAuthPlugin::new();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: [("authorization".to_string(), "Bearer test-token".to_string())]
                .into_iter()
                .collect(),
            client_ip: None,
            body: None,
        };

        assert_eq!(plugin.extract_token(&req), Some("test-token".to_string()));
    }
}
