use lib_client_google_auth::AuthStrategy;
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use tracing::{debug, warn};

use crate::error::{Error, Result};
use crate::types::*;

const DEFAULT_BASE_URL: &str = "https://sheets.googleapis.com/v4";

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

    async fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&impl serde::Serialize>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        debug!("Sheets API request: {} {}", method, url);

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
            warn!("Sheets API error ({}): {}", status_code, body);

            match status_code {
                401 => Err(Error::Unauthorized),
                404 => Err(Error::NotFound(body)),
                429 => Err(Error::RateLimited { retry_after: 60 }),
                _ => Err(Error::Api {
                    status: status_code,
                    message: body,
                }),
            }
        }
    }

    /// Get a spreadsheet.
    pub async fn get_spreadsheet(&self, id: &str) -> Result<Spreadsheet> {
        self.request(
            reqwest::Method::GET,
            &format!("/spreadsheets/{}", id),
            None::<&()>,
        )
        .await
    }

    /// Create a spreadsheet.
    pub async fn create_spreadsheet(&self, request: CreateSpreadsheetRequest) -> Result<Spreadsheet> {
        self.request(reqwest::Method::POST, "/spreadsheets", Some(&request))
            .await
    }

    /// Get values from a range.
    pub async fn get_values(&self, spreadsheet_id: &str, range: &str) -> Result<ValueRange> {
        let encoded_range = urlencoding::encode(range);
        self.request(
            reqwest::Method::GET,
            &format!("/spreadsheets/{}/values/{}", spreadsheet_id, encoded_range),
            None::<&()>,
        )
        .await
    }

    /// Update values in a range.
    pub async fn update_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: ValueRange,
        input_option: ValueInputOption,
    ) -> Result<UpdateValuesResponse> {
        let encoded_range = urlencoding::encode(range);
        self.request(
            reqwest::Method::PUT,
            &format!(
                "/spreadsheets/{}/values/{}?valueInputOption={}",
                spreadsheet_id,
                encoded_range,
                input_option.as_str()
            ),
            Some(&values),
        )
        .await
    }

    /// Append values to a range.
    pub async fn append_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: ValueRange,
        input_option: ValueInputOption,
        insert_option: InsertDataOption,
    ) -> Result<AppendValuesResponse> {
        let encoded_range = urlencoding::encode(range);
        self.request(
            reqwest::Method::POST,
            &format!(
                "/spreadsheets/{}/values/{}:append?valueInputOption={}&insertDataOption={}",
                spreadsheet_id,
                encoded_range,
                input_option.as_str(),
                insert_option.as_str()
            ),
            Some(&values),
        )
        .await
    }

    /// Clear values in a range.
    pub async fn clear_values(&self, spreadsheet_id: &str, range: &str) -> Result<()> {
        let encoded_range = urlencoding::encode(range);
        let _: serde_json::Value = self
            .request(
                reqwest::Method::POST,
                &format!(
                    "/spreadsheets/{}/values/{}:clear",
                    spreadsheet_id, encoded_range
                ),
                Some(&serde_json::json!({})),
            )
            .await?;
        Ok(())
    }

    /// Batch update spreadsheet (add/delete sheets, etc.).
    pub async fn batch_update(
        &self,
        spreadsheet_id: &str,
        request: BatchUpdateRequest,
    ) -> Result<BatchUpdateResponse> {
        self.request(
            reqwest::Method::POST,
            &format!("/spreadsheets/{}:batchUpdate", spreadsheet_id),
            Some(&request),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_client_google_auth::ApiKeyAuth;

    #[test]
    fn test_builder() {
        let client = Client::builder()
            .auth(ApiKeyAuth::new("test-key"))
            .build();
        assert_eq!(client.base_url, DEFAULT_BASE_URL);
    }
}
