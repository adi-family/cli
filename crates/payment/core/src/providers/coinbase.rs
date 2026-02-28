use async_trait::async_trait;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::config::CoinbaseConfig;
use crate::models::{
    CheckoutSession, CreateCheckoutRequest, CreateSubscriptionRequest, ParsedWebhookEvent,
    Subscription,
};
use crate::types::{PaymentStatus, ProviderType};

use super::{PaymentProvider, ProviderError};

const COINBASE_API_URL: &str = "https://api.commerce.coinbase.com";

pub struct CoinbaseProvider {
    config: CoinbaseConfig,
    client: reqwest::Client,
}

impl CoinbaseProvider {
    pub fn new(config: CoinbaseConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentProvider for CoinbaseProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Coinbase
    }

    async fn create_checkout(
        &self,
        req: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, ProviderError> {
        let body = serde_json::json!({
            "name": req.name.as_deref().unwrap_or("Payment"),
            "description": req.description.as_deref().unwrap_or(""),
            "pricing_type": "fixed_price",
            "local_price": {
                "amount": format!("{:.2}", req.amount_cents as f64 / 100.0),
                "currency": req.currency,
            },
            "redirect_url": req.success_url,
            "cancel_url": req.cancel_url,
            "metadata": req.metadata,
        });

        let resp = self
            .client
            .post(format!("{COINBASE_API_URL}/charges"))
            .header("X-CC-Api-Key", &self.config.api_key)
            .header("X-CC-Version", "2018-03-22")
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderError::Request(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp
                .text()
                .await
                .unwrap_or_else(|_| "unknown error".to_string());
            return Err(ProviderError::Request(format!(
                "Coinbase API returned {status}: {text}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let data = &json["data"];
        let charge_id = data["id"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("missing charge id".to_string()))?;
        let hosted_url = data["hosted_url"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("missing hosted_url".to_string()))?;

        Ok(CheckoutSession {
            id: Uuid::new_v4(),
            provider: ProviderType::Coinbase.to_string(),
            provider_payment_id: charge_id.to_string(),
            checkout_url: hosted_url.to_string(),
            status: "pending".to_string(),
        })
    }

    async fn create_subscription(
        &self,
        _req: &CreateSubscriptionRequest,
    ) -> Result<Subscription, ProviderError> {
        Err(ProviderError::NotSupported(
            "Coinbase Commerce does not support subscriptions".to_string(),
        ))
    }

    async fn cancel_subscription(&self, _provider_id: &str) -> Result<(), ProviderError> {
        Err(ProviderError::NotSupported(
            "Coinbase Commerce does not support subscriptions".to_string(),
        ))
    }

    async fn get_subscription(&self, _provider_id: &str) -> Result<Subscription, ProviderError> {
        Err(ProviderError::NotSupported(
            "Coinbase Commerce does not support subscriptions".to_string(),
        ))
    }

    fn verify_webhook(&self, payload: &[u8], signature: &str) -> Result<bool, ProviderError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.webhook_secret.as_bytes())
            .map_err(|_| ProviderError::InvalidSignature)?;
        mac.update(payload);

        let expected = hex::decode(signature).map_err(|_| ProviderError::InvalidSignature)?;
        Ok(mac.verify_slice(&expected).is_ok())
    }

    fn parse_webhook(&self, payload: &[u8]) -> Result<ParsedWebhookEvent, ProviderError> {
        let json: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let event_type = json["event"]["type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        let charge_id = json["event"]["data"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        match event_type.as_str() {
            "charge:confirmed" | "charge:resolved" => Ok(ParsedWebhookEvent::PaymentCompleted {
                provider_payment_id: charge_id,
                status: PaymentStatus::Completed,
            }),
            "charge:failed" => Ok(ParsedWebhookEvent::PaymentFailed {
                provider_payment_id: charge_id,
                status: PaymentStatus::Failed,
            }),
            "charge:expired" => Ok(ParsedWebhookEvent::PaymentFailed {
                provider_payment_id: charge_id,
                status: PaymentStatus::Expired,
            }),
            _ => Ok(ParsedWebhookEvent::Unknown { event_type }),
        }
    }
}
