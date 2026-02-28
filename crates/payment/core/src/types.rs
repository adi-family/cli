use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Coinbase,
    Paddle,
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coinbase => write!(f, "coinbase"),
            Self::Paddle => write!(f, "paddle"),
        }
    }
}

impl ProviderType {
    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s {
            "coinbase" => Some(Self::Coinbase),
            "paddle" => Some(Self::Paddle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Cancelled,
    Refunded,
    Expired,
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Refunded => write!(f, "refunded"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    PastDue,
    Cancelled,
    Paused,
    Trialing,
    Expired,
}

impl fmt::Display for SubscriptionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::PastDue => write!(f, "past_due"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Paused => write!(f, "paused"),
            Self::Trialing => write!(f, "trialing"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BillingInterval {
    Monthly,
    Yearly,
}

impl fmt::Display for BillingInterval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Monthly => write!(f, "monthly"),
            Self::Yearly => write!(f, "yearly"),
        }
    }
}
