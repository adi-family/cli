//! CORS Proxy Middleware Plugin for Hive
//!
//! Handles Cross-Origin Resource Sharing (CORS) for proxied requests.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: cors
//!       cors:
//!         origins: ["https://example.com"]
//!         methods: ["GET", "POST", "PUT", "DELETE"]
//!         headers: ["Content-Type", "Authorization"]
//!         credentials: true
//!         max_age: 86400
//! ```

use async_trait::async_trait;
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};

pub struct CorsPlugin {
    allowed_origins: Vec<String>,
    allowed_methods: Vec<String>,
    allowed_headers: Vec<String>,
    exposed_headers: Vec<String>,
    allow_credentials: bool,
    max_age: u32,
}

impl Default for CorsPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CorsPlugin {
    pub fn new() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["Content-Type".to_string(), "Authorization".to_string()],
            exposed_headers: vec![],
            allow_credentials: false,
            max_age: 86400, // 24 hours
        }
    }

    fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins
            .iter()
            .any(|o| o == "*" || o == origin)
    }

    fn parse_config(&mut self, config: &serde_json::Value) {
        if let Some(origins) = config.get("origins").and_then(|v| v.as_array()) {
            self.allowed_origins = origins
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(methods) = config.get("methods").and_then(|v| v.as_array()) {
            self.allowed_methods = methods
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_uppercase()))
                .collect();
        }

        if let Some(headers) = config.get("headers").and_then(|v| v.as_array()) {
            self.allowed_headers = headers
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(expose) = config.get("expose_headers").and_then(|v| v.as_array()) {
            self.exposed_headers = expose
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }

        if let Some(creds) = config.get("credentials").and_then(|v| v.as_bool()) {
            self.allow_credentials = creds;
        }

        if let Some(age) = config.get("max_age").and_then(|v| v.as_u64()) {
            self.max_age = age as u32;
        }
    }
}

#[async_trait]
impl Plugin for CorsPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.cors".to_string(),
            name: "CORS Middleware".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Cross-Origin Resource Sharing middleware".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.parse_config(&ctx.config);
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
impl ProxyMiddleware for CorsPlugin {
    async fn init_middleware(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        self.parse_config(config);
        Ok(())
    }

    async fn process_request(&self, req: ProxyRequest) -> PluginResult<ProxyResult> {
        let origin = req.header("origin").map(String::from);

        if req.method == "OPTIONS" {
            if let Some(ref origin) = origin {
                if self.is_origin_allowed(origin) {
                    let mut response = ProxyResponse::no_content();

                    response.headers.insert(
                        "access-control-allow-origin".to_string(),
                        origin.clone(),
                    );
                    response.headers.insert(
                        "access-control-allow-methods".to_string(),
                        self.allowed_methods.join(", "),
                    );
                    response.headers.insert(
                        "access-control-allow-headers".to_string(),
                        self.allowed_headers.join(", "),
                    );
                    response.headers.insert(
                        "access-control-max-age".to_string(),
                        self.max_age.to_string(),
                    );

                    if self.allow_credentials {
                        response.headers.insert(
                            "access-control-allow-credentials".to_string(),
                            "true".to_string(),
                        );
                    }

                    return Ok(ProxyResult::Response(response));
                }
            }
        }

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, mut resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        // Note: origin from the request is not tracked here; uses configured allowed origin
        if !self.allowed_origins.is_empty() {
            let origin = if self.allowed_origins.iter().any(|o| o == "*") {
                "*".to_string()
            } else {
                self.allowed_origins.first().cloned().unwrap_or_default()
            };

            resp.headers
                .insert("access-control-allow-origin".to_string(), origin);

            if self.allow_credentials {
                resp.headers.insert(
                    "access-control-allow-credentials".to_string(),
                    "true".to_string(),
                );
            }

            if !self.exposed_headers.is_empty() {
                resp.headers.insert(
                    "access-control-expose-headers".to_string(),
                    self.exposed_headers.join(", "),
                );
            }
        }

        Ok(resp)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CorsPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cors_preflight() {
        let mut plugin = CorsPlugin::new();
        plugin.parse_config(&serde_json::json!({
            "origins": ["https://example.com"],
            "credentials": true
        }));

        let req = ProxyRequest {
            method: "OPTIONS".to_string(),
            uri: "/api/test".to_string(),
            headers: [("origin".to_string(), "https://example.com".to_string())]
                .into_iter()
                .collect(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Response(resp) => {
                assert_eq!(resp.status, 204);
                assert_eq!(
                    resp.headers.get("access-control-allow-origin"),
                    Some(&"https://example.com".to_string())
                );
            }
            _ => panic!("Expected response"),
        }
    }

    #[test]
    fn test_origin_allowed() {
        let plugin = CorsPlugin::new();
        assert!(plugin.is_origin_allowed("https://anything.com")); // * allows all

        let mut specific = CorsPlugin::new();
        specific.allowed_origins = vec!["https://example.com".to_string()];
        assert!(specific.is_origin_allowed("https://example.com"));
        assert!(!specific.is_origin_allowed("https://other.com"));
    }
}
