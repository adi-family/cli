use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "credential_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    GithubToken,
    GitlabToken,
    ApiKey,
    Oauth2,
    SshKey,
    Password,
    Certificate,
    Custom,
}

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct CredentialRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub credential_type: CredentialType,
    pub encrypted_data: String,
    pub metadata: serde_json::Value,
    pub provider: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Credential {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub credential_type: CredentialType,
    pub metadata: serde_json::Value,
    pub provider: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

impl From<CredentialRow> for Credential {
    fn from(row: CredentialRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            description: row.description,
            credential_type: row.credential_type,
            metadata: row.metadata,
            provider: row.provider,
            expires_at: row.expires_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_used_at: row.last_used_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CredentialWithData {
    #[serde(flatten)]
    pub credential: Credential,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCredential {
    pub name: String,
    pub description: Option<String>,
    pub credential_type: CredentialType,
    pub data: serde_json::Value,
    pub metadata: Option<serde_json::Value>,
    pub provider: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCredential {
    pub name: Option<String>,
    pub description: Option<String>,
    pub data: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
    pub provider: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListCredentialsQuery {
    pub credential_type: Option<CredentialType>,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, FromRow, Serialize)]
pub struct CredentialAccessLog {
    pub id: Uuid,
    pub credential_id: Uuid,
    pub user_id: Uuid,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub details: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
