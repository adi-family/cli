use async_trait::async_trait;
use chrono::Utc;
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

    fn create_charge_body(
        &self,
        amount_cents: i64,
        currency: &str,
        name: &str,
        description: &str,
        success_url: Option<&str>,
        cancel_url: Option<&str>,
        metadata: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        serde_json::json!({
            "name": name,
            "description": description,
            "pricing_type": "fixed_price",
            "local_price": {
                "amount": format!("{:.2}", amount_cents as f64 / 100.0),
                "currency": currency,
            },
            "redirect_url": success_url,
            "cancel_url": cancel_url,
            "metadata": metadata,
        })
    }

    async fn post_charge(
        &self,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        let resp = self
            .client
            .post(format!("{COINBASE_API_URL}/charges"))
            .header("X-CC-Api-Key", &self.config.api_key)
            .header("X-CC-Version", "2018-03-22")
            .json(body)
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

        resp.json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))
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
        let body = self.create_charge_body(
            req.amount_cents,
            &req.currency,
            req.name.as_deref().unwrap_or("Payment"),
            req.description.as_deref().unwrap_or(""),
            req.success_url.as_deref(),
            req.cancel_url.as_deref(),
            req.metadata.as_ref(),
        );

        let json = self.post_charge(&body).await?;
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
        req: &CreateSubscriptionRequest,
    ) -> Result<Subscription, ProviderError> {
        let amount_cents = req.amount_cents.ok_or_else(|| {
            ProviderError::Request("amount_cents is required for Coinbase subscriptions".to_string())
        })?;
        let currency = req.currency.as_deref().unwrap_or("USD");

        let sub_id = Uuid::new_v4();

        let metadata = serde_json::json!({
            "subscription_id": sub_id.to_string(),
            "plan_id": req.plan_id,
            "type": "subscription",
        });

        let body = self.create_charge_body(
            amount_cents,
            currency,
            &format!("Subscription: {}", req.plan_id),
            "Subscription payment via crypto",
            req.success_url.as_deref(),
            req.cancel_url.as_deref(),
            Some(&metadata),
        );

        let json = self.post_charge(&body).await?;
        let data = &json["data"];

        let charge_id = data["id"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("missing charge id".to_string()))?;
        let checkout_url = data["hosted_url"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(Subscription {
            id: sub_id,
            provider: ProviderType::Coinbase.to_string(),
            provider_subscription_id: Some(sub_id.to_string()),
            user_id: Uuid::nil(),
            plan_id: req.plan_id.clone(),
            status: "pending".to_string(),
            billing_interval: req.billing_interval.as_ref().map(|b| b.to_string()),
            amount_cents: Some(amount_cents),
            currency: Some(currency.to_string()),
            current_period_start: None,
            current_period_end: None,
            metadata: Some(serde_json::json!({
                "checkout_url": checkout_url,
                "provider_payment_id": charge_id,
            })),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn cancel_subscription(&self, _provider_id: &str) -> Result<(), ProviderError> {
        // Coinbase subscriptions are managed locally — no external API call needed
        Ok(())
    }

    async fn get_subscription(&self, _provider_id: &str) -> Result<Subscription, ProviderError> {
        // Coinbase subscriptions are fetched from local DB by the handler
        Err(ProviderError::NotSupported(
            "Coinbase subscriptions are managed locally".to_string(),
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
