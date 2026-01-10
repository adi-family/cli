use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::auth::AuthStrategy;
use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://api.trello.com/1";

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

    fn build_url(&self, path: &str, extra_params: &[(&str, &str)]) -> String {
        let auth_params = self.auth.query_params();
        let mut all_params: Vec<(String, String)> = auth_params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();

        for (k, v) in extra_params {
            all_params.push((k.to_string(), v.to_string()));
        }

        let query: String = all_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}{}?{}", self.base_url, path, query)
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(reqwest::Method::GET, path, &[], None::<&()>)
            .await
    }

    async fn post<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(reqwest::Method::POST, path, &[], Some(body))
            .await
    }

    async fn put<T: DeserializeOwned, B: serde::Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        self.request(reqwest::Method::PUT, path, &[], Some(body))
            .await
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let url = self.build_url(path, &[]);
        debug!("Trello API request: DELETE {}", url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let response = self.http.delete(&url).headers(headers).send().await?;
        let status = response.status();

        if status.is_success() {
            Ok(())
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            Err(Error::Api {
                status: status_code,
                message: body,
            })
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        extra_params: &[(&str, &str)],
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = self.build_url(path, extra_params);
        debug!("Trello API request: {} {}", method, url);

        let mut headers = HeaderMap::new();
        self.auth.apply(&mut headers).await?;

        let mut request = self.http.request(method, &url).headers(headers);

        if let Some(body) = body {
            request = request.json(body);
        }

        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            let body = response.text().await?;
            serde_json::from_str(&body).map_err(Error::from)
        } else {
            let status_code = status.as_u16();
            let body = response.text().await.unwrap_or_default();
            warn!("Trello API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                403 => Err(Error::Forbidden(body)),
                404 => Err(Error::NotFound(body)),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get a board by ID.
    pub async fn get_board(&self, id: &str) -> Result<Board> {
        self.get(&format!("/boards/{}", id)).await
    }

    /// List boards for the authenticated member.
    pub async fn list_boards(&self) -> Result<Vec<Board>> {
        self.get("/members/me/boards").await
    }

    /// Create a board.
    pub async fn create_board(&self, input: CreateBoardInput) -> Result<Board> {
        self.post("/boards", &input).await
    }

    /// Get a card by ID.
    pub async fn get_card(&self, id: &str) -> Result<Card> {
        self.get(&format!("/cards/{}", id)).await
    }

    /// Create a card.
    pub async fn create_card(&self, input: CreateCardInput) -> Result<Card> {
        self.post("/cards", &input).await
    }

    /// Update a card.
    pub async fn update_card(&self, id: &str, input: UpdateCardInput) -> Result<Card> {
        self.put(&format!("/cards/{}", id), &input).await
    }

    /// Move a card to a different list.
    pub async fn move_card(&self, id: &str, list_id: &str) -> Result<Card> {
        self.update_card(id, UpdateCardInput::new().id_list(list_id))
            .await
    }

    /// Archive a card.
    pub async fn archive_card(&self, id: &str) -> Result<Card> {
        self.update_card(id, UpdateCardInput::new().closed(true))
            .await
    }

    /// Delete a card.
    pub async fn delete_card(&self, id: &str) -> Result<()> {
        self.delete(&format!("/cards/{}", id)).await
    }

    /// List lists on a board.
    pub async fn list_lists(&self, board_id: &str) -> Result<Vec<List>> {
        self.get(&format!("/boards/{}/lists", board_id)).await
    }

    /// Create a list.
    pub async fn create_list(&self, input: CreateListInput) -> Result<List> {
        self.post("/lists", &input).await
    }

    /// List cards in a list.
    pub async fn list_cards(&self, list_id: &str) -> Result<Vec<Card>> {
        self.get(&format!("/lists/{}/cards", list_id)).await
    }

    /// List labels on a board.
    pub async fn list_labels(&self, board_id: &str) -> Result<Vec<Label>> {
        self.get(&format!("/boards/{}/labels", board_id)).await
    }

    /// List members of a board.
    pub async fn list_members(&self, board_id: &str) -> Result<Vec<Member>> {
        self.get(&format!("/boards/{}/members", board_id)).await
    }

    /// Get the authenticated member.
    pub async fn me(&self) -> Result<Member> {
        self.get("/members/me").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::ApiKeyAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("api-key", "token"))
            .build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }

    #[test]
    fn test_build_url() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("my-key", "my-token"))
            .build();

        let url = client.build_url("/boards/123", &[]);
        assert!(url.contains("key=my-key"));
        assert!(url.contains("token=my-token"));
    }
}
