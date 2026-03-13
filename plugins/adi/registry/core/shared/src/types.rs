use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformBuild {
    pub platform: String,
    #[serde(alias = "download_url")]
    pub download_url: String,
    #[serde(default, alias = "size_bytes")]
    pub size_bytes: u64,
    pub checksum: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_signature: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher_certificate: Option<PublisherCertificate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublisherCertificate {
    pub publisher_id: String,
    pub publisher_public_key: String,
    pub registry_signature: String,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub status: String,
    pub id: String,
    pub version: String,
}
