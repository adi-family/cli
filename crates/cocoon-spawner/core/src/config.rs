use lib_env_parse::{env_opt, env_or, env_vars};
use lib_signaling_protocol::CocoonKind;
use std::time::Duration;

use crate::SpawnerError;

env_vars! {
    SignalingServerUrl => "SIGNALING_SERVER_URL",
    SpawnerId => "COCOON_SPAWNER_ID",
    SpawnerSecret => "COCOON_SPAWNER_SECRET",
    CocoonKinds => "COCOON_SPAWNER_KINDS",
    MaxConcurrentCocoons => "COCOON_SPAWNER_MAX_CONCURRENT",
    SetupTokens => "COCOON_SPAWNER_TOKENS",
    ReconnectDelaySecs => "COCOON_SPAWNER_RECONNECT_DELAY_SECS",
    HealthCheckIntervalSecs => "COCOON_SPAWNER_HEALTH_CHECK_INTERVAL_SECS",
    WebrtcIceServers => "WEBRTC_ICE_SERVERS",
    WebrtcTurnUsername => "WEBRTC_TURN_USERNAME",
    WebrtcTurnCredential => "WEBRTC_TURN_CREDENTIAL",
}

const DEFAULT_REGISTRY: &str = "docker-registry.the-ihor.com/cocoon";

/// Per-kind Docker configuration.
#[derive(Debug, Clone)]
pub struct KindConfig {
    pub kind: CocoonKind,
    pub image: String,
    pub cpu_limit: Option<i64>,
    pub memory_limit_mb: Option<i64>,
}

/// Top-level spawner configuration loaded from environment variables.
#[derive(Debug, Clone)]
pub struct SpawnerConfig {
    pub signaling_url: String,
    pub hive_id: String,
    pub hive_secret: String,
    pub kinds: Vec<KindConfig>,
    pub max_concurrent: usize,
    pub setup_tokens: Vec<String>,
    pub reconnect_delay: Duration,
    pub health_check_interval: Duration,
    pub webrtc_ice_servers: Option<String>,
    pub webrtc_turn_username: Option<String>,
    pub webrtc_turn_credential: Option<String>,
}

impl SpawnerConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self, SpawnerError> {
        let signaling_url = env_opt(EnvVar::SignalingServerUrl.as_str())
            .ok_or_else(|| SpawnerError::Config("SIGNALING_SERVER_URL is required".into()))?;

        let hive_id = env_opt(EnvVar::SpawnerId.as_str())
            .ok_or_else(|| SpawnerError::Config("COCOON_SPAWNER_ID is required".into()))?;

        let hive_secret = env_or(EnvVar::SpawnerSecret.as_str(), "");

        let kinds_str = env_or(EnvVar::CocoonKinds.as_str(), "ubuntu");
        let kinds = parse_kinds(&kinds_str);

        let max_concurrent = env_or(EnvVar::MaxConcurrentCocoons.as_str(), "10")
            .parse::<usize>()
            .map_err(|e| SpawnerError::Config(format!("invalid COCOON_SPAWNER_MAX_CONCURRENT: {e}")))?;

        let setup_tokens = env_opt(EnvVar::SetupTokens.as_str())
            .map(|s| s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect())
            .unwrap_or_default();

        let reconnect_delay_secs = env_or(EnvVar::ReconnectDelaySecs.as_str(), "5")
            .parse::<u64>()
            .map_err(|e| SpawnerError::Config(format!("invalid COCOON_SPAWNER_RECONNECT_DELAY_SECS: {e}")))?;

        let health_check_secs = env_or(EnvVar::HealthCheckIntervalSecs.as_str(), "30")
            .parse::<u64>()
            .map_err(|e| SpawnerError::Config(format!("invalid COCOON_SPAWNER_HEALTH_CHECK_INTERVAL_SECS: {e}")))?;

        let webrtc_ice_servers = env_opt(EnvVar::WebrtcIceServers.as_str());
        let webrtc_turn_username = env_opt(EnvVar::WebrtcTurnUsername.as_str());
        let webrtc_turn_credential = env_opt(EnvVar::WebrtcTurnCredential.as_str());

        Ok(Self {
            signaling_url,
            hive_id,
            hive_secret,
            kinds,
            max_concurrent,
            setup_tokens,
            reconnect_delay: Duration::from_secs(reconnect_delay_secs),
            health_check_interval: Duration::from_secs(health_check_secs),
            webrtc_ice_servers,
            webrtc_turn_username,
            webrtc_turn_credential,
        })
    }

    /// Get the `CocoonKind` list for signaling registration.
    pub fn cocoon_kinds(&self) -> Vec<CocoonKind> {
        self.kinds.iter().map(|k| k.kind.clone()).collect()
    }

    /// Find a kind config by ID.
    pub fn find_kind(&self, id: &str) -> Option<&KindConfig> {
        self.kinds.iter().find(|k| k.kind.id == id)
    }
}

/// Parse comma-separated kind IDs and load per-kind env overrides.
fn parse_kinds(kinds_str: &str) -> Vec<KindConfig> {
    kinds_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|id| {
            let upper = id.to_uppercase();
            let image_key = format!("COCOON_KIND_{upper}_IMAGE");
            let cpu_key = format!("COCOON_KIND_{upper}_CPU");
            let mem_key = format!("COCOON_KIND_{upper}_MEMORY_MB");

            let image = env_opt(&image_key)
                .unwrap_or_else(|| format!("{DEFAULT_REGISTRY}:{id}"));

            let cpu_limit = env_opt(&cpu_key)
                .and_then(|v| v.parse::<i64>().ok())
                .map(|cpus| cpus * 1_000_000_000);

            let memory_limit_mb = env_opt(&mem_key)
                .and_then(|v| v.parse::<i64>().ok());

            let runner_config = serde_json::json!({ "image": &image });

            KindConfig {
                kind: CocoonKind {
                    id: id.to_string(),
                    runner_type: "docker".to_string(),
                    runner_config,
                    image: image.clone(),
                },
                image,
                cpu_limit,
                memory_limit_mb,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_kind() {
        let kinds = parse_kinds("ubuntu");
        assert_eq!(kinds.len(), 1);
        assert_eq!(kinds[0].kind.id, "ubuntu");
        assert!(kinds[0].image.contains("cocoon:ubuntu"));
    }

    #[test]
    fn parse_multiple_kinds() {
        let kinds = parse_kinds("ubuntu, alpine, full");
        assert_eq!(kinds.len(), 3);
        assert_eq!(kinds[0].kind.id, "ubuntu");
        assert_eq!(kinds[1].kind.id, "alpine");
        assert_eq!(kinds[2].kind.id, "full");
    }

    #[test]
    fn parse_empty_string() {
        let kinds = parse_kinds("");
        assert!(kinds.is_empty());
    }
}
