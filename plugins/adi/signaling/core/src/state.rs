use dashmap::DashMap;
use serde_json::Value as JsonValue;
use std::{
    collections::{HashMap, HashSet},
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

/// A multi-party room where actors (devices) communicate and users collaborate.
#[derive(Clone, Debug)]
pub struct Room {
    pub room_id: String,
    pub owner_user_id: String,
    pub granted_users: HashSet<String>,
    pub actors: HashSet<String>,
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
    /// room_id → Room
    pub rooms: Arc<DashMap<String, Room>>,
    /// device_id → set of room_ids (reverse index for disconnect cleanup)
    pub device_rooms: Arc<DashMap<String, HashSet<String>>>,
    /// hive_id → registered hive info (for cocoon spawning)
    pub hives: Arc<DashMap<String, RegisteredHive>>,
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
            rooms: Arc::new(DashMap::new()),
            device_rooms: Arc::new(DashMap::new()),
            hives: Arc::new(DashMap::new()),
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

    /// Check if a user has access to a room (owner or granted).
    pub fn user_has_room_access(&self, room_id: &str, user_id: &str) -> bool {
        self.rooms
            .get(room_id)
            .map(|room| room.owner_user_id == user_id || room.granted_users.contains(user_id))
            .unwrap_or(false)
    }

    /// Get all rooms a user can access (owned or granted).
    pub fn get_user_rooms(&self, user_id: &str) -> Vec<Room> {
        self.rooms
            .iter()
            .filter(|entry| {
                let room = entry.value();
                room.owner_user_id == user_id || room.granted_users.contains(user_id)
            })
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Send a message to all room participants (actors + user app connections), excluding one device.
    pub fn notify_room(&self, room_id: &str, json: &str, exclude_device: Option<&str>) {
        let Some(room) = self.rooms.get(room_id) else { return };

        for actor_id in &room.actors {
            if exclude_device == Some(actor_id.as_str()) {
                continue;
            }
            if let Some(tx) = self.connections.get(actor_id) {
                let _ = tx.send(json.to_string());
            }
        }

        self.notify_user(&room.owner_user_id, json);
        for uid in &room.granted_users {
            self.notify_user(uid, json);
        }
    }

    /// Remove a device from all rooms on disconnect. Returns affected (room_id, device_id) pairs.
    pub fn cleanup_device_rooms(&self, device_id: &str) -> Vec<String> {
        let Some((_, room_ids)) = self.device_rooms.remove(device_id) else {
            return Vec::new();
        };
        room_ids.into_iter().collect()
    }

    /// Add a device to a room's actor set and update the reverse index.
    pub fn add_actor_to_room(&self, room_id: &str, device_id: &str) {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            room.actors.insert(device_id.to_string());
        }
        self.device_rooms
            .entry(device_id.to_string())
            .or_default()
            .insert(room_id.to_string());
    }

    /// Remove a device from a room's actor set and update the reverse index.
    pub fn remove_actor_from_room(&self, room_id: &str, device_id: &str) {
        if let Some(mut room) = self.rooms.get_mut(room_id) {
            room.actors.remove(device_id);
        }
        if let Some(mut rooms) = self.device_rooms.get_mut(device_id) {
            rooms.remove(room_id);
            if rooms.is_empty() {
                drop(rooms);
                self.device_rooms.remove(device_id);
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

/// A registered hive that can spawn cocoons.
#[derive(Clone, Debug)]
pub struct RegisteredHive {
    pub hive_id: String,
    pub connection_id: String,
    pub cocoon_kinds: Vec<String>,
}
