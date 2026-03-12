use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::hive_config::{
    extract_blue_green_config, get_rollout_ports,
    BlueGreenPort, BlueGreenRolloutConfig, RolloutConfig,
    ROLLOUT_TYPE_BLUE_GREEN, ROLLOUT_TYPE_RECREATE,
};
use crate::service_manager::parse_duration;
use crate::service_proxy::ServiceProxyState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueGreenColor {
    Blue,
    Green,
}

impl BlueGreenColor {
    pub fn opposite(&self) -> Self {
        match self {
            BlueGreenColor::Blue => BlueGreenColor::Green,
            BlueGreenColor::Green => BlueGreenColor::Blue,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlueGreenState {
    pub active: BlueGreenColor,
    pub ports: HashMap<String, BlueGreenPort>,
    pub healthy_duration: Duration,
    pub timeout: Duration,
    pub on_failure: OnFailureAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnFailureAction {
    KeepOld,
    Abort,
}

impl Default for OnFailureAction {
    fn default() -> Self {
        Self::KeepOld
    }
}

impl BlueGreenState {
    pub fn from_config(config: &BlueGreenRolloutConfig) -> Self {
        let healthy_duration = config
            .healthy_duration
            .as_ref()
            .and_then(|s| parse_duration(s))
            .unwrap_or(Duration::from_secs(10));

        let timeout = config
            .timeout
            .as_ref()
            .and_then(|s| parse_duration(s))
            .unwrap_or(Duration::from_secs(60));

        let on_failure = match config.on_failure.as_deref() {
            Some("abort") => OnFailureAction::Abort,
            _ => OnFailureAction::KeepOld,
        };

        Self {
            active: BlueGreenColor::Blue, // always start with blue
            ports: config.ports.clone(),
            healthy_duration,
            timeout,
            on_failure,
        }
    }

    pub fn active_port(&self, name: &str) -> Option<u16> {
        self.ports.get(name).map(|p| match self.active {
            BlueGreenColor::Blue => p.blue,
            BlueGreenColor::Green => p.green,
        })
    }

    pub fn inactive_port(&self, name: &str) -> Option<u16> {
        self.ports.get(name).map(|p| match self.active {
            BlueGreenColor::Blue => p.green,
            BlueGreenColor::Green => p.blue,
        })
    }

    pub fn active_ports(&self) -> HashMap<String, u16> {
        self.ports
            .iter()
            .map(|(name, port)| {
                let p = match self.active {
                    BlueGreenColor::Blue => port.blue,
                    BlueGreenColor::Green => port.green,
                };
                (name.clone(), p)
            })
            .collect()
    }

    pub fn inactive_ports(&self) -> HashMap<String, u16> {
        self.ports
            .iter()
            .map(|(name, port)| {
                let p = match self.active {
                    BlueGreenColor::Blue => port.green,
                    BlueGreenColor::Green => port.blue,
                };
                (name.clone(), p)
            })
            .collect()
    }

    pub fn switch(&mut self) {
        self.active = self.active.opposite();
    }
}

pub struct RolloutManager {
    blue_green_states: Arc<RwLock<HashMap<String, BlueGreenState>>>,
    proxy_state: Arc<ServiceProxyState>,
}

impl RolloutManager {
    pub fn new(proxy_state: Arc<ServiceProxyState>) -> Self {
        Self {
            blue_green_states: Arc::new(RwLock::new(HashMap::new())),
            proxy_state,
        }
    }

    pub async fn get_ports(&self, service_name: &str, rollout: &RolloutConfig) -> Result<HashMap<String, u16>> {
        match rollout.rollout_type.as_str() {
            ROLLOUT_TYPE_RECREATE => {
                get_rollout_ports(rollout)
            }
            ROLLOUT_TYPE_BLUE_GREEN => {
                let config = extract_blue_green_config(rollout)?;
                let states = self.blue_green_states.read().await;

                if let Some(state) = states.get(service_name) {
                    Ok(state.active_ports())
                } else {
                    // First deployment — use blue ports
                    let mut ports = HashMap::new();
                    for (name, port) in &config.ports {
                        ports.insert(name.clone(), port.blue);
                    }
                    Ok(ports)
                }
            }
            other => Err(anyhow!("Unknown rollout type: {}", other)),
        }
    }

    pub async fn init_blue_green(&self, service_name: &str, rollout: &RolloutConfig) -> Result<BlueGreenState> {
        let config = extract_blue_green_config(rollout)?;
        let state = BlueGreenState::from_config(&config);

        let mut states = self.blue_green_states.write().await;
        states.insert(service_name.to_string(), state.clone());

        Ok(state)
    }

    pub async fn get_deployment_ports(&self, service_name: &str, rollout: &RolloutConfig) -> Result<HashMap<String, u16>> {
        match rollout.rollout_type.as_str() {
            ROLLOUT_TYPE_RECREATE => self.get_ports(service_name, rollout).await,
            ROLLOUT_TYPE_BLUE_GREEN => {
                let states = self.blue_green_states.read().await;

                if let Some(state) = states.get(service_name) {
                    // New instance goes on inactive ports
                    Ok(state.inactive_ports())
                } else {
                    // First deployment — use blue ports
                    self.get_ports(service_name, rollout).await
                }
            }
            other => Err(anyhow!("Unknown rollout type: {}", other)),
        }
    }

    pub async fn switch_blue_green(&self, service_name: &str) -> Result<()> {
        let mut states = self.blue_green_states.write().await;

        if let Some(state) = states.get_mut(service_name) {
            let old_color = state.active;
            let new_color = old_color.opposite();

            info!(
                "Switching {} from {:?} to {:?}",
                service_name, old_color, new_color
            );

            let new_ports = state.inactive_ports();

            for (port_name, port) in &new_ports {
                debug!(
                    "Updating proxy for {}:{} to port {}",
                    service_name, port_name, port
                );
            }

            // Proxy uses the http port by default
            if let Some(&http_port) = new_ports.get("http") {
                self.proxy_state.update_service_port(service_name, http_port);
            }

            state.switch();

            info!(
                "Blue-green switch complete for {}: now {:?}",
                service_name, new_color
            );

            Ok(())
        } else {
            Err(anyhow!("No blue-green state for service: {}", service_name))
        }
    }

    pub async fn get_blue_green_state(&self, service_name: &str) -> Option<BlueGreenState> {
        let states = self.blue_green_states.read().await;
        states.get(service_name).cloned()
    }

    pub async fn is_blue_green(&self, service_name: &str) -> bool {
        let states = self.blue_green_states.read().await;
        states.contains_key(service_name)
    }

    pub async fn handle_failure(&self, service_name: &str) -> Result<OnFailureAction> {
        let states = self.blue_green_states.read().await;

        if let Some(state) = states.get(service_name) {
            let action = state.on_failure;

            match action {
                OnFailureAction::KeepOld => {
                    warn!(
                        "Blue-green deployment failed for {}, keeping old instance",
                        service_name
                    );
                }
                OnFailureAction::Abort => {
                    error!(
                        "Blue-green deployment failed for {}, aborting",
                        service_name
                    );
                }
            }

            Ok(action)
        } else {
            Err(anyhow!("No blue-green state for service: {}", service_name))
        }
    }
}

pub struct BlueGreenDeployment {
    state: BlueGreenState,
    start_time: std::time::Instant,
}

impl BlueGreenDeployment {
    pub fn new(_service_name: &str, state: BlueGreenState) -> Self {
        Self {
            state,
            start_time: std::time::Instant::now(),
        }
    }

    pub fn new_instance_ports(&self) -> HashMap<String, u16> {
        self.state.inactive_ports()
    }

    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.state.timeout
    }

    pub fn remaining_timeout(&self) -> Duration {
        self.state.timeout.saturating_sub(self.start_time.elapsed())
    }

    pub async fn wait_healthy_duration(&self) {
        tokio::time::sleep(self.state.healthy_duration).await;
    }

    pub fn healthy_duration(&self) -> Duration {
        self.state.healthy_duration
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blue_green_color() {
        assert_eq!(BlueGreenColor::Blue.opposite(), BlueGreenColor::Green);
        assert_eq!(BlueGreenColor::Green.opposite(), BlueGreenColor::Blue);
    }

    #[test]
    fn test_blue_green_state() {
        let mut ports = HashMap::new();
        ports.insert(
            "http".to_string(),
            BlueGreenPort {
                blue: 8080,
                green: 8081,
            },
        );

        let config = BlueGreenRolloutConfig {
            ports,
            healthy_duration: Some("5s".to_string()),
            timeout: Some("30s".to_string()),
            on_failure: Some("keep-old".to_string()),
        };

        let mut state = BlueGreenState::from_config(&config);

        assert_eq!(state.active, BlueGreenColor::Blue);
        assert_eq!(state.active_port("http"), Some(8080));
        assert_eq!(state.inactive_port("http"), Some(8081));

        state.switch();
        assert_eq!(state.active, BlueGreenColor::Green);
        assert_eq!(state.active_port("http"), Some(8081));
        assert_eq!(state.inactive_port("http"), Some(8080));
    }

    #[test]
    fn test_on_failure_action() {
        let mut ports = HashMap::new();
        ports.insert(
            "http".to_string(),
            BlueGreenPort {
                blue: 8080,
                green: 8081,
            },
        );

        let config = BlueGreenRolloutConfig {
            ports: ports.clone(),
            healthy_duration: None,
            timeout: None,
            on_failure: Some("abort".to_string()),
        };
        let state = BlueGreenState::from_config(&config);
        assert_eq!(state.on_failure, OnFailureAction::Abort);

        let config = BlueGreenRolloutConfig {
            ports,
            healthy_duration: None,
            timeout: None,
            on_failure: Some("keep-old".to_string()),
        };
        let state = BlueGreenState::from_config(&config);
        assert_eq!(state.on_failure, OnFailureAction::KeepOld);
    }
}
