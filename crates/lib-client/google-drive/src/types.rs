use serde::{Deserialize, Serialize};

/// File in Google Drive.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct File {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(default)]
    pub parents: Vec<String>,
    pub created_time: Option<String>,
    pub modified_time: Option<String>,
    pub size: Option<String>,
    pub web_view_link: Option<String>,
    pub web_content_link: Option<String>,
    pub trashed: Option<bool>,
    pub shared: Option<bool>,
}

/// File list response.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    pub files: Vec<File>,
    pub next_page_token: Option<String>,
    pub incomplete_search: Option<bool>,
}

/// File metadata for create/update.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parents: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl FileMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parents = Some(vec![parent_id.into()]);
        self
    }

    pub fn folder(name: impl Into<String>) -> Self {
        Self::new(name).mime_type("application/vnd.google-apps.folder")
    }
}

/// Permission.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub permission_type: String,
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
}

impl Permission {
    pub fn user(email: impl Into<String>, role: PermissionRole) -> Self {
        Self {
            id: None,
            permission_type: "user".to_string(),
            role: role.as_str().to_string(),
            email_address: Some(email.into()),
            domain: None,
        }
    }

    pub fn anyone(role: PermissionRole) -> Self {
        Self {
            id: None,
            permission_type: "anyone".to_string(),
            role: role.as_str().to_string(),
            email_address: None,
            domain: None,
        }
    }

    pub fn domain(domain: impl Into<String>, role: PermissionRole) -> Self {
        Self {
            id: None,
            permission_type: "domain".to_string(),
            role: role.as_str().to_string(),
            email_address: None,
            domain: Some(domain.into()),
        }
    }
}

/// Permission role.
#[derive(Debug, Clone, Copy)]
pub enum PermissionRole {
    Owner,
    Organizer,
    FileOrganizer,
    Writer,
    Commenter,
    Reader,
}

impl PermissionRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Organizer => "organizer",
            Self::FileOrganizer => "fileOrganizer",
            Self::Writer => "writer",
            Self::Commenter => "commenter",
            Self::Reader => "reader",
        }
    }
}

/// Permission list.
#[derive(Debug, Clone, Deserialize)]
pub struct PermissionList {
    pub permissions: Vec<Permission>,
}

/// About info.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct About {
    pub user: User,
    pub storage_quota: StorageQuota,
}

/// User info.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub display_name: String,
    pub email_address: String,
    pub photo_link: Option<String>,
}

/// Storage quota.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageQuota {
    pub limit: Option<String>,
    pub usage: String,
    pub usage_in_drive: Option<String>,
    pub usage_in_drive_trash: Option<String>,
}
