//! Hive YAML Configuration Module
//!
//! This module provides parsing and management of hive.yaml configuration files
//! according to the Hive YAML Specification v0.3.0.
//!
//! Key features:
//! - YAML configuration parsing with serde
//! - Parse-time variable interpolation (${env.VAR}, ${service.name})
//! - Runtime template resolution ({{runtime.port.X}})
//! - Plugin-based architecture for runners, environment, health checks, and rollout

mod interpolation;
mod parser;
mod types;
mod validation;

pub use interpolation::*;
pub use parser::{
    extract_blue_green_config, extract_cmd_health_config, extract_docker_config,
    extract_http_health_config, extract_recreate_config, extract_script_config,
    extract_tcp_health_config, find_project_root, get_rollout_ports, HiveConfigParser,
};
pub use types::*;
pub use validation::*;
