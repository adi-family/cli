mod coinbase;
mod paddle;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::config::Config;
use crate::error::ApiError;
use crate::models::{
    CheckoutSession, CreateCheckoutRequest, CreateSubscriptionRequest, ParsedWebhookEvent,
    Subscription,
};
use crate::types::ProviderType;

pub use coinbase::CoinbaseProvider;
pub use paddle::PaddleProvider;

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("API request failed: {0}")]
    Request(String),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Signature verification failed")]
    InvalidSignature,

    #[error("Operation not supported by this provider: {0}")]
    NotSupported(String),
}

impl From<ProviderError> for ApiError {
    fn from(e: ProviderError) -> Self {
        match e {
            ProviderError::NotSupported(msg) => ApiError::NotSupported(msg),
            other => ApiError::Provider(other.to_string()),
        }
    }
}

#[async_trait]
pub trait PaymentProvider: Send + Sync {
    fn provider_type(&self) -> ProviderType;

    async fn create_checkout(
        &self,
        req: &CreateCheckoutRequest,
    ) -> Result<CheckoutSession, ProviderError>;

    async fn create_subscription(
        &self,
        req: &CreateSubscriptionRequest,
    ) -> Result<Subscription, ProviderError>;

    async fn cancel_subscription(&self, provider_id: &str) -> Result<(), ProviderError>;

    async fn get_subscription(&self, provider_id: &str) -> Result<Subscription, ProviderError>;

    fn verify_webhook(&self, payload: &[u8], signature: &str) -> Result<bool, ProviderError>;

    fn parse_webhook(&self, payload: &[u8]) -> Result<ParsedWebhookEvent, ProviderError>;
}

pub fn create_providers(config: &Config) -> HashMap<ProviderType, Box<dyn PaymentProvider>> {
    let mut providers: HashMap<ProviderType, Box<dyn PaymentProvider>> = HashMap::new();

    if let Some(ref coinbase_config) = config.coinbase {
        providers.insert(
            ProviderType::Coinbase,
            Box::new(CoinbaseProvider::new(coinbase_config.clone())),
        );
        tracing::info!("Coinbase Commerce provider enabled");
    }

    if let Some(ref paddle_config) = config.paddle {
        providers.insert(
            ProviderType::Paddle,
            Box::new(PaddleProvider::new(paddle_config.clone())),
        );
        tracing::info!("Paddle provider enabled");
    }

    providers
}
