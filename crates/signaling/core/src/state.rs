use dashmap::DashMap;
use serde_json::Value as JsonValue;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::mpsc;

/// Per-device metadata stored by the signaling server.
#[derive(Clone, Debug)]
pub struct DeviceMeta {
    pub tags: HashMap<String, String>,
    pub device_type: Option<String>,
    pub device_config: Option<JsonValue>,
}

#[derive(Clone)]
pub struct AppState {
    pub connections: Arc<DashMap<String, mpsc::UnboundedSender<String>>>,
    pub pairing_codes: Arc<DashMap<String, String>>,
    pub paired_devices: Arc<DashMap<String, String>>,
    pub device_meta: Arc<DashMap<String, DeviceMeta>>,
    /// device_id → owner user_id (from setup_token)
    pub device_owners: Arc<DashMap<String, String>>,
    /// user_id → (connection_id → sender) for authenticated app clients
    pub user_connections: Arc<DashMap<String, HashMap<u64, mpsc::UnboundedSender<String>>>>,
    connection_counter: Arc<AtomicU64>,
    pub hmac_salt: String,
    pub auth_domain: Option<String>,
    pub allow_manual_registration: bool,
    pub ice_servers: Vec<JsonValue>,
}

impl AppState {
    pub fn new(
        hmac_salt: String,
        auth_domain: Option<String>,
        allow_manual_registration: bool,
        ice_servers: Vec<JsonValue>,
    ) -> Self {
        Self {
            connections: Arc::new(DashMap::new()),
            pairing_codes: Arc::new(DashMap::new()),
            paired_devices: Arc::new(DashMap::new()),
            device_meta: Arc::new(DashMap::new()),
            device_owners: Arc::new(DashMap::new()),
            user_connections: Arc::new(DashMap::new()),
            connection_counter: Arc::new(AtomicU64::new(0)),
            hmac_salt,
            auth_domain,
            allow_manual_registration,
            ice_servers,
        }
    }

    pub fn next_connection_id(&self) -> u64 {
        self.connection_counter.fetch_add(1, Ordering::Relaxed)
    }

    /// Send a message to all app connections for a given user.
    pub fn notify_user(&self, user_id: &str, json: &str) {
        if let Some(conns) = self.user_connections.get(user_id) {
            for tx in conns.value().values() {
                let _ = tx.send(json.to_string());
            }
        }
    }

    /// Collect all devices owned by a given user.
    pub fn get_user_devices(&self, user_id: &str) -> Vec<UserDevice> {
        self.device_owners
            .iter()
            .filter(|entry| entry.value() == user_id)
            .map(|entry| {
                let device_id = entry.key().clone();
                let meta = self.device_meta.get(&device_id);
                let (tags, device_type, device_config) = match meta {
                    Some(m) => (m.tags.clone(), m.device_type.clone(), m.device_config.clone()),
                    None => (HashMap::new(), None, None),
                };
                let online = self.connections.contains_key(&device_id);
                UserDevice { device_id, tags, online, device_type, device_config }
            })
            .collect()
    }
}

pub struct UserDevice {
    pub device_id: String,
    pub tags: HashMap<String, String>,
    pub online: bool,
    pub device_type: Option<String>,
    pub device_config: Option<JsonValue>,
}
