use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

use crate::config::PaddleConfig;
use crate::models::{
    CheckoutSession, CreateCheckoutRequest, CreateSubscriptionRequest, ParsedWebhookEvent,
    Subscription,
};
use crate::types::{PaymentStatus, ProviderType, SubscriptionStatus};

use super::{PaymentProvider, ProviderError};

fn paddle_api_url(sandbox: bool) -> &'static str {
    if sandbox {
        "https://sandbox-api.paddle.com"
    } else {
        "https://api.paddle.com"
    }
}

pub struct PaddleProvider {
    config: PaddleConfig,
    client: reqwest::Client,
}

impl PaddleProvider {
    pub fn new(config: PaddleConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    fn api_url(&self) -> &'static str {
        paddle_api_url(self.config.sandbox)
    }
}

#[async_trait]
impl PaymentProvider for PaddleProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Paddle
    }

    async fn create_checkout(
        &self,
        req: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, ProviderError> {
        let body = serde_json::json!({
            "items": [{
                "price_id": req.metadata.as_ref()
                    .and_then(|m| m.get("price_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProviderError::Request(
                        "metadata.price_id is required for Paddle checkout".to_string()
                    ))?,
                "quantity": 1,
            }],
            "custom_data": req.metadata,
        });

        let resp = self
            .client
            .post(format!("{}/transactions", self.api_url()))
            .bearer_auth(&self.config.api_key)
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
                "Paddle API returned {status}: {text}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let data = &json["data"];
        let txn_id = data["id"]
            .as_str()
            .ok_or_else(|| ProviderError::InvalidResponse("missing transaction id".to_string()))?;
        let checkout_url = data["checkout"]["url"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(CheckoutSession {
            id: Uuid::new_v4(),
            provider: ProviderType::Paddle.to_string(),
            provider_payment_id: txn_id.to_string(),
            checkout_url,
            conversion_rate: 0.0,
            expected_credits: 0,
            status: "pending".to_string(),
        })
    }

    async fn create_subscription(
        &self,
        req: &CreateSubscriptionRequest,
    ) -> Result<Subscription, ProviderError> {
        // Paddle subscriptions are created via transactions with recurring price IDs.
        // The subscription object is created automatically by Paddle after checkout.
        // We create a transaction and return a stub subscription to be updated via webhook.
        let price_id = req
            .metadata
            .as_ref()
            .and_then(|m| m.get("price_id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ProviderError::Request(
                    "metadata.price_id is required for Paddle subscription".to_string(),
                )
            })?;

        let body = serde_json::json!({
            "items": [{
                "price_id": price_id,
                "quantity": 1,
            }],
            "custom_data": req.metadata,
        });

        let resp = self
            .client
            .post(format!("{}/transactions", self.api_url()))
            .bearer_auth(&self.config.api_key)
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
                "Paddle API returned {status}: {text}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let txn_id = json["data"]["id"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(Subscription {
            id: Uuid::new_v4(),
            provider: ProviderType::Paddle.to_string(),
            provider_subscription_id: Some(txn_id),
            user_id: Uuid::nil(),
            plan_id: req.plan_id.clone(),
            status: "pending".to_string(),
            billing_interval: req.billing_interval.as_ref().map(|b| b.to_string()),
            amount_cents: None,
            currency: None,
            current_period_start: None,
            current_period_end: None,
            metadata: req.metadata.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    async fn cancel_subscription(&self, provider_id: &str) -> Result<(), ProviderError> {
        let body = serde_json::json!({
            "effective_from": "next_billing_period",
        });

        let resp = self
            .client
            .post(format!(
                "{}/subscriptions/{}/cancel",
                self.api_url(),
                provider_id
            ))
            .bearer_auth(&self.config.api_key)
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
                "Paddle API returned {status}: {text}"
            )));
        }

        Ok(())
    }

    async fn get_subscription(&self, provider_id: &str) -> Result<Subscription, ProviderError> {
        let resp = self
            .client
            .get(format!(
                "{}/subscriptions/{}",
                self.api_url(),
                provider_id
            ))
            .bearer_auth(&self.config.api_key)
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
                "Paddle API returned {status}: {text}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let data = &json["data"];
        let status = data["status"].as_str().unwrap_or("unknown").to_string();

        Ok(Subscription {
            id: Uuid::new_v4(),
            provider: ProviderType::Paddle.to_string(),
            provider_subscription_id: Some(provider_id.to_string()),
            user_id: Uuid::nil(),
            plan_id: data["items"][0]["price"]["id"]
                .as_str()
                .unwrap_or("")
                .to_string(),
            status,
            billing_interval: data["billing_cycle"]["interval"]
                .as_str()
                .map(|s| s.to_string()),
            amount_cents: data["items"][0]["price"]["unit_price"]["amount"]
                .as_str()
                .and_then(|s| s.parse::<i64>().ok()),
            currency: data["currency_code"]
                .as_str()
                .map(|s| s.to_string()),
            current_period_start: None,
            current_period_end: None,
            metadata: data["custom_data"].as_object().map(|o| {
                serde_json::Value::Object(o.clone())
            }),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    fn verify_webhook(&self, payload: &[u8], signature: &str) -> Result<bool, ProviderError> {
        // Paddle uses ts;h1=<hash> signature format
        let parts: Vec<&str> = signature.split(';').collect();
        if parts.len() < 2 {
            return Err(ProviderError::InvalidSignature);
        }

        let ts = parts[0]
            .strip_prefix("ts=")
            .ok_or(ProviderError::InvalidSignature)?;
        let h1 = parts[1]
            .strip_prefix("h1=")
            .ok_or(ProviderError::InvalidSignature)?;

        // Build signed payload: ts:payload
        let signed_payload = format!(
            "{}:{}",
            ts,
            std::str::from_utf8(payload).map_err(|_| ProviderError::InvalidSignature)?
        );

        let mut mac = Hmac::<Sha256>::new_from_slice(self.config.webhook_secret.as_bytes())
            .map_err(|_| ProviderError::InvalidSignature)?;
        mac.update(signed_payload.as_bytes());

        let expected = hex::decode(h1).map_err(|_| ProviderError::InvalidSignature)?;
        Ok(mac.verify_slice(&expected).is_ok())
    }

    fn parse_webhook(&self, payload: &[u8]) -> Result<ParsedWebhookEvent, ProviderError> {
        let json: serde_json::Value = serde_json::from_slice(payload)
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;

        let event_type = json["event_type"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        match event_type.as_str() {
            "transaction.completed" => {
                let txn_id = json["data"]["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(ParsedWebhookEvent::PaymentCompleted {
                    provider_payment_id: txn_id,
                    status: PaymentStatus::Completed,
                })
            }
            "transaction.payment_failed" => {
                let txn_id = json["data"]["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(ParsedWebhookEvent::PaymentFailed {
                    provider_payment_id: txn_id,
                    status: PaymentStatus::Failed,
                })
            }
            "subscription.activated" | "subscription.updated" | "subscription.resumed" => {
                let sub_id = json["data"]["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                let status = match json["data"]["status"].as_str().unwrap_or("active") {
                    "active" => SubscriptionStatus::Active,
                    "past_due" => SubscriptionStatus::PastDue,
                    "paused" => SubscriptionStatus::Paused,
                    "trialing" => SubscriptionStatus::Trialing,
                    _ => SubscriptionStatus::Active,
                };
                Ok(ParsedWebhookEvent::SubscriptionUpdated {
                    provider_subscription_id: sub_id,
                    status,
                })
            }
            "subscription.canceled" => {
                let sub_id = json["data"]["id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(ParsedWebhookEvent::SubscriptionCancelled {
                    provider_subscription_id: sub_id,
                })
            }
            _ => Ok(ParsedWebhookEvent::Unknown { event_type }),
        }
    }
}
