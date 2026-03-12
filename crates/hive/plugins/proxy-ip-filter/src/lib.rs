//! IP Filter Proxy Middleware Plugin for Hive
//!
//! Filters requests based on client IP address.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   plugins:
//!     - type: ip-filter
//!       ip-filter:
//!         allow:
//!           - 10.0.0.0/8
//!           - 192.168.1.0/24
//!         deny:
//!           - 1.2.3.4
//!         trust_xff: true
//! ```

use async_trait::async_trait;
use ipnetwork::IpNetwork;
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use std::net::IpAddr;
use tracing::debug;

pub struct IpFilterPlugin {
    allow_list: Vec<IpNetwork>,
    deny_list: Vec<IpNetwork>,
    trust_xff: bool,
}

impl Default for IpFilterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl IpFilterPlugin {
    pub fn new() -> Self {
        Self {
            allow_list: vec![],
            deny_list: vec![],
            trust_xff: false,
        }
    }

    fn get_client_ip(&self, req: &ProxyRequest) -> Option<IpAddr> {
        if self.trust_xff {
            if let Some(xff) = req.header("x-forwarded-for") {
                if let Some(ip_str) = xff.split(',').next() {
                    if let Ok(ip) = ip_str.trim().parse() {
                        return Some(ip);
                    }
                }
            }

            if let Some(xri) = req.header("x-real-ip") {
                if let Ok(ip) = xri.trim().parse() {
                    return Some(ip);
                }
            }
        }

        req.client_ip.as_ref().and_then(|ip| ip.parse().ok())
    }

    fn is_ip_allowed(&self, ip: &IpAddr) -> bool {
        for network in &self.deny_list {
            if network.contains(*ip) {
                debug!("IP {} denied by deny list", ip);
                return false;
            }
        }

        if self.allow_list.is_empty() {
            return true;
        }

        for network in &self.allow_list {
            if network.contains(*ip) {
                debug!("IP {} allowed by allow list", ip);
                return true;
            }
        }

        debug!("IP {} not in allow list", ip);
        false
    }
}

#[async_trait]
impl Plugin for IpFilterPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.ip-filter".to_string(),
            name: "IP Filter".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI".to_string()),
            description: Some("IP allow/deny list filter".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        if let Some(allow) = ctx.config.get("allow").and_then(|v| v.as_array()) {
            self.allow_list = allow
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse().ok())
                .collect();
        }

        if let Some(deny) = ctx.config.get("deny").and_then(|v| v.as_array()) {
            self.deny_list = deny
                .iter()
                .filter_map(|v| v.as_str())
                .filter_map(|s| s.parse().ok())
                .collect();
        }

        if let Some(trust) = ctx.config.get("trust_xff").and_then(|v| v.as_bool()) {
            self.trust_xff = trust;
        }

        debug!(
            "IP filter initialized: {} allow rules, {} deny rules, trust_xff={}",
            self.allow_list.len(),
            self.deny_list.len(),
            self.trust_xff
        );

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
impl ProxyMiddleware for IpFilterPlugin {
    async fn process_request(&self, req: ProxyRequest) -> PluginResult<ProxyResult> {
        let Some(ip) = self.get_client_ip(&req) else {
            debug!("No client IP detected, allowing request");
            return Ok(ProxyResult::Continue(req));
        };

        if self.is_ip_allowed(&ip) {
            Ok(ProxyResult::Continue(req))
        } else {
            Ok(ProxyResult::Response(ProxyResponse::forbidden()))
        }
    }

    async fn process_response(&self, resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        Ok(resp)
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(IpFilterPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_context(config: serde_json::Value) -> PluginContext {
        PluginContext::new(
            "hive.proxy.ip-filter",
            PathBuf::from("/tmp/data"),
            PathBuf::from("/tmp/config"),
            config,
        )
    }

    #[tokio::test]
    async fn test_allow_list() {
        let mut plugin = IpFilterPlugin::new();
        plugin
            .init(&make_context(serde_json::json!({
                "allow": ["10.0.0.0/8", "192.168.1.0/24"]
            })))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            client_ip: Some("10.1.2.3".to_string()),
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(_) => {}
            _ => panic!("Expected continue"),
        }
    }

    #[tokio::test]
    async fn test_deny_list() {
        let mut plugin = IpFilterPlugin::new();
        plugin
            .init(&make_context(serde_json::json!({
                "deny": ["1.2.3.4"]
            })))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: HashMap::new(),
            client_ip: Some("1.2.3.4".to_string()),
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Response(resp) => assert_eq!(resp.status, 403),
            _ => panic!("Expected forbidden response"),
        }
    }

    #[tokio::test]
    async fn test_xff_header() {
        let mut plugin = IpFilterPlugin::new();
        plugin
            .init(&make_context(serde_json::json!({
                "allow": ["10.0.0.0/8"],
                "trust_xff": true
            })))
            .await
            .unwrap();

        let req = ProxyRequest {
            method: "GET".to_string(),
            uri: "/".to_string(),
            headers: [("x-forwarded-for".to_string(), "10.1.2.3, 192.168.1.1".to_string())]
                .into_iter()
                .collect(),
            client_ip: Some("192.168.1.1".to_string()), // Proxy IP
            body: None,
        };

        match plugin.process_request(req).await.unwrap() {
            ProxyResult::Continue(_) => {}
            _ => panic!("Expected continue"),
        }
    }
}
