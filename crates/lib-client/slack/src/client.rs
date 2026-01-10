use reqwest::header::HeaderMap;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://slack.com/api";

pub struct ClientBuilder<A> {
    auth: A,
    base_url: String,
}

impl ClientBuilder<()> {
    pub fn new() -> Self {
        Self {
            auth: (),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }

    pub fn auth<S: AuthStrategy + 'static>(self, auth: S) -> ClientBuilder<S> {
        ClientBuilder {
            auth,
            base_url: self.base_url,
        }
    }
}

impl Default for ClientBuilder<()> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: AuthStrategy + 'static> ClientBuilder<A> {
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn build(self) -> Client {
        Client {
            http: reqwest::Client::new(),
            auth: Arc::new(self.auth),
            base_url: self.base_url,
        }
    }
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    auth: Arc<dyn AuthStrategy>,
    base_url: String,
}

impl Client {
    pub fn builder() -> ClientBuilder<()> {
        ClientBuilder::new()
    }

    async fn post<T, B>(&self, method: &str, body: &B) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
        B: serde::Serialize,
    {
        let url = format!("{}/{}", self.base_url, method);
        debug!("Slack API request: POST {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;
        headers.insert("Content-Type", "application/json; charset=utf-8".parse().unwrap());

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn get<T>(&self, method: &str, params: &[(&str, &str)]) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let url = format!("{}/{}", self.base_url, method);
        debug!("Slack API request: GET {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .query(params)
            .send()
            .await?;

        self.handle_response(response).await
    }

    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let status = response.status();

        if status == 429 {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(60);
            return Err(Error::RateLimited { retry_after });
        }

        let body = response.text().await?;
        let slack_resp: SlackResponse<T> = serde_json::from_str(&body)?;

        if slack_resp.ok {
            slack_resp.data.ok_or_else(|| Error::SlackApi("No data returned".to_string()))
        } else {
            let error = slack_resp.error.unwrap_or_else(|| "unknown_error".to_string());
            warn!("Slack API error: {}", error);

            match error.as_str() {
                "invalid_auth" | "not_authed" => Err(Error::Unauthorized),
                "channel_not_found" => Err(Error::ChannelNotFound(error)),
                "user_not_found" => Err(Error::UserNotFound(error)),
                _ => Err(Error::SlackApi(error)),
            }
        }
    }

    /// Post a message to a channel.
    pub async fn post_message(&self, request: PostMessageRequest) -> Result<MessageResponse> {
        self.post("chat.postMessage", &request).await
    }

    /// Update a message.
    pub async fn update_message(&self, request: UpdateMessageRequest) -> Result<MessageResponse> {
        self.post("chat.update", &request).await
    }

    /// Delete a message.
    pub async fn delete_message(&self, channel: &str, ts: &str) -> Result<()> {
        #[derive(serde::Serialize)]
        struct Request<'a> {
            channel: &'a str,
            ts: &'a str,
        }

        let _: serde_json::Value = self.post("chat.delete", &Request { channel, ts }).await?;
        Ok(())
    }

    /// List channels.
    pub async fn list_channels(&self, cursor: Option<&str>) -> Result<ChannelsListResponse> {
        let mut params = vec![("types", "public_channel,private_channel")];
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        self.get("conversations.list", &params).await
    }

    /// Get channel info.
    pub async fn get_channel(&self, channel_id: &str) -> Result<Channel> {
        #[derive(serde::Deserialize)]
        struct Response {
            channel: Channel,
        }
        let resp: Response = self.get("conversations.info", &[("channel", channel_id)]).await?;
        Ok(resp.channel)
    }

    /// List users.
    pub async fn list_users(&self, cursor: Option<&str>) -> Result<UsersListResponse> {
        let mut params = vec![];
        if let Some(c) = cursor {
            params.push(("cursor", c));
        }
        self.get("users.list", &params).await
    }

    /// Get user info.
    pub async fn get_user(&self, user_id: &str) -> Result<User> {
        #[derive(serde::Deserialize)]
        struct Response {
            user: User,
        }
        let resp: Response = self.get("users.info", &[("user", user_id)]).await?;
        Ok(resp.user)
    }

    /// Add a reaction to a message.
    pub async fn add_reaction(&self, request: ReactionRequest) -> Result<()> {
        let _: serde_json::Value = self.post("reactions.add", &request).await?;
        Ok(())
    }

    /// Remove a reaction from a message.
    pub async fn remove_reaction(&self, request: ReactionRequest) -> Result<()> {
        let _: serde_json::Value = self.post("reactions.remove", &request).await?;
        Ok(())
    }

    /// Upload a file.
    pub async fn upload_file(
        &self,
        channels: &[&str],
        content: Vec<u8>,
        filename: &str,
        title: Option<&str>,
    ) -> Result<FileUploadResponse> {
        let url = format!("{}/files.upload", self.base_url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let mut form = reqwest::multipart::Form::new()
            .text("channels", channels.join(","))
            .text("filename", filename.to_string())
            .part(
                "file",
                reqwest::multipart::Part::bytes(content).file_name(filename.to_string()),
            );

        if let Some(t) = title {
            form = form.text("title", t.to_string());
        }

        let response = self.http.post(&url).headers(headers).multipart(form).send().await?;
        self.handle_response(response).await
    }

    /// Test authentication.
    pub async fn auth_test(&self) -> Result<AuthTestResponse> {
        self.post("auth.test", &serde_json::json!({})).await
    }
}

/// Auth test response.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct AuthTestResponse {
    pub url: String,
    pub team: String,
    pub user: String,
    pub team_id: String,
    pub user_id: String,
    pub bot_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::BotTokenAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(BotTokenAuth::new("xoxb-token"))
            .build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }
}
