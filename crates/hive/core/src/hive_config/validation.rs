//! Validates hive.yaml configuration for correctness and consistency.

use anyhow::{anyhow, Result};
use std::collections::{HashMap, HashSet};
use tracing::{debug, trace, warn};

use super::types::*;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

pub struct ValidationResult {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationError>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn add_error(&mut self, path: &str, message: &str) {
        self.errors.push(ValidationError {
            path: path.to_string(),
            message: message.to_string(),
        });
    }

    pub fn add_warning(&mut self, path: &str, message: &str) {
        self.warnings.push(ValidationError {
            path: path.to_string(),
            message: message.to_string(),
        });
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_config(config: &HiveConfig) -> ValidationResult {
    debug!(
        services = config.services.len(),
        version = %config.version,
        "Validating hive configuration"
    );
    let mut result = ValidationResult::new();

    if config.version != "1" {
        result.add_error(
            "version",
            &format!("Unsupported version: {}. Expected \"1\"", config.version),
        );
    }

    let service_names: HashSet<_> = config.services.keys().collect();

    for (name, service) in &config.services {
        trace!(service = %name, "Validating service");
        validate_service_name(name, &mut result);
        validate_service(name, service, &service_names, &mut result);
    }

    if let Err(e) = check_circular_dependencies(config) {
        result.add_error("services", &e.to_string());
    }

    check_port_conflicts(config, &mut result);
    check_route_conflicts(config, &mut result);
    check_expose_conflicts(config, &mut result);

    if result.is_valid() {
        debug!("Configuration validation passed");
    } else {
        warn!(
            errors = result.errors.len(),
            warnings = result.warnings.len(),
            "Configuration validation found issues"
        );
        for err in &result.errors {
            debug!(path = %err.path, message = %err.message, "Validation error");
        }
        for w in &result.warnings {
            debug!(path = %w.path, message = %w.message, "Validation warning");
        }
    }

    result
}

fn validate_service_name(name: &str, result: &mut ValidationResult) {
    if name.is_empty() {
        result.add_error("services", "Service name cannot be empty");
        return;
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() {
        result.add_error(
            &format!("services.{}", name),
            "Service name must start with a lowercase letter",
        );
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' && c != '_' {
            result.add_error(
                &format!("services.{}", name),
                "Service name must contain only lowercase letters, numbers, hyphens, and underscores",
            );
            break;
        }
    }
}

fn validate_service(
    name: &str,
    service: &ServiceConfig,
    all_services: &HashSet<&String>,
    result: &mut ValidationResult,
) {
    let path = format!("services.{}", name);

    validate_runner(&format!("{}.runner", path), &service.runner, result);

    if service.proxy.is_some() || service.healthcheck.is_some() {
        if service.rollout.is_none() {
            result.add_error(
                &path,
                "rollout is required when proxy or healthcheck is configured",
            );
        }
    }

    if let Some(rollout) = &service.rollout {
        validate_rollout(&format!("{}.rollout", path), rollout, result);
    }

    if let Some(proxy) = &service.proxy {
        validate_proxy(&format!("{}.proxy", path), proxy, result);
    }

    if let Some(healthcheck) = &service.healthcheck {
        validate_healthcheck(&format!("{}.healthcheck", path), healthcheck, result);
    }

    for dep in &service.depends_on {
        if !all_services.contains(dep) {
            result.add_error(
                &format!("{}.depends_on", path),
                &format!("Unknown dependency: {}", dep),
            );
        }
        if dep == name {
            result.add_error(
                &format!("{}.depends_on", path),
                "Service cannot depend on itself",
            );
        }
    }

    if let Some(expose) = &service.expose {
        validate_expose(&format!("{}.expose", path), expose, result);
    }

    for (i, uses) in service.uses.iter().enumerate() {
        validate_uses(&format!("{}.uses[{}]", path, i), uses, result);
    }
}

fn validate_runner(path: &str, runner: &RunnerConfig, result: &mut ValidationResult) {
    if runner.runner_type.is_empty() {
        result.add_error(&format!("{}.type", path), "Runner type is required");
        return;
    }

    match runner.runner_type.as_str() {
        "script" => {
            if !runner.config.contains_key("script") {
                result.add_error(path, "Missing 'script' configuration for script runner");
            } else if let Some(script) = runner.config.get("script") {
                if script.get("run").is_none() {
                    result.add_error(&format!("{}.script.run", path), "'run' is required");
                }
            }
        }
        "docker" => {
            if !runner.config.contains_key("docker") {
                result.add_error(path, "Missing 'docker' configuration for docker runner");
            } else if let Some(docker) = runner.config.get("docker") {
                if docker.get("image").is_none() {
                    result.add_error(&format!("{}.docker.image", path), "'image' is required");
                }
            }
        }
        "compose" => {
            if !runner.config.contains_key("compose") {
                result.add_error(path, "Missing 'compose' configuration for compose runner");
            }
        }
        _ => {
            // External plugin - just warn if config key doesn't exist
            if !runner.config.contains_key(&runner.runner_type) {
                result.add_warning(
                    path,
                    &format!(
                        "Missing '{}' configuration for {} runner (may be auto-installed)",
                        runner.runner_type, runner.runner_type
                    ),
                );
            }
        }
    }
}

fn validate_rollout(path: &str, rollout: &RolloutConfig, result: &mut ValidationResult) {
    if rollout.rollout_type.is_empty() {
        result.add_error(&format!("{}.type", path), "Rollout type is required");
        return;
    }

    match rollout.rollout_type.as_str() {
        ROLLOUT_TYPE_RECREATE => {
            if !rollout.config.contains_key(ROLLOUT_TYPE_RECREATE) {
                result.add_error(path, "Missing 'recreate' configuration");
            } else if let Some(recreate) = rollout.config.get(ROLLOUT_TYPE_RECREATE) {
                if recreate.get("ports").is_none() {
                    result.add_error(&format!("{}.recreate.ports", path), "'ports' is required");
                }
            }
        }
        ROLLOUT_TYPE_BLUE_GREEN => {
            if !rollout.config.contains_key(ROLLOUT_TYPE_BLUE_GREEN) {
                result.add_error(path, "Missing 'blue-green' configuration");
            } else if let Some(bg) = rollout.config.get(ROLLOUT_TYPE_BLUE_GREEN) {
                if bg.get("ports").is_none() {
                    result.add_error(&format!("{}.blue-green.ports", path), "'ports' is required");
                }
            }
        }
        _ => {
            // External plugin
            if !rollout.config.contains_key(&rollout.rollout_type) {
                result.add_warning(
                    path,
                    &format!(
                        "Missing '{}' configuration for {} rollout",
                        rollout.rollout_type, rollout.rollout_type
                    ),
                );
            }
        }
    }
}

fn validate_proxy(path: &str, proxy: &ServiceProxyConfig, result: &mut ValidationResult) {
    for (i, endpoint) in proxy.endpoints().iter().enumerate() {
        let ep_path = if matches!(proxy, ServiceProxyConfig::Multiple(_)) {
            format!("{}[{}]", path, i)
        } else {
            path.to_string()
        };

        if endpoint.path.is_empty() {
            result.add_error(&format!("{}.path", ep_path), "Path is required");
        } else if !endpoint.path.starts_with('/') {
            result.add_error(&format!("{}.path", ep_path), "Path must start with '/'");
        }

        if let Some(host) = &endpoint.host {
            if host.starts_with("http://") || host.starts_with("https://") {
                result.add_error(
                    &format!("{}.host", ep_path),
                    "Host must not include protocol (no http:// or https://)",
                );
            }
        }
    }
}

fn validate_healthcheck(
    path: &str,
    healthcheck: &HealthCheckConfig,
    result: &mut ValidationResult,
) {
    for (i, check) in healthcheck.checks().iter().enumerate() {
        let check_path = if matches!(healthcheck, HealthCheckConfig::Multiple(_)) {
            format!("{}[{}]", path, i)
        } else {
            path.to_string()
        };

        if check.check_type.is_empty() {
            result.add_error(
                &format!("{}.type", check_path),
                "Health check type is required",
            );
            continue;
        }

        match check.check_type.as_str() {
            "http" => {
                if let Some(http) = check.config.get("http") {
                    if http.get("port").is_none() {
                        result
                            .add_error(&format!("{}.http.port", check_path), "'port' is required");
                    }
                } else {
                    result.add_error(&check_path, "Missing 'http' configuration");
                }
            }
            "tcp" => {
                if let Some(tcp) = check.config.get("tcp") {
                    if tcp.get("port").is_none() {
                        result.add_error(&format!("{}.tcp.port", check_path), "'port' is required");
                    }
                } else {
                    result.add_error(&check_path, "Missing 'tcp' configuration");
                }
            }
            "cmd" => {
                if let Some(cmd) = check.config.get("cmd") {
                    if cmd.get("command").is_none() {
                        result.add_error(
                            &format!("{}.cmd.command", check_path),
                            "'command' is required",
                        );
                    }
                } else {
                    result.add_error(&check_path, "Missing 'cmd' configuration");
                }
            }
            _ => {
                // External plugin
                if !check.config.contains_key(&check.check_type) {
                    result.add_warning(
                        &check_path,
                        &format!(
                            "Missing '{}' configuration for {} health check",
                            check.check_type, check.check_type
                        ),
                    );
                }
            }
        }
    }
}

fn validate_expose(path: &str, expose: &ExposeConfig, result: &mut ValidationResult) {
    if expose.name.is_empty() {
        result.add_error(&format!("{}.name", path), "Expose name is required");
    }

    if expose.vars.is_empty() {
        result.add_error(
            &format!("{}.vars", path),
            "At least one variable is required",
        );
    }
}

fn validate_uses(path: &str, uses: &UsesConfig, result: &mut ValidationResult) {
    if uses.name.is_empty() {
        result.add_error(&format!("{}.name", path), "Uses name is required");
    }
}

pub fn check_circular_dependencies(config: &HiveConfig) -> Result<()> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for (name, service) in &config.services {
        let deps: Vec<&str> = service.depends_on.iter().map(|s| s.as_str()).collect();
        trace!(service = %name, deps = ?deps, "Building dependency graph");
        graph.insert(name.as_str(), deps);
    }

    let mut visited = HashSet::new();
    let mut rec_stack = HashSet::new();

    for name in config.services.keys() {
        if has_cycle(&graph, name, &mut visited, &mut rec_stack)? {
            warn!(service = %name, "Circular dependency detected");
            return Err(anyhow!(
                "Circular dependency detected involving service: {}",
                name
            ));
        }
    }

    debug!("No circular dependencies found");
    Ok(())
}

fn has_cycle<'a>(
    graph: &HashMap<&'a str, Vec<&'a str>>,
    node: &'a str,
    visited: &mut HashSet<&'a str>,
    rec_stack: &mut HashSet<&'a str>,
) -> Result<bool> {
    if rec_stack.contains(node) {
        return Ok(true);
    }

    if visited.contains(node) {
        return Ok(false);
    }

    visited.insert(node);
    rec_stack.insert(node);

    if let Some(deps) = graph.get(node) {
        for dep in deps {
            if has_cycle(graph, dep, visited, rec_stack)? {
                return Ok(true);
            }
        }
    }

    rec_stack.remove(node);
    Ok(false)
}

fn check_port_conflicts(config: &HiveConfig, result: &mut ValidationResult) {
    let mut port_usage: HashMap<u16, Vec<String>> = HashMap::new();

    for (name, service) in &config.services {
        if let Some(rollout) = &service.rollout {
            if let Ok(ports) = super::parser::get_rollout_ports(rollout) {
                for (port_name, port) in &ports {
                    trace!(service = %name, port_name = %port_name, port = %port, "Registered port");
                    port_usage
                        .entry(*port)
                        .or_default()
                        .push(format!("{}:{}", name, port_name));
                }
            }
        }
    }

    for (port, services) in port_usage {
        if services.len() > 1 {
            warn!(port = port, services = ?services, "Port conflict detected");
            result.add_error(
                "services",
                &format!(
                    "Port {} is used by multiple services: {}",
                    port,
                    services.join(", ")
                ),
            );
        }
    }
}

fn check_route_conflicts(config: &HiveConfig, result: &mut ValidationResult) {
    let mut routes: HashMap<(Option<String>, String), String> = HashMap::new();

    for (name, service) in &config.services {
        if let Some(proxy) = &service.proxy {
            for endpoint in proxy.endpoints() {
                let key = (endpoint.host.clone(), endpoint.path.clone());
                if let Some(existing) = routes.get(&key) {
                    result.add_error(
                        &format!("services.{}.proxy", name),
                        &format!(
                            "Route {:?}/{} conflicts with service {}",
                            endpoint.host, endpoint.path, existing
                        ),
                    );
                } else {
                    routes.insert(key, name.clone());
                }
            }
        }
    }
}

fn check_expose_conflicts(config: &HiveConfig, result: &mut ValidationResult) {
    let mut expose_names: HashMap<&str, &str> = HashMap::new();

    for (name, service) in &config.services {
        if let Some(expose) = &service.expose {
            if let Some(existing) = expose_names.get(expose.name.as_str()) {
                result.add_error(
                    &format!("services.{}.expose.name", name),
                    &format!(
                        "Expose name '{}' conflicts with service {}",
                        expose.name, existing
                    ),
                );
            } else {
                expose_names.insert(&expose.name, name);
            }
        }
    }
}

/// Groups services by dependency level for parallel startup/shutdown.
pub fn topological_sort_levels(config: &HiveConfig) -> Result<Vec<Vec<String>>> {
    debug!(
        services = config.services.len(),
        "Computing topological sort levels"
    );
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for name in config.services.keys() {
        in_degree.insert(name.as_str(), 0);
        graph.insert(name.as_str(), Vec::new());
    }

    for (name, service) in &config.services {
        for dep in &service.depends_on {
            graph.get_mut(dep.as_str()).map(|v| v.push(name.as_str()));
            *in_degree.get_mut(name.as_str()).unwrap() += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();
    queue.sort(); // deterministic order within levels

    let mut levels = Vec::new();
    let mut total = 0;

    while !queue.is_empty() {
        let level: Vec<String> = queue.iter().map(|s| s.to_string()).collect();
        total += level.len();

        let mut next_queue = Vec::new();
        for &node in &queue {
            if let Some(dependents) = graph.get(node) {
                for &dependent in dependents {
                    let deg = in_degree.get_mut(dependent).unwrap();
                    *deg -= 1;
                    if *deg == 0 {
                        next_queue.push(dependent);
                    }
                }
            }
        }
        next_queue.sort(); // deterministic order
        levels.push(level);
        queue = next_queue;
    }

    if total != config.services.len() {
        warn!("Circular dependency detected during topological sort");
        return Err(anyhow!("Circular dependency detected"));
    }

    debug!(levels = levels.len(), "Topological sort completed");
    for (i, level) in levels.iter().enumerate() {
        trace!(level = i, services = ?level, "Startup level");
    }

    Ok(levels)
}

pub fn topological_sort(config: &HiveConfig) -> Result<Vec<String>> {
    // Kahn's algorithm
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();

    for name in config.services.keys() {
        in_degree.insert(name.as_str(), 0);
        graph.insert(name.as_str(), Vec::new());
    }

    for (name, service) in &config.services {
        for dep in &service.depends_on {
            graph.get_mut(dep.as_str()).map(|v| v.push(name.as_str()));
            *in_degree.get_mut(name.as_str()).unwrap() += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    let mut result = Vec::new();

    while let Some(node) = queue.pop() {
        result.push(node.to_string());

        if let Some(dependents) = graph.get(node) {
            for &dependent in dependents {
                let deg = in_degree.get_mut(dependent).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push(dependent);
                }
            }
        }
    }

    if result.len() != config.services.len() {
        return Err(anyhow!("Circular dependency detected"));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_service_name() {
        let mut result = ValidationResult::new();

        validate_service_name("valid-name", &mut result);
        assert!(result.errors.is_empty());

        validate_service_name("also_valid", &mut result);
        assert!(result.errors.is_empty());

        validate_service_name("Invalid", &mut result);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_circular_dependency_detection() {
        let yaml = r#"
version: "1"

services:
  a:
    runner:
      type: script
      script:
        run: echo a
    depends_on:
      - b
  b:
    runner:
      type: script
      script:
        run: echo b
    depends_on:
      - c
  c:
    runner:
      type: script
      script:
        run: echo c
    depends_on:
      - a
"#;

        let config: HiveConfig = serde_yml::from_str(yaml).unwrap();
        let result = check_circular_dependencies(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_topological_sort() {
        let yaml = r#"
version: "1"

services:
  c:
    runner:
      type: script
      script:
        run: echo c
    depends_on:
      - a
      - b
  a:
    runner:
      type: script
      script:
        run: echo a
  b:
    runner:
      type: script
      script:
        run: echo b
    depends_on:
      - a
"#;

        let config: HiveConfig = serde_yml::from_str(yaml).unwrap();
        let order = topological_sort(&config).unwrap();

        // a must come before b and c
        let a_pos = order.iter().position(|s| s == "a").unwrap();
        let b_pos = order.iter().position(|s| s == "b").unwrap();
        let c_pos = order.iter().position(|s| s == "c").unwrap();

        assert!(a_pos < b_pos);
        assert!(a_pos < c_pos);
        assert!(b_pos < c_pos);
    }
}
