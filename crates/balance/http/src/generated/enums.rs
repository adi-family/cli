//! Auto-generated enums from TypeSpec.
//! DO NOT EDIT.

use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    #[serde(rename = "deposit")]
    Deposit,
    #[serde(rename = "debit")]
    Debit,
    #[serde(rename = "adjustment")]
    Adjustment,
    #[serde(rename = "transfer_in")]
    TransferIn,
    #[serde(rename = "transfer_out")]
    TransferOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "reversed")]
    Reversed,
}
