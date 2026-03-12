//! Header Manipulation Proxy Middleware Plugin for Hive
//!
//! Adds, removes, or modifies HTTP headers.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: headers
//!       headers:
//!         add:
//!           X-Frame-Options: DENY
//!           X-Content-Type-Options: nosniff
//!         remove:
//!           - Server
//!           - X-Powered-By
//!         set:
//!           X-Request-ID: "${uuid}"
//! ```

use lib_plugin_abi_v3::{
    async_trait,
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use std::collections::HashMap;
use tracing::debug;

pub struct HeadersPlugin {
    add_headers: HashMap<String, String>,
    remove_headers: Vec<String>,
    set_headers: HashMap<String, String>,
}

impl Default for HeadersPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadersPlugin {
    pub fn new() -> Self {
        Self {
            add_headers: HashMap::new(),
            remove_headers: vec![],
            set_headers: HashMap::new(),
        }
    }

    fn interpolate_value(&self, value: &str) -> String {
        let mut result = value.to_string();

        if result.contains("${uuid}") {
            result = result.replace("${uuid}", &uuid::Uuid::new_v4().to_string());
        }

        if result.contains("${timestamp}") {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs().to_string())
                .unwrap_or_default();
            result = result.replace("${timestamp}", &ts);
        }

        result
    }
}

#[async_trait]
impl Plugin for HeadersPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.headers".to_string(),
            name: "Headers".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: None,
            description: Some("HTTP header manipulation".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, _ctx: &PluginContext) -> PluginResult<()> {
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
impl ProxyMiddleware for HeadersPlugin {
    async fn init_middleware(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        if let Some(add) = config.get("add").and_then(|v| v.as_object()) {
            self.add_headers = add
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
        }

        if let Some(remove) = config.get("remove").and_then(|v| v.as_array()) {
            self.remove_headers = remove
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_lowercase()))
                .collect();
        }

        if let Some(set) = config.get("set").and_then(|v| v.as_object()) {
            self.set_headers = set
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect();
        }

        debug!(
            "Headers plugin initialized: add={}, remove={}, set={}",
            self.add_headers.len(),
            self.remove_headers.len(),
            self.set_headers.len()
        );

        Ok(())
    }

    async fn process_request(&self, mut req: ProxyRequest) -> PluginResult<ProxyResult> {
        for (key, value) in &self.add_headers {
            let lower_key = key.to_lowercase();
            if !req.headers.contains_key(&lower_key) {
                let interpolated = self.interpolate_value(value);
                req.headers.insert(lower_key, interpolated);
            }
        }

        for key in &self.remove_headers {
            req.headers.remove(key);
        }

        for (key, value) in &self.set_headers {
            let interpolated = self.interpolate_value(value);
            req.headers.insert(key.to_lowercase(), interpolated);
        }

        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, mut resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        for (key, value) in &self.add_headers {
            let lower_key = key.to_lowercase();
            if !resp.headers.contains_key(&lower_key) {
                let interpolated = self.interpolate_value(value);
                resp.headers.insert(lower_key, interpolated);
            }
        }

        for key in &self.remove_headers {
            resp.headers.remove(key);
        }

        for (key, value) in &self.set_headers {
            let interpolated = self.interpolate_value(value);
            resp.headers.insert(key.to_lowercase(), interpolated);
        }

        Ok(resp)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(HeadersPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_headers() {
        let mut plugin = HeadersPlugin::new();
        plugin
            .init_middleware(&serde_json::json!({
                "add": {
                    "X-Frame-Options": "DENY",
                    "X-Content-Type-Options": "nosniff"
                }
            }))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(req) => {
                assert_eq!(req.headers.get("x-frame-options"), Some(&"DENY".to_string()));
            }
            _ => panic!("Expected continue"),
        }
    }

    #[tokio::test]
    async fn test_remove_headers() {
        let mut plugin = HeadersPlugin::new();
        plugin
            .init_middleware(&serde_json::json!({
                "remove": ["server", "x-powered-by"]
            }))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: [
                ("server".to_string(), "nginx".to_string()),
                ("x-powered-by".to_string(), "PHP".to_string()),
                ("content-type".to_string(), "text/html".to_string()),
            ]
            .into_iter()
            .collect(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(req) => {
                assert!(req.headers.get("server").is_none());
                assert!(req.headers.get("x-powered-by").is_none());
                assert!(req.headers.get("content-type").is_some());
            }
            _ => panic!("Expected continue"),
        }
    }

    #[tokio::test]
    async fn test_uuid_interpolation() {
        let mut plugin = HeadersPlugin::new();
        plugin
            .init_middleware(&serde_json::json!({
                "set": {
                    "X-Request-ID": "${uuid}"
                }
            }))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            client_ip: None,
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(req) => {
                let request_id = req.headers.get("x-request-id").unwrap();
                assert!(uuid::Uuid::parse_str(request_id).is_ok());
            }
            _ => panic!("Expected continue"),
        }
    }
}
