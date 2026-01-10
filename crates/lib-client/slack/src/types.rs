use serde::{Deserialize, Serialize};

use crate::blocks::Block;

/// Slack API response wrapper.
#[derive(Debug, Clone, Deserialize)]
pub struct SlackResponse<T> {
    pub ok: bool,
    #[serde(flatten)]
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Message.
#[derive(Debug, Clone, Deserialize)]
pub struct Message {
    pub ts: String,
    pub text: Option<String>,
    pub user: Option<String>,
    #[serde(rename = "type")]
    pub message_type: String,
    pub thread_ts: Option<String>,
}

/// Channel.
#[derive(Debug, Clone, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub is_channel: Option<bool>,
    pub is_private: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_member: Option<bool>,
    pub topic: Option<Topic>,
    pub purpose: Option<Purpose>,
    pub num_members: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Topic {
    pub value: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Purpose {
    pub value: String,
}

/// User.
#[derive(Debug, Clone, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub real_name: Option<String>,
    pub profile: Option<UserProfile>,
    pub is_bot: Option<bool>,
    pub is_admin: Option<bool>,
    pub deleted: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UserProfile {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub image_72: Option<String>,
    pub image_192: Option<String>,
}

/// File.
#[derive(Debug, Clone, Deserialize)]
pub struct File {
    pub id: String,
    pub name: String,
    pub title: String,
    pub mimetype: String,
    pub filetype: String,
    pub size: u64,
    pub url_private: Option<String>,
    pub url_private_download: Option<String>,
    pub permalink: Option<String>,
}

/// Post message request.
#[derive(Debug, Clone, Serialize)]
pub struct PostMessageRequest {
    pub channel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<Block>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_broadcast: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unfurl_links: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unfurl_media: Option<bool>,
}

impl PostMessageRequest {
    pub fn new(channel: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            text: None,
            blocks: None,
            thread_ts: None,
            reply_broadcast: None,
            unfurl_links: None,
            unfurl_media: None,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn blocks(mut self, blocks: Vec<Block>) -> Self {
        self.blocks = Some(blocks);
        self
    }

    pub fn thread_ts(mut self, ts: impl Into<String>) -> Self {
        self.thread_ts = Some(ts.into());
        self
    }
}

/// Update message request.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateMessageRequest {
    pub channel: String,
    pub ts: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<Block>>,
}

impl UpdateMessageRequest {
    pub fn new(channel: impl Into<String>, ts: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            ts: ts.into(),
            text: None,
            blocks: None,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn blocks(mut self, blocks: Vec<Block>) -> Self {
        self.blocks = Some(blocks);
        self
    }
}

/// Message response.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub channel: String,
    pub ts: String,
    pub message: Option<Message>,
}

/// Channels list response.
#[derive(Debug, Clone, Deserialize)]
pub struct ChannelsListResponse {
    pub channels: Vec<Channel>,
    pub response_metadata: Option<ResponseMetadata>,
}

/// Users list response.
#[derive(Debug, Clone, Deserialize)]
pub struct UsersListResponse {
    pub members: Vec<User>,
    pub response_metadata: Option<ResponseMetadata>,
}

/// Response metadata for pagination.
#[derive(Debug, Clone, Deserialize)]
pub struct ResponseMetadata {
    pub next_cursor: Option<String>,
}

/// File upload response.
#[derive(Debug, Clone, Deserialize)]
pub struct FileUploadResponse {
    pub file: File,
}

/// Reaction request.
#[derive(Debug, Clone, Serialize)]
pub struct ReactionRequest {
    pub channel: String,
    pub timestamp: String,
    pub name: String,
}

impl ReactionRequest {
    pub fn new(
        channel: impl Into<String>,
        timestamp: impl Into<String>,
        emoji: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            timestamp: timestamp.into(),
            name: emoji.into(),
        }
    }
}
