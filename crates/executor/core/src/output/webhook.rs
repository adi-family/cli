use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use tracing::info;

use crate::error::{ExecutorError, Result};
use crate::types::OutputFile;

#[derive(Debug, Serialize)]
struct WebhookPayload<'a> {
    files: &'a [OutputFile],
}

pub async fn send_webhook(
    url: &str,
    headers: Option<&HashMap<String, String>>,
    files: &[OutputFile],
) -> Result<()> {
    info!(url = %url, files = files.len(), "Sending webhook");

    let client = Client::builder()
        .user_agent("adi-executor/0.1")
        .build()
        .map_err(|e| ExecutorError::WebhookFailed(e.to_string()))?;

    let payload = WebhookPayload { files };

    let mut request = client.post(url).json(&payload);

    if let Some(hdrs) = headers {
        for (key, value) in hdrs {
            request = request.header(key, value);
        }
    }

    let response = request
        .send()
        .await
        .map_err(|e| ExecutorError::WebhookFailed(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown".to_string());
        return Err(ExecutorError::WebhookFailed(format!(
            "HTTP {}: {}",
            status, body
        )));
    }

    info!(url = %url, "Webhook sent successfully");

    Ok(())
}
