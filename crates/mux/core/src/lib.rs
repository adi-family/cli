pub mod error;
pub mod forward;
pub mod types;

pub use error::{Error, Result};
pub use types::{Backend, BackendResponse, Config, MuxResponse, Route, Strategy};

use bytes::Bytes;
use reqwest::Client;
use std::path::{Path, PathBuf};
use std::time::Duration;

const DEFAULT_CONFIG_SUBPATH: &str = ".adi/mux/config.toml";

pub struct MuxManager {
    config: Config,
    client: Client,
}

impl MuxManager {
    /// Load config from the given path and build the manager.
    pub fn load(config_path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(config_path).map_err(|e| {
            Error::Config(format!("Cannot read {}: {e}", config_path.display()))
        })?;

        let config: Config = toml::from_str(&contents)
            .map_err(|e| Error::Config(format!("TOML parse error: {e}")))?;

        Ok(Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| Error::Config(format!("HTTP client error: {e}")))?,
        })
    }

    /// Load from `MUX_CONFIG` env var or the default `~/.adi/mux/config.toml`.
    pub fn load_from_env() -> Result<Self> {
        let path = lib_env_parse::env_opt("MUX_CONFIG")
            .map(PathBuf::from)
            .or_else(|| {
                dirs::home_dir().map(|h| h.join(DEFAULT_CONFIG_SUBPATH))
            })
            .ok_or_else(|| Error::Config("Cannot determine config path".to_string()))?;

        Self::load(&path)
    }

    /// Dispatch the request to the matching route.
    pub async fn handle(
        &self,
        method: &str,
        path: &str,
        headers: Vec<(String, String)>,
        body: Bytes,
    ) -> Result<MuxResponse> {
        let route = match self.find_route(path) {
            Some(r) => r,
            None => return Ok(MuxResponse::NoMatch),
        };

        let enabled: Vec<_> = route.backends.iter().filter(|b| b.enabled).collect();
        if enabled.is_empty() {
            return Err(Error::NoBackends(route.name.clone()));
        }

        let fwd_path = if route.strip_prefix {
            path.strip_prefix(&route.path_prefix).unwrap_or(path)
        } else {
            path
        };

        let response = match route.strategy {
            Strategy::All => {
                let responses = forward::forward_all(
                    &self.client,
                    &route.backends,
                    method,
                    fwd_path,
                    &headers,
                    body,
                    route.timeout_ms,
                )
                .await;
                MuxResponse::Aggregate(responses)
            }
            Strategy::First => {
                match forward::forward_first(
                    &self.client,
                    &route.backends,
                    method,
                    fwd_path,
                    &headers,
                    body,
                    route.timeout_ms,
                )
                .await
                {
                    Some(r) => MuxResponse::Single(r),
                    None => MuxResponse::Aggregate(vec![]),
                }
            }
            Strategy::Fastest => {
                match forward::forward_fastest(
                    &self.client,
                    &route.backends,
                    method,
                    fwd_path,
                    &headers,
                    body,
                    route.timeout_ms,
                )
                .await
                {
                    Some(r) => MuxResponse::Single(r),
                    None => MuxResponse::Aggregate(vec![]),
                }
            }
        };

        Ok(response)
    }

    /// Return the route whose `path_prefix` is the longest match for `path`.
    fn find_route(&self, path: &str) -> Option<&Route> {
        self.config
            .routes
            .iter()
            .filter(|r| path.starts_with(&r.path_prefix))
            .max_by_key(|r| r.path_prefix.len())
    }
}
