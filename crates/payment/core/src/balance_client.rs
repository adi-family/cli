use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct DepositRequest {
    pub user_id: Uuid,
    pub amount: i64,
    pub description: Option<String>,
    pub reference_type: Option<String>,
    pub reference_id: Option<String>,
    pub idempotency_key: Option<String>,
}

pub async fn deposit(
    client: &reqwest::Client,
    balance_url: &str,
    req: &DepositRequest,
) -> Result<(), String> {
    let resp = client
        .post(format!("{balance_url}/transactions/deposit"))
        .json(req)
        .send()
        .await
        .map_err(|e| format!("Balance API request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "unknown error".to_string());
        return Err(format!("Balance API returned {status}: {text}"));
    }

    Ok(())
}
