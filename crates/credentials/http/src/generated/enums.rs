//! Auto-generated enums from TypeSpec.
//! DO NOT EDIT.

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialType {
    #[serde(rename = "github_token")]
    GithubToken,
    #[serde(rename = "gitlab_token")]
    GitlabToken,
    #[serde(rename = "api_key")]
    ApiKey,
    #[serde(rename = "oauth2")]
    Oauth2,
    #[serde(rename = "ssh_key")]
    SshKey,
    #[serde(rename = "password")]
    Password,
    #[serde(rename = "certificate")]
    Certificate,
    #[serde(rename = "custom")]
    Custom,
}
