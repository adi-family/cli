use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub routes: Vec<Route>,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub name: String,
    pub path_prefix: String,
    #[serde(default)]
    pub strategy: Strategy,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub strip_prefix: bool,
    pub backends: Vec<Backend>,
}

fn default_timeout_ms() -> u64 {
    5000
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    #[default]
    First,
    All,
    Fastest,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Backend {
    pub url: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct BackendResponse {
    pub backend_url: String,
    pub status: u16,
    pub body: bytes::Bytes,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug)]
pub enum MuxResponse {
    /// Returned for `first` and `fastest` strategies.
    Single(BackendResponse),
    /// Returned for `all` strategy.
    Aggregate(Vec<BackendResponse>),
    /// No route matched the incoming path.
    NoMatch,
}
