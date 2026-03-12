//! SQLite Configuration Backend
//!
//! Provides a read-write alternative to YAML for Hive configuration.
//! Implements full schema as specified in Section 20 of the Hive YAML spec.

use crate::hive_config::{
    BuildConfig, BuildTrigger, EnvironmentConfig, ExposeConfig, HealthCheck, HealthCheckConfig,
    HiveConfig, ProxyBind, ProxyConfig, ProxyEndpoint, RestartPolicy, RolloutConfig, RunnerConfig,
    ServiceConfig, ServiceProxyConfig, UsesConfig,
};
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tracing::{debug, info, trace, warn};

pub struct SqliteBackend {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteBackend {
    pub fn open(path: &Path) -> Result<Self> {
        debug!(path = %path.display(), "Opening SQLite backend");
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SQLite database: {}", path.display()))?;

        let backend = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        backend.init_schema()?;
        Ok(backend)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute_batch(
            r#"
            -- Meta table
            CREATE TABLE IF NOT EXISTS hive_meta (
                key TEXT PRIMARY KEY,
                value TEXT
            );

            -- Global configuration
            CREATE TABLE IF NOT EXISTS global_defaults (
                plugin_id TEXT PRIMARY KEY,
                config JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS global_proxy (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                bind JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS global_environment (
                provider TEXT PRIMARY KEY,
                config JSON NOT NULL,
                priority INTEGER DEFAULT 0
            );

            -- Services
            CREATE TABLE IF NOT EXISTS services (
                name TEXT PRIMARY KEY,
                enabled BOOLEAN DEFAULT true,
                restart_policy TEXT DEFAULT 'never',
                working_dir TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS service_runners (
                service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
                runner_type TEXT NOT NULL,
                config JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS service_rollouts (
                service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
                rollout_type TEXT NOT NULL,
                config JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS service_proxies (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                host TEXT,
                path TEXT NOT NULL,
                port_ref TEXT DEFAULT '{{runtime.port.http}}',
                strip_prefix BOOLEAN DEFAULT false,
                timeout_ms INTEGER DEFAULT 60000,
                extra JSON
            );

            CREATE TABLE IF NOT EXISTS service_healthchecks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                check_type TEXT NOT NULL,
                config JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS service_environment (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                provider TEXT NOT NULL,
                config JSON NOT NULL,
                priority INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS service_dependencies (
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                depends_on TEXT NOT NULL,
                PRIMARY KEY (service_name, depends_on)
            );

            CREATE TABLE IF NOT EXISTS service_builds (
                service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
                command TEXT NOT NULL,
                working_dir TEXT,
                build_when TEXT DEFAULT 'missing'
            );

            CREATE TABLE IF NOT EXISTS service_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                log_type TEXT NOT NULL,
                config JSON NOT NULL
            );

            -- Service exposure
            CREATE TABLE IF NOT EXISTS service_expose (
                service_name TEXT PRIMARY KEY REFERENCES services(name) ON DELETE CASCADE,
                expose_name TEXT UNIQUE NOT NULL,
                secret_hash TEXT,
                vars JSON NOT NULL
            );

            CREATE TABLE IF NOT EXISTS service_uses (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                service_name TEXT NOT NULL REFERENCES services(name) ON DELETE CASCADE,
                exposed_name TEXT NOT NULL,
                secret_encrypted TEXT,
                local_alias TEXT,
                var_remaps JSON,
                UNIQUE(service_name, exposed_name)
            );

            -- Runtime state (not config)
            CREATE TABLE IF NOT EXISTS runtime_state (
                service_name TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                pid INTEGER,
                container_id TEXT,
                started_at TIMESTAMP,
                stopped_at TIMESTAMP,
                restart_count INTEGER DEFAULT 0,
                last_exit_code INTEGER,
                last_error TEXT
            );

            -- Set version
            INSERT OR REPLACE INTO hive_meta (key, value) VALUES ('schema_version', '1');
            INSERT OR REPLACE INTO hive_meta (key, value) VALUES ('config_version', '1');
            "#,
        )
        .context("Failed to initialize SQLite schema")?;

        debug!("SQLite schema initialized");
        Ok(())
    }

    pub fn schema_version(&self) -> Result<String> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM hive_meta WHERE key = 'schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        Ok(version.unwrap_or_else(|| "0".to_string()))
    }

    /// Load entire configuration as HiveConfig
    pub fn load_config(&self) -> Result<HiveConfig> {
        debug!("Loading full configuration from SQLite");
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Load version
        let version: Option<String> = conn
            .query_row(
                "SELECT value FROM hive_meta WHERE key = 'config_version'",
                [],
                |row| row.get(0),
            )
            .optional()?;

        // Load defaults
        let mut defaults: HashMap<String, JsonValue> = HashMap::new();
        {
            let mut stmt = conn.prepare("SELECT plugin_id, config FROM global_defaults")?;
            let rows = stmt.query_map([], |row| {
                let plugin_id: String = row.get(0)?;
                let config: String = row.get(1)?;
                Ok((plugin_id, config))
            })?;

            for row in rows {
                let (plugin_id, config) = row?;
                if let Ok(value) = serde_json::from_str(&config) {
                    defaults.insert(plugin_id, value);
                }
            }
        }

        // Load proxy bind
        let proxy: Option<ProxyConfig> = conn
            .query_row("SELECT bind FROM global_proxy WHERE id = 1", [], |row| {
                let bind: String = row.get(0)?;
                Ok(bind)
            })
            .optional()?
            .and_then(|bind_str| {
                serde_json::from_str::<Vec<String>>(&bind_str)
                    .ok()
                    .map(|addrs| ProxyConfig {
                        bind: ProxyBind::Multiple(addrs),
                        ssl: None,
                        dns: None,
                        plugins: vec![],
                        show_error_logs: true,
                        debug: false,
                    })
            });

        // Load services
        let services = self.load_services(&conn)?;

        Ok(HiveConfig {
            version: version.unwrap_or_else(|| "1".to_string()),
            registry_url: None,
            defaults,
            proxy,
            environment: None,
            observability: None,
            hooks: None,
            services,
        })
    }

    fn load_services(&self, conn: &Connection) -> Result<HashMap<String, ServiceConfig>> {
        let mut services = HashMap::new();
        trace!("Loading services from SQLite");

        let mut stmt = conn.prepare(
            "SELECT name, enabled, restart_policy, working_dir FROM services WHERE enabled = true",
        )?;

        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let _enabled: bool = row.get(1)?;
            let restart_policy: Option<String> = row.get(2)?;
            let _working_dir: Option<String> = row.get(3)?;
            Ok((name, restart_policy))
        })?;

        for row in rows {
            let (name, restart_policy_str) = row?;

            let restart = restart_policy_str
                .map(|s| match s.as_str() {
                    "always" => RestartPolicy::Always,
                    "on-failure" => RestartPolicy::OnFailure,
                    "unless-stopped" => RestartPolicy::UnlessStopped,
                    _ => RestartPolicy::Never,
                })
                .unwrap_or(RestartPolicy::Never);

            // Load runner (required)
            let runner = self
                .load_runner(conn, &name)?
                .ok_or_else(|| anyhow!("Service '{}' has no runner configuration", name))?;

            // Load rollout
            let rollout = self.load_rollout(conn, &name)?;

            // Load proxy
            let proxy = self.load_proxy(conn, &name)?;

            // Load healthcheck
            let healthcheck = self.load_healthcheck(conn, &name)?;

            // Load dependencies
            let depends_on = self.load_dependencies(conn, &name)?;

            // Load environment
            let environment = self.load_environment(conn, &name)?;

            // Load build
            let build = self.load_build(conn, &name)?;

            // Load expose
            let expose = self.load_expose(conn, &name)?;

            // Load uses
            let uses = self.load_uses(conn, &name)?;

            let service = ServiceConfig {
                runner,
                rollout,
                proxy,
                healthcheck,
                depends_on,
                environment,
                build,
                restart,
                expose,
                uses,
                hooks: None,
            };

            trace!(service = %name, "Loaded service config");
            services.insert(name, service);
        }

        debug!(count = services.len(), "Loaded services from SQLite");
        Ok(services)
    }

    fn load_runner(&self, conn: &Connection, service_name: &str) -> Result<Option<RunnerConfig>> {
        let result: Option<(String, String)> = conn
            .query_row(
                "SELECT runner_type, config FROM service_runners WHERE service_name = ?1",
                params![service_name],
                |row| {
                    let runner_type: String = row.get(0)?;
                    let config: String = row.get(1)?;
                    Ok((runner_type, config))
                },
            )
            .optional()?;

        match result {
            Some((runner_type, config_str)) => {
                let config: HashMap<String, JsonValue> = serde_json::from_str(&config_str)?;
                Ok(Some(RunnerConfig {
                    runner_type,
                    config,
                }))
            }
            None => Ok(None),
        }
    }

    fn load_rollout(&self, conn: &Connection, service_name: &str) -> Result<Option<RolloutConfig>> {
        let result: Option<(String, String)> = conn
            .query_row(
                "SELECT rollout_type, config FROM service_rollouts WHERE service_name = ?1",
                params![service_name],
                |row| {
                    let rollout_type: String = row.get(0)?;
                    let config: String = row.get(1)?;
                    Ok((rollout_type, config))
                },
            )
            .optional()?;

        match result {
            Some((rollout_type, config_str)) => {
                let config: HashMap<String, JsonValue> = serde_json::from_str(&config_str)?;
                Ok(Some(RolloutConfig {
                    rollout_type,
                    config,
                }))
            }
            None => Ok(None),
        }
    }

    fn load_proxy(
        &self,
        conn: &Connection,
        service_name: &str,
    ) -> Result<Option<ServiceProxyConfig>> {
        let mut stmt = conn.prepare(
            "SELECT host, path, port_ref, strip_prefix, timeout_ms, extra 
             FROM service_proxies WHERE service_name = ?1",
        )?;

        let rows = stmt.query_map(params![service_name], |row| {
            let host: Option<String> = row.get(0)?;
            let path: String = row.get(1)?;
            let port_ref: Option<String> = row.get(2)?;
            let strip_prefix: bool = row.get(3)?;
            let timeout_ms: i64 = row.get(4)?;
            let _extra: Option<String> = row.get(5)?;
            Ok((host, path, port_ref, strip_prefix, timeout_ms))
        })?;

        let mut endpoints = Vec::new();
        for row in rows {
            let (host, path, port, strip_prefix, timeout_ms) = row?;

            let endpoint = ProxyEndpoint {
                host,
                path,
                port,
                strip_prefix,
                timeout: if timeout_ms != 60000 {
                    Some(format!("{}ms", timeout_ms))
                } else {
                    None
                },
                buffer_size: None,
                headers: None,
                plugins: vec![],
            };

            endpoints.push(endpoint);
        }

        if endpoints.is_empty() {
            Ok(None)
        } else if endpoints.len() == 1 {
            Ok(Some(ServiceProxyConfig::Single(endpoints.remove(0))))
        } else {
            Ok(Some(ServiceProxyConfig::Multiple(endpoints)))
        }
    }

    fn load_healthcheck(
        &self,
        conn: &Connection,
        service_name: &str,
    ) -> Result<Option<HealthCheckConfig>> {
        let mut stmt = conn.prepare(
            "SELECT check_type, config FROM service_healthchecks WHERE service_name = ?1",
        )?;

        let rows = stmt.query_map(params![service_name], |row| {
            let check_type: String = row.get(0)?;
            let config: String = row.get(1)?;
            Ok((check_type, config))
        })?;

        let mut checks = Vec::new();
        for row in rows {
            let (check_type, config_str) = row?;
            let mut config: HashMap<String, JsonValue> = serde_json::from_str(&config_str)?;
            // Add the check_type config under its own key
            let check_config = config.clone();
            config.clear();
            config.insert(check_type.clone(), serde_json::to_value(check_config)?);

            checks.push(HealthCheck { check_type, config });
        }

        if checks.is_empty() {
            Ok(None)
        } else if checks.len() == 1 {
            Ok(Some(HealthCheckConfig::Single(checks.remove(0))))
        } else {
            Ok(Some(HealthCheckConfig::Multiple(checks)))
        }
    }

    fn load_dependencies(&self, conn: &Connection, service_name: &str) -> Result<Vec<String>> {
        let mut stmt =
            conn.prepare("SELECT depends_on FROM service_dependencies WHERE service_name = ?1")?;

        let rows = stmt.query_map(params![service_name], |row| {
            let dep: String = row.get(0)?;
            Ok(dep)
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    fn load_environment(
        &self,
        conn: &Connection,
        service_name: &str,
    ) -> Result<Option<EnvironmentConfig>> {
        let mut stmt = conn.prepare(
            "SELECT provider, config FROM service_environment WHERE service_name = ?1 ORDER BY priority",
        )?;

        let rows = stmt.query_map(params![service_name], |row| {
            let provider: String = row.get(0)?;
            let config: String = row.get(1)?;
            Ok((provider, config))
        })?;

        let mut static_env: Option<HashMap<String, String>> = None;
        let mut providers: HashMap<String, JsonValue> = HashMap::new();

        for row in rows {
            let (provider, config_str) = row?;
            if provider == "static" {
                if let Ok(env) = serde_json::from_str(&config_str) {
                    static_env = Some(env);
                }
            } else if let Ok(config) = serde_json::from_str(&config_str) {
                providers.insert(provider, config);
            }
        }

        if static_env.is_none() && providers.is_empty() {
            Ok(None)
        } else {
            Ok(Some(EnvironmentConfig {
                static_env,
                providers,
            }))
        }
    }

    fn load_build(&self, conn: &Connection, service_name: &str) -> Result<Option<BuildConfig>> {
        let result: Option<(String, Option<String>, String)> = conn
            .query_row(
                "SELECT command, working_dir, build_when FROM service_builds WHERE service_name = ?1",
                params![service_name],
                |row| {
                    let command: String = row.get(0)?;
                    let working_dir: Option<String> = row.get(1)?;
                    let build_when: String = row.get(2)?;
                    Ok((command, working_dir, build_when))
                },
            )
            .optional()?;

        match result {
            Some((command, working_dir, build_when_str)) => {
                let build_when = match build_when_str.as_str() {
                    "always" => BuildTrigger::Always,
                    "never" => BuildTrigger::Never,
                    _ => BuildTrigger::Missing,
                };
                Ok(Some(BuildConfig {
                    command,
                    working_dir,
                    build_when,
                    output: None, // SQLite backend doesn't support output field yet
                }))
            }
            None => Ok(None),
        }
    }

    fn load_expose(&self, conn: &Connection, service_name: &str) -> Result<Option<ExposeConfig>> {
        let result: Option<(String, Option<String>, String)> = conn
            .query_row(
                "SELECT expose_name, secret_hash, vars FROM service_expose WHERE service_name = ?1",
                params![service_name],
                |row| {
                    let expose_name: String = row.get(0)?;
                    let secret_hash: Option<String> = row.get(1)?;
                    let vars: String = row.get(2)?;
                    Ok((expose_name, secret_hash, vars))
                },
            )
            .optional()?;

        match result {
            Some((name, secret, vars_str)) => {
                let vars: HashMap<String, String> = serde_json::from_str(&vars_str)?;
                Ok(Some(ExposeConfig { name, secret, vars }))
            }
            None => Ok(None),
        }
    }

    fn load_uses(&self, conn: &Connection, service_name: &str) -> Result<Vec<UsesConfig>> {
        let mut stmt = conn.prepare(
            "SELECT exposed_name, secret_encrypted, local_alias, var_remaps 
             FROM service_uses WHERE service_name = ?1",
        )?;

        let rows = stmt.query_map(params![service_name], |row| {
            let exposed_name: String = row.get(0)?;
            let secret: Option<String> = row.get(1)?;
            let local_alias: Option<String> = row.get(2)?;
            let var_remaps: Option<String> = row.get(3)?;
            Ok((exposed_name, secret, local_alias, var_remaps))
        })?;

        let mut uses = Vec::new();
        for row in rows {
            let (name, secret, alias, vars_str) = row?;
            let vars: HashMap<String, String> = vars_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();

            uses.push(UsesConfig {
                name,
                secret,
                alias,
                vars,
            });
        }

        Ok(uses)
    }

    // ==================== Write Operations ====================

    pub fn create_service(&self, name: &str, config: &ServiceConfig) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])?;

        let result = (|| -> Result<()> {
            // Insert service
            let restart_policy = match config.restart {
                RestartPolicy::Never => "never",
                RestartPolicy::Always => "always",
                RestartPolicy::OnFailure => "on-failure",
                RestartPolicy::UnlessStopped => "unless-stopped",
            };

            conn.execute(
                "INSERT INTO services (name, restart_policy) VALUES (?1, ?2)",
                params![name, restart_policy],
            )?;

            // Insert runner
            conn.execute(
                "INSERT INTO service_runners (service_name, runner_type, config) VALUES (?1, ?2, ?3)",
                params![
                    name,
                    config.runner.runner_type,
                    serde_json::to_string(&config.runner.config)?
                ],
            )?;

            // Insert rollout
            if let Some(rollout) = &config.rollout {
                conn.execute(
                    "INSERT INTO service_rollouts (service_name, rollout_type, config) VALUES (?1, ?2, ?3)",
                    params![
                        name,
                        rollout.rollout_type,
                        serde_json::to_string(&rollout.config)?
                    ],
                )?;
            }

            // Insert proxy endpoints
            if let Some(proxy) = &config.proxy {
                for endpoint in proxy.endpoints() {
                    let timeout_ms = endpoint
                        .timeout
                        .as_ref()
                        .and_then(|t| parse_timeout_ms(t))
                        .unwrap_or(60000);

                    conn.execute(
                        "INSERT INTO service_proxies (service_name, host, path, port_ref, strip_prefix, timeout_ms) 
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![
                            name,
                            endpoint.host,
                            endpoint.path,
                            endpoint.port,
                            endpoint.strip_prefix,
                            timeout_ms
                        ],
                    )?;
                }
            }

            // Insert healthchecks
            if let Some(healthcheck) = &config.healthcheck {
                for check in healthcheck.checks() {
                    let check_config = check
                        .config
                        .get(&check.check_type)
                        .unwrap_or(&JsonValue::Null);
                    conn.execute(
                        "INSERT INTO service_healthchecks (service_name, check_type, config) VALUES (?1, ?2, ?3)",
                        params![name, check.check_type, serde_json::to_string(check_config)?],
                    )?;
                }
            }

            // Insert dependencies
            for dep in &config.depends_on {
                conn.execute(
                    "INSERT INTO service_dependencies (service_name, depends_on) VALUES (?1, ?2)",
                    params![name, dep],
                )?;
            }

            // Insert build
            if let Some(build) = &config.build {
                let build_when = match build.build_when {
                    BuildTrigger::Missing => "missing",
                    BuildTrigger::Always => "always",
                    BuildTrigger::Never => "never",
                };
                conn.execute(
                    "INSERT INTO service_builds (service_name, command, working_dir, build_when) VALUES (?1, ?2, ?3, ?4)",
                    params![name, build.command, build.working_dir, build_when],
                )?;
            }

            // Insert expose
            if let Some(expose) = &config.expose {
                conn.execute(
                    "INSERT INTO service_expose (service_name, expose_name, secret_hash, vars) VALUES (?1, ?2, ?3, ?4)",
                    params![
                        name,
                        expose.name,
                        expose.secret,
                        serde_json::to_string(&expose.vars)?
                    ],
                )?;
            }

            // Insert uses
            for uses in &config.uses {
                let vars_json = if uses.vars.is_empty() {
                    None
                } else {
                    Some(serde_json::to_string(&uses.vars)?)
                };
                conn.execute(
                    "INSERT INTO service_uses (service_name, exposed_name, secret_encrypted, local_alias, var_remaps) 
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![name, uses.name, uses.secret, uses.alias, vars_json],
                )?;
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                info!("Created service '{}' in SQLite backend", name);
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    pub fn delete_service(&self, name: &str) -> Result<()> {
        debug!(service = %name, "Deleting service from SQLite");
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // CASCADE will handle related tables
        let rows = conn.execute("DELETE FROM services WHERE name = ?1", params![name])?;

        if rows == 0 {
            warn!(service = %name, "Service not found for deletion");
            return Err(anyhow!("Service '{}' not found", name));
        }

        info!("Deleted service '{}' from SQLite backend", name);
        Ok(())
    }

    /// Update a service (partial update via JSON patch)
    pub fn update_service(&self, name: &str, patch: &ServicePatch) -> Result<()> {
        debug!(service = %name, "Updating service in SQLite");
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        // Check service exists
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM services WHERE name = ?1",
                params![name],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !exists {
            return Err(anyhow!("Service '{}' not found", name));
        }

        // Start transaction
        conn.execute("BEGIN TRANSACTION", [])?;

        let result = (|| -> Result<()> {
            // Update restart policy
            if let Some(restart) = &patch.restart {
                let restart_str = match restart {
                    RestartPolicy::Never => "never",
                    RestartPolicy::Always => "always",
                    RestartPolicy::OnFailure => "on-failure",
                    RestartPolicy::UnlessStopped => "unless-stopped",
                };
                conn.execute(
                    "UPDATE services SET restart_policy = ?1, updated_at = CURRENT_TIMESTAMP WHERE name = ?2",
                    params![restart_str, name],
                )?;
            }

            // Update runner
            if let Some(runner) = &patch.runner {
                conn.execute(
                    "INSERT OR REPLACE INTO service_runners (service_name, runner_type, config) VALUES (?1, ?2, ?3)",
                    params![
                        name,
                        runner.runner_type,
                        serde_json::to_string(&runner.config)?
                    ],
                )?;
            }

            // Update rollout
            if let Some(rollout) = &patch.rollout {
                conn.execute(
                    "INSERT OR REPLACE INTO service_rollouts (service_name, rollout_type, config) VALUES (?1, ?2, ?3)",
                    params![
                        name,
                        rollout.rollout_type,
                        serde_json::to_string(&rollout.config)?
                    ],
                )?;
            }

            // Update enabled status
            if let Some(enabled) = patch.enabled {
                conn.execute(
                    "UPDATE services SET enabled = ?1, updated_at = CURRENT_TIMESTAMP WHERE name = ?2",
                    params![enabled, name],
                )?;
            }

            Ok(())
        })();

        match result {
            Ok(()) => {
                conn.execute("COMMIT", [])?;
                info!("Updated service '{}' in SQLite backend", name);
                Ok(())
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK", []);
                Err(e)
            }
        }
    }

    pub fn list_services(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let mut stmt = conn.prepare("SELECT name FROM services ORDER BY name")?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            Ok(name)
        })?;

        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get_service(&self, name: &str) -> Result<Option<ServiceConfig>> {
        let config = self.load_config()?;
        Ok(config.services.get(name).cloned())
    }

    pub fn set_default(&self, plugin_id: &str, config: &JsonValue) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO global_defaults (plugin_id, config) VALUES (?1, ?2)",
            params![plugin_id, serde_json::to_string(config)?],
        )?;

        info!("Set default for plugin '{}' in SQLite backend", plugin_id);
        Ok(())
    }

    pub fn get_default(&self, plugin_id: &str) -> Result<Option<JsonValue>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let result: Option<String> = conn
            .query_row(
                "SELECT config FROM global_defaults WHERE plugin_id = ?1",
                params![plugin_id],
                |row| row.get(0),
            )
            .optional()?;

        match result {
            Some(config_str) => Ok(Some(serde_json::from_str(&config_str)?)),
            None => Ok(None),
        }
    }

    pub fn set_proxy_bind(&self, bind: &[String]) -> Result<()> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute(
            "INSERT OR REPLACE INTO global_proxy (id, bind) VALUES (1, ?1)",
            params![serde_json::to_string(bind)?],
        )?;

        Ok(())
    }

    // ==================== Runtime State ====================

    pub fn update_runtime_state(&self, name: &str, state: &RuntimeState) -> Result<()> {
        trace!(service = %name, state = %state.state, "Updating runtime state");
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        conn.execute(
            r#"INSERT OR REPLACE INTO runtime_state 
               (service_name, state, pid, container_id, started_at, stopped_at, restart_count, last_exit_code, last_error)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
            params![
                name,
                state.state,
                state.pid,
                state.container_id,
                state.started_at,
                state.stopped_at,
                state.restart_count,
                state.last_exit_code,
                state.last_error
            ],
        )?;

        Ok(())
    }

    pub fn get_runtime_state(&self, name: &str) -> Result<Option<RuntimeState>> {
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

        let result = conn
            .query_row(
                r#"SELECT state, pid, container_id, started_at, stopped_at, restart_count, last_exit_code, last_error
               FROM runtime_state WHERE service_name = ?1"#,
                params![name],
                |row| {
                    Ok(RuntimeState {
                        state: row.get(0)?,
                        pid: row.get(1)?,
                        container_id: row.get(2)?,
                        started_at: row.get(3)?,
                        stopped_at: row.get(4)?,
                        restart_count: row.get(5)?,
                        last_exit_code: row.get(6)?,
                        last_error: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    pub fn clear_runtime_state(&self) -> Result<()> {
        debug!("Clearing all runtime state");
        let conn = self.conn.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        conn.execute("DELETE FROM runtime_state", [])?;
        Ok(())
    }

    pub fn export_yaml(&self) -> Result<String> {
        debug!("Exporting configuration to YAML");
        let config = self.load_config()?;
        serde_yml::to_string(&config).context("Failed to serialize config to YAML")
    }
}

fn parse_timeout_ms(s: &str) -> Option<i64> {
    let s = s.trim();
    if let Some(ms) = s.strip_suffix("ms") {
        ms.parse().ok()
    } else if let Some(sec) = s.strip_suffix('s') {
        sec.parse::<i64>().ok().map(|s| s * 1000)
    } else if let Some(min) = s.strip_suffix('m') {
        min.parse::<i64>().ok().map(|m| m * 60 * 1000)
    } else {
        s.parse().ok()
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ServicePatch {
    pub runner: Option<RunnerConfig>,
    pub rollout: Option<RolloutConfig>,
    pub restart: Option<RestartPolicy>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeState {
    pub state: String,
    pub pid: Option<i64>,
    pub container_id: Option<String>,
    pub started_at: Option<String>,
    pub stopped_at: Option<String>,
    pub restart_count: i64,
    pub last_exit_code: Option<i32>,
    pub last_error: Option<String>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            state: "stopped".to_string(),
            pid: None,
            container_id: None,
            started_at: None,
            stopped_at: None,
            restart_count: 0,
            last_exit_code: None,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sqlite_backend_init() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let backend = SqliteBackend::open(&db_path).unwrap();
        assert_eq!(backend.schema_version().unwrap(), "1");
    }

    #[test]
    fn test_sqlite_create_and_load_service() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let backend = SqliteBackend::open(&db_path).unwrap();

        // Create a simple service
        let mut config_map = HashMap::new();
        config_map.insert(
            "script".to_string(),
            serde_json::json!({ "run": "echo hello" }),
        );

        let service = ServiceConfig {
            runner: RunnerConfig {
                runner_type: "script".to_string(),
                config: config_map,
            },
            rollout: None,
            proxy: None,
            healthcheck: None,
            depends_on: vec!["postgres".to_string()],
            environment: None,
            build: None,
            restart: RestartPolicy::OnFailure,
            expose: None,
            uses: vec![],
            hooks: None,
        };

        backend.create_service("test-service", &service).unwrap();

        // Load and verify
        let loaded = backend.get_service("test-service").unwrap().unwrap();
        assert_eq!(loaded.runner.runner_type, "script");
        assert_eq!(loaded.depends_on, vec!["postgres".to_string()]);
    }

    #[test]
    fn test_parse_timeout_ms() {
        assert_eq!(parse_timeout_ms("100ms"), Some(100));
        assert_eq!(parse_timeout_ms("5s"), Some(5000));
        assert_eq!(parse_timeout_ms("2m"), Some(120000));
        assert_eq!(parse_timeout_ms("1000"), Some(1000));
    }
}
