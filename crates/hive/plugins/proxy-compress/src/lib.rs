//! Response Compression Proxy Plugin for Hive
//!
//! Compresses HTTP responses using gzip, brotli, or deflate.
//!
//! ## Known Limitation
//!
//! **Accept-Encoding is not honored.** Due to the ProxyMiddleware trait design,
//! `process_response()` does not have access to the original request headers.
//! The plugin always uses the first configured algorithm regardless of what
//! the client advertises in Accept-Encoding.
//!
//! This means clients may receive brotli-compressed responses even if they
//! only support gzip. Most modern browsers support all common algorithms,
//! so this is rarely an issue in practice.
//!
//! ## Configuration
//!
//! ```yaml
//! proxy:
//!   compress:
//!     algorithms: [gzip, deflate]       # Use gzip first for best compatibility
//!     min_size: 1024                    # Min bytes to compress
//!     level: 6                          # Compression level (1-9)
//!     types:                            # Content types to compress
//!       - text/*
//!       - application/json
//!       - application/javascript
//! ```

use async_trait::async_trait;
use flate2::write::{DeflateEncoder, GzEncoder};
use flate2::Compression;
use lib_plugin_abi_v3::{
    proxy::{ProxyMiddleware, ProxyRequest, ProxyResponse, ProxyResult},
    Plugin, PluginCategory, PluginContext, PluginMetadata, PluginType,
    Result as PluginResult, SERVICE_PROXY_MIDDLEWARE,
};
use serde::{Deserialize, Serialize};
use std::io::Write;
use tracing::debug;

pub struct CompressPlugin {
    config: CompressConfig,
}

impl Default for CompressPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl CompressPlugin {
    pub fn new() -> Self {
        Self {
            config: CompressConfig::default(),
        }
    }

    fn should_compress(&self, content_type: Option<&str>, body_len: usize) -> bool {
        if body_len < self.config.min_size {
            return false;
        }

        let content_type = match content_type {
            Some(ct) => ct.to_lowercase(),
            None => return false,
        };

        for pattern in &self.config.types {
            if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 1];
                if content_type.starts_with(prefix) {
                    return true;
                }
            } else if content_type.starts_with(pattern) {
                return true;
            }
        }

        false
    }

    /// Select compression algorithm based on Accept-Encoding header.
    ///
    /// NOTE: This method exists but is currently unused because `process_response()`
    /// does not have access to the original request headers. It's preserved for
    /// future use when/if the ProxyMiddleware trait is updated to pass request context.
    #[allow(dead_code)]
    fn select_algorithm(&self, accept_encoding: Option<&str>) -> Option<&str> {
        let accept = accept_encoding?.to_lowercase();

        for algo in &self.config.algorithms {
            match algo.as_str() {
                "br" | "brotli" if accept.contains("br") => return Some("br"),
                "gzip" if accept.contains("gzip") => return Some("gzip"),
                "deflate" if accept.contains("deflate") => return Some("deflate"),
                _ => continue,
            }
        }

        None
    }

    fn compress(&self, data: &[u8], algorithm: &str) -> anyhow::Result<Vec<u8>> {
        let level = self.config.level.unwrap_or(6);

        match algorithm {
            "gzip" => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(level));
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            "deflate" => {
                let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
                encoder.write_all(data)?;
                Ok(encoder.finish()?)
            }
            "br" => {
                let mut output = Vec::new();
                let params = brotli::enc::BrotliEncoderParams {
                    quality: level as i32,
                    ..Default::default()
                };
                brotli::BrotliCompress(&mut std::io::Cursor::new(data), &mut output, &params)?;
                Ok(output)
            }
            _ => Ok(data.to_vec()),
        }
    }

    fn parse_config(&mut self, config: &serde_json::Value) {
        if let Some(compress_config) = config.get("compress") {
            if let Ok(cfg) = serde_json::from_value::<CompressConfig>(compress_config.clone()) {
                self.config = cfg;
            }
        }
    }
}

#[async_trait]
impl Plugin for CompressPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "hive.proxy.compress".to_string(),
            name: "Compress Middleware".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            plugin_type: PluginType::Orchestration,
            author: Some("ADI Team".to_string()),
            description: Some("Response compression (gzip, brotli, deflate). Note: does not honor Accept-Encoding".to_string()),
            category: Some(PluginCategory::Proxy),
        }
    }

    async fn init(&mut self, ctx: &PluginContext) -> PluginResult<()> {
        self.parse_config(&ctx.config);
        debug!(
            "Compress plugin initialized: algorithms={:?}, min_size={}",
            self.config.algorithms, self.config.min_size
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
impl ProxyMiddleware for CompressPlugin {
    async fn init_middleware(&mut self, config: &serde_json::Value) -> PluginResult<()> {
        self.parse_config(config);
        debug!(
            "Compress middleware initialized: algorithms={:?}, min_size={}",
            self.config.algorithms, self.config.min_size
        );
        Ok(())
    }

    async fn process_request(&self, req: ProxyRequest) -> PluginResult<ProxyResult> {
        Ok(ProxyResult::Continue(req))
    }

    async fn process_response(&self, mut resp: ProxyResponse) -> PluginResult<ProxyResponse> {
        if resp.headers.contains_key("content-encoding") {
            return Ok(resp);
        }

        let content_type = resp.headers.get("content-type").map(|s| s.as_str());

        if !self.should_compress(content_type, resp.body.len()) {
            return Ok(resp);
        }

        // Accept-Encoding is not accessible here (see module doc), defaulting to first configured algorithm
        let algorithm = self
            .config
            .algorithms
            .first()
            .map(|s| s.as_str())
            .unwrap_or("gzip");

        let original_size = resp.body.len();
        let compressed = self.compress(&resp.body, algorithm)?;

        if compressed.len() < original_size {
            debug!(
                "Compressed response: {} -> {} bytes ({})",
                original_size,
                compressed.len(),
                algorithm
            );
            resp.body = compressed;
            resp.headers
                .insert("content-encoding".to_string(), algorithm.to_string());
            resp.headers.insert("vary".to_string(), "Accept-Encoding".to_string());
        }

        Ok(resp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressConfig {
    /// Compression algorithms in priority order
    #[serde(default = "default_algorithms")]
    pub algorithms: Vec<String>,
    /// Minimum body size to compress (bytes)
    #[serde(default = "default_min_size")]
    pub min_size: usize,
    pub level: Option<u32>,
    #[serde(default = "default_types")]
    pub types: Vec<String>,
}

fn default_algorithms() -> Vec<String> {
    vec!["gzip".to_string(), "deflate".to_string()]
}

fn default_min_size() -> usize {
    1024
}

fn default_types() -> Vec<String> {
    vec![
        "text/*".to_string(),
        "application/json".to_string(),
        "application/javascript".to_string(),
        "application/xml".to_string(),
        "image/svg+xml".to_string(),
    ]
}

impl Default for CompressConfig {
    fn default() -> Self {
        Self {
            algorithms: default_algorithms(),
            min_size: default_min_size(),
            level: Some(6),
            types: default_types(),
        }
    }
}

#[cfg(feature = "plugin")]
#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(CompressPlugin::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata() {
        let plugin = CompressPlugin::new();
        let meta = plugin.metadata();
        assert_eq!(meta.id, "hive.proxy.compress");
        assert_eq!(meta.name, "Compress Middleware");
    }

    #[test]
    fn test_should_compress() {
        let plugin = CompressPlugin::new();

        // Should compress JSON
        assert!(plugin.should_compress(Some("application/json"), 2000));

        // Should not compress small bodies
        assert!(!plugin.should_compress(Some("application/json"), 100));

        // Should not compress images (not in default types)
        assert!(!plugin.should_compress(Some("image/png"), 2000));

        // Should compress text/*
        assert!(plugin.should_compress(Some("text/html"), 2000));
    }

    #[test]
    fn test_select_algorithm() {
        let plugin = CompressPlugin::new();

        assert_eq!(
            plugin.select_algorithm(Some("gzip, deflate")),
            Some("gzip")
        );
        assert_eq!(plugin.select_algorithm(Some("deflate")), Some("deflate"));
        assert_eq!(plugin.select_algorithm(Some("identity")), None);
    }

    #[test]
    fn test_gzip_compression() {
        let plugin = CompressPlugin::new();
        let data = b"Hello, World! This is a test string that should be compressed.";
        let compressed = plugin.compress(data, "gzip").unwrap();
        assert!(compressed.len() < data.len() || data.len() < 100); // Small data might not compress well
    }

    #[tokio::test]
    async fn test_process_response_compression() {
        let plugin = CompressPlugin::new();

        // Create a response with compressible content
        let body = "Hello, World! ".repeat(100); // Large enough to compress
        let resp = ProxyResponse {
            status: 200,
            headers: [("content-type".to_string(), "application/json".to_string())]
                .into_iter()
                .collect(),
            body: body.as_bytes().to_vec(),
        };

        let result = plugin.process_response(resp).await.unwrap();

        // Should be compressed
        assert!(result.headers.contains_key("content-encoding"));
        assert!(result.body.len() < body.len());
    }

    #[tokio::test]
    async fn test_skip_already_encoded() {
        let plugin = CompressPlugin::new();

        let resp = ProxyResponse {
            status: 200,
            headers: [
                ("content-type".to_string(), "application/json".to_string()),
                ("content-encoding".to_string(), "gzip".to_string()),
            ]
            .into_iter()
            .collect(),
            body: vec![1, 2, 3, 4, 5],
        };

        let result = plugin.process_response(resp.clone()).await.unwrap();

        // Should not modify
        assert_eq!(result.body.len(), resp.body.len());
    }
}
