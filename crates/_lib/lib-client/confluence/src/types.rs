use serde::{Deserialize, Serialize};

/// Page in Confluence.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub status: String,
    pub space: Option<SpaceRef>,
    pub body: Option<Body>,
    pub version: Option<Version>,
    #[serde(rename = "_links")]
    pub links: Option<Links>,
}

/// Space reference.
#[derive(Debug, Clone, Deserialize)]
pub struct SpaceRef {
    pub id: Option<String>,
    pub key: String,
    pub name: Option<String>,
}

/// Body content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Body {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<Storage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<Storage>,
}

/// Storage format content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    pub value: String,
    pub representation: String,
}

impl Storage {
    pub fn storage(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            representation: "storage".to_string(),
        }
    }
}

/// Version info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub number: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Links.
#[derive(Debug, Clone, Deserialize)]
pub struct Links {
    #[serde(rename = "webui")]
    pub web_ui: Option<String>,
    #[serde(rename = "self")]
    pub self_url: Option<String>,
}

/// Space in Confluence.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Space {
    pub id: String,
    pub key: String,
    pub name: String,
    #[serde(rename = "type")]
    pub space_type: String,
    pub status: String,
    #[serde(rename = "_links")]
    pub links: Option<Links>,
}

/// Content search result.
#[derive(Debug, Clone, Deserialize)]
pub struct SearchResult {
    pub results: Vec<Page>,
    pub start: u32,
    pub limit: u32,
    pub size: u32,
    #[serde(rename = "_links")]
    pub links: Option<SearchLinks>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchLinks {
    pub next: Option<String>,
}

/// Spaces list result.
#[derive(Debug, Clone, Deserialize)]
pub struct SpacesResult {
    pub results: Vec<Space>,
    pub start: u32,
    pub limit: u32,
    pub size: u32,
}

/// Children result.
#[derive(Debug, Clone, Deserialize)]
pub struct ChildrenResult {
    pub page: Option<PageChildren>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PageChildren {
    pub results: Vec<Page>,
    pub start: u32,
    pub limit: u32,
    pub size: u32,
}

/// Create page input.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePageInput {
    #[serde(rename = "type")]
    pub content_type: String,
    pub title: String,
    pub space: SpaceKey,
    pub body: Body,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ancestors: Option<Vec<Ancestor>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpaceKey {
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Ancestor {
    pub id: String,
}

impl CreatePageInput {
    pub fn new(
        space_key: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            content_type: "page".to_string(),
            title: title.into(),
            space: SpaceKey {
                key: space_key.into(),
            },
            body: Body {
                storage: Some(Storage::storage(body)),
                view: None,
            },
            ancestors: None,
        }
    }

    pub fn parent(mut self, parent_id: impl Into<String>) -> Self {
        self.ancestors = Some(vec![Ancestor {
            id: parent_id.into(),
        }]);
        self
    }
}

/// Update page input.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePageInput {
    pub version: Version,
    pub title: String,
    #[serde(rename = "type")]
    pub content_type: String,
    pub body: Body,
}

impl UpdatePageInput {
    pub fn new(version: u32, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            version: Version {
                number: version,
                message: None,
            },
            title: title.into(),
            content_type: "page".to_string(),
            body: Body {
                storage: Some(Storage::storage(body)),
                view: None,
            },
        }
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.version.message = Some(message.into());
        self
    }
}

/// Attachment.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub id: String,
    pub title: String,
    pub media_type: String,
    pub file_size: u64,
    #[serde(rename = "_links")]
    pub links: Option<AttachmentLinks>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttachmentLinks {
    pub download: Option<String>,
    #[serde(rename = "webui")]
    pub web_ui: Option<String>,
}

/// Attachments result.
#[derive(Debug, Clone, Deserialize)]
pub struct AttachmentsResult {
    pub results: Vec<Attachment>,
    pub start: u32,
    pub limit: u32,
    pub size: u32,
}
