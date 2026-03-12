//! HTTP Basic Authentication Proxy Middleware Plugin for Hive
//!
//! Validates HTTP Basic authentication credentials.
//!
//! ## Limitations
//!
//! **Hashed passwords are NOT supported.** Only plaintext passwords work currently.
//! This means htpasswd files with bcrypt (`$2...`) or Apache MD5 (`$apr1$...`) hashes
//! will fail authentication. Use plaintext passwords or implement password hashing
//! support in a fork.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: auth.basic
//!       auth.basic:
//!         realm: "Restricted"
//!         users:
//!           admin: "plaintext-password"  # Only plaintext supported
//!         # Or use a file (plaintext only):
//!         # users_file: ~/.adi/hive/htpasswd
//! ```

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

pub struct BasicAuthPlugin {
    realm: String,
    users: HashMap<String, String>,
    skip: bool,
}

impl Default for BasicAuthPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl BasicAuthPlugin {
    pub fn new() -> Self {
        Self {
            realm: "Restricted".to_string(),
            users: HashMap::new(),
            skip: false,
        }
    }

    fn extract_credentials(&self, req: &ProxyRequest) -> Option<(String, String)> {
        let auth_header = req.headers.get("authorization")?;

        if !auth_header.starts_with("Basic ") {
            return None;
        }

        let encoded = &auth_header[6..];
        let decoded = STANDARD.decode(encoded).ok()?;
        let credentials = String::from_utf8(decoded).ok()?;

        let mut parts = credentials.splitn(2, ':');
        let username = parts.next()?.to_string();
        let password = parts.next()?.to_string();

        Some((username, password))
    }

    fn verify_password(&self, username: &str, password: &str) -> bool {
        if let Some(stored) = self.users.get(username) {
            if stored == password {
                return true;
            }

            if stored.starts_with("$apr1$") || stored.starts_with("$2") || stored.starts_with("{SHA}") {
                // Hashed passwords are not supported - log once per attempt
                // To add support, consider using the bcrypt or md5-crypt crates
                warn!(
                    "Hashed password detected for user '{}' but hash verification is not implemented. \
                     Use plaintext passwords or implement hash support.",
                    username
                );
                return false;
            }
        }
        false
    }

    fn load_htpasswd(&mut self, path: &Path) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(path)?;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((username, password)) = line.split_once(':') {
                self.users
                    .insert(username.to_string(), password.to_string());
            }
        }

        debug!("Loaded {} users from htpasswd", self.users.len());
        Ok(())
    }

    fn parse_config(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        let basic_config: BasicAuthConfig = serde_json::from_value(config.clone())?;

        if let Some(realm) = basic_config.realm {
            self.realm = realm;
        }

        if let Some(skip) = basic_config.skip {
            self.skip = skip;
        }

        if let Some(ref users_file) = basic_config.users_file {
            let path = shellexpand::tilde(users_file);
            self.load_htpasswd(Path::new(path.as_ref()))
                .map_err(|e| lib_plugin_abi_v3::PluginError::Config(format!("Failed to read htpasswd file: {}", e)))?;
        }

        if let Some(users) = basic_config.users {
            for (username, password) in users {
                self.users.insert(username, password);
            }
        }

        if self.users.is_empty() && !self.skip {
            return Err(lib_plugin_abi_v3::PluginError::Config("No users configured for basic auth".to_string()));
        }

        Ok(())
    }
}

#[async_trait]
impl Plugin for BasicAuthPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.auth.basic".to_string(),
            name: "Basic Auth Middleware".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("HTTP Basic authentication".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.parse_config(&ctx.config)?;
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
impl ProxyMiddleware for BasicAuthPlugin {
    async fn init_middleware(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        self.parse_config(config)?;
        Ok(())
    }

    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        if self.skip {
            return Ok(ProxyResult::Continue(req));
        }

        let Some((username, password)) = self.extract_credentials(&req) else {
            debug!("No basic auth credentials provided");
            return Ok(ProxyResult::Response(
                ProxyResponse::new(401)
                    .with_header("www-authenticate", &format!("Basic realm=\"{}\"", self.realm))
                    .with_header("content-type", "text/plain")
                    .with_body("Authentication required"),
            ));
        };

        if self.verify_password(&username, &password) {
            req.headers
                .insert("x-authenticated-user".to_string(), username);
            Ok(ProxyResult::Continue(req))
        } else {
            debug!("Invalid credentials for user: {}", username);
            Ok(ProxyResult::Response(
                ProxyResponse::new(401)
                    .with_header("www-authenticate", &format!("Basic realm=\"{}\"", self.realm))
                    .with_header("content-type", "text/plain")
                    .with_body("Invalid credentials"),
            ))
        }
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BasicAuthConfig {
    pub realm: Option<String>,
    pub users_file: Option<String>,
    pub users: Option<HashMap<String, String>>,
    pub skip: Option<bool>,
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(BasicAuthPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_context(config: serde_json::Value) -> PluginContext {
        PluginContext::new(
            "hive.proxy.auth.basic",
            PathBuf::from("/tmp"),
            PathBuf::from("/tmp"),
            config,
        )
    }

    #[tokio::test]
    async fn test_basic_auth() {
        let mut plugin = BasicAuthPlugin::new();
        plugin
            .init(&test_context(serde_json::json!({
                "users": {
                    "admin": "password123"
                }
            })))
            .await
            .unwrap();

        // Valid credentials
        let encoded = STANDARD.encode("admin:password123");
        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: [("authorization".to_string(), format!("Basic {}", encoded))]
                .into_iter()
                .collect(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(req) => {
                assert_eq!(
                    req.headers.get("x-authenticated-user"),
                    Some(&"admin".to_string())
                );
            }
            _ => panic!("Expected continue"),
        }
    }

    #[tokio::test]
    async fn test_invalid_credentials() {
        let mut plugin = BasicAuthPlugin::new();
        plugin
            .init(&test_context(serde_json::json!({
                "users": {
                    "admin": "password123"
                }
            })))
            .await
            .unwrap();

        let encoded = STANDARD.encode("admin:wrongpassword");
        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: [("authorization".to_string(), format!("Basic {}", encoded))]
                .into_iter()
                .collect(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Response(resp) => assert_eq!(resp.status, 401),
            _ => panic!("Expected 401 response"),
        }
    }
}
