use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A running cocoon tracked by the spawner.
#[derive(Debug, Clone)]
pub struct SpawnedCocoon {
    pub container_name: String,
    pub container_id: String,
    pub kind: String,
    pub setup_token: String,
    pub spawned_at: DateTime<Utc>,
    pub request_id: String,
}

/// Thread-safe spawner state tracking active cocoons and token pool.
#[derive(Debug, Clone)]
pub struct SpawnerState {
    inner: Arc<RwLock<Inner>>,
}

#[derive(Debug)]
struct Inner {
    /// Active cocoons keyed by container name.
    cocoons: HashMap<String, SpawnedCocoon>,
    max_concurrent: usize,
    /// Tokens available for assignment.
    available_tokens: Vec<String>,
    /// Token → container name mapping for assigned tokens.
    used_tokens: HashMap<String, String>,
}

impl SpawnerState {
    pub fn new(max_concurrent: usize, setup_tokens: Vec<String>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                cocoons: HashMap::new(),
                max_concurrent,
                available_tokens: setup_tokens,
                used_tokens: HashMap::new(),
            })),
        }
    }

    /// Check if we can spawn another cocoon.
    pub async fn can_spawn(&self) -> bool {
        let inner = self.inner.read().await;
        inner.cocoons.len() < inner.max_concurrent
    }

    /// Current number of active cocoons.
    pub async fn count(&self) -> usize {
        self.inner.read().await.cocoons.len()
    }

    /// Max concurrent limit.
    pub async fn max_concurrent(&self) -> usize {
        self.inner.read().await.max_concurrent
    }

    /// Claim a token from the pool for a container.
    pub async fn claim_token(&self, container_name: &str) -> Option<String> {
        let mut inner = self.inner.write().await;
        let token = inner.available_tokens.pop()?;
        inner
            .used_tokens
            .insert(token.clone(), container_name.to_string());
        Some(token)
    }

    /// Release a token back to the pool.
    pub async fn release_token(&self, token: &str) {
        let mut inner = self.inner.write().await;
        if inner.used_tokens.remove(token).is_some() {
            inner.available_tokens.push(token.to_string());
        }
    }

    /// Track a newly spawned cocoon.
    pub async fn add_cocoon(&self, cocoon: SpawnedCocoon) {
        let mut inner = self.inner.write().await;
        inner.cocoons.insert(cocoon.container_name.clone(), cocoon);
    }

    /// Remove a cocoon by container name. Returns the removed entry.
    pub async fn remove_cocoon(&self, container_name: &str) -> Option<SpawnedCocoon> {
        let mut inner = self.inner.write().await;
        inner.cocoons.remove(container_name)
    }

    /// Find a cocoon by container ID (from signaling TerminateCocoon).
    pub async fn find_by_container_id(&self, container_id: &str) -> Option<SpawnedCocoon> {
        let inner = self.inner.read().await;
        inner
            .cocoons
            .values()
            .find(|c| c.container_id == container_id)
            .cloned()
    }

    /// List all active cocoons.
    pub async fn list(&self) -> Vec<SpawnedCocoon> {
        let inner = self.inner.read().await;
        inner.cocoons.values().cloned().collect()
    }

    /// Get all tracked container names (for health checks).
    pub async fn container_names(&self) -> Vec<String> {
        let inner = self.inner.read().await;
        inner.cocoons.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn can_spawn_respects_limit() {
        let state = SpawnerState::new(2, vec![]);
        assert!(state.can_spawn().await);

        state
            .add_cocoon(SpawnedCocoon {
                container_name: "c1".into(),
                container_id: "id1".into(),
                kind: "ubuntu".into(),
                setup_token: "t1".into(),
                spawned_at: Utc::now(),
                request_id: "r1".into(),
            })
            .await;
        assert!(state.can_spawn().await);

        state
            .add_cocoon(SpawnedCocoon {
                container_name: "c2".into(),
                container_id: "id2".into(),
                kind: "ubuntu".into(),
                setup_token: "t2".into(),
                spawned_at: Utc::now(),
                request_id: "r2".into(),
            })
            .await;
        assert!(!state.can_spawn().await);
    }

    #[tokio::test]
    async fn token_claim_and_release() {
        let state = SpawnerState::new(10, vec!["tok1".into(), "tok2".into()]);

        let t1 = state.claim_token("c1").await;
        assert!(t1.is_some());

        let t2 = state.claim_token("c2").await;
        assert!(t2.is_some());

        let t3 = state.claim_token("c3").await;
        assert!(t3.is_none());

        state.release_token(t1.as_deref().unwrap()).await;
        let t4 = state.claim_token("c4").await;
        assert!(t4.is_some());
    }

    #[tokio::test]
    async fn find_by_container_id() {
        let state = SpawnerState::new(10, vec![]);
        state
            .add_cocoon(SpawnedCocoon {
                container_name: "cocoon-abc".into(),
                container_id: "sha256:abc123".into(),
                kind: "ubuntu".into(),
                setup_token: "t".into(),
                spawned_at: Utc::now(),
                request_id: "r".into(),
            })
            .await;

        assert!(state.find_by_container_id("sha256:abc123").await.is_some());
        assert!(state.find_by_container_id("unknown").await.is_none());
    }
}
