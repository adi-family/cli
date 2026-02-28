mod error;
mod types;

pub use error::PaymentClientError;
pub use types::{BalanceResponse, BalanceTransactionResponse};

use std::sync::Arc;

type Result<T> = std::result::Result<T, PaymentClientError>;

#[derive(Clone)]
pub struct PaymentClient {
    http: reqwest::Client,
    base_url: Arc<str>,
}

impl PaymentClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: Arc::from(base_url.into()),
        }
    }

    pub async fn get_balance(&self, token: &str) -> Result<BalanceResponse> {
        let url = format!("{}/balance", self.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| PaymentClientError::ServiceUnavailable(e.to_string()))?;

        self.parse_response(response).await
    }

    /// Ask the server whether this user can be charged more.
    ///
    /// The decision logic lives server-side (balance > 0, overdraft, etc.).
    pub async fn can_charge_more(&self, token: &str) -> Result<bool> {
        let url = format!("{}/balance/can-charge-more", self.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| PaymentClientError::ServiceUnavailable(e.to_string()))?;

        self.parse_response::<CanChargeMoreResponse>(response)
            .await
            .map(|r| r.allowed)
    }

    pub async fn list_transactions(
        &self,
        token: &str,
    ) -> Result<Vec<BalanceTransactionResponse>> {
        let url = format!("{}/balance/transactions", self.base_url);

        let response = self
            .http
            .get(&url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| PaymentClientError::ServiceUnavailable(e.to_string()))?;

        self.parse_response(response).await
    }

    async fn parse_response<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<T> {
        let status = response.status();

        if status.is_success() {
            return response.json().await.map_err(PaymentClientError::Http);
        }

        match status.as_u16() {
            401 => Err(PaymentClientError::Unauthorized),
            _ => {
                let message = response.text().await.unwrap_or_default();
                tracing::warn!(status = status.as_u16(), %message, "payment API error");
                Err(PaymentClientError::Api {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct CanChargeMoreResponse {
    allowed: bool,
}
