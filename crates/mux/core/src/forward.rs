use bytes::Bytes;
use futures::future::select_all;
use reqwest::Client;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::types::{Backend, BackendResponse};

type BoxFut<T> = Pin<Box<dyn Future<Output = T> + Send>>;

async fn send_request(
    client: Client,
    backend_url: String,
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Bytes,
    timeout_ms: u64,
) -> Option<BackendResponse> {
    let url = format!("{}{}", backend_url.trim_end_matches('/'), path);

    let mut req = client
        .request(method.parse().unwrap_or(reqwest::Method::GET), &url)
        .timeout(Duration::from_millis(timeout_ms))
        .body(body);

    for (name, value) in &headers {
        if let (Ok(n), Ok(v)) = (
            reqwest::header::HeaderName::from_bytes(name.as_bytes()),
            reqwest::header::HeaderValue::from_str(value),
        ) {
            req = req.header(n, v);
        }
    }

    let resp = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(url = %url, error = %e, "backend request failed");
            return None;
        }
    };

    let status = resp.status().as_u16();
    let resp_headers: Vec<(String, String)> = resp
        .headers()
        .iter()
        .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_string())))
        .collect();

    match resp.bytes().await {
        Ok(body) => Some(BackendResponse {
            backend_url,
            status,
            body,
            headers: resp_headers,
        }),
        Err(e) => {
            tracing::warn!(url = %url, error = %e, "failed to read backend body");
            None
        }
    }
}

fn build_futures(
    client: &Client,
    backends: &[Backend],
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Bytes,
    timeout_ms: u64,
) -> Vec<BoxFut<Option<BackendResponse>>> {
    backends
        .iter()
        .filter(|b| b.enabled)
        .map(|b| {
            // reqwest::Client is cheap to clone (internally Arc-wrapped).
            let fut = send_request(
                client.clone(),
                b.url.clone(),
                method.to_string(),
                path.to_string(),
                headers.to_vec(),
                body.clone(),
                timeout_ms,
            );
            Box::pin(fut) as BoxFut<Option<BackendResponse>>
        })
        .collect()
}

/// Send to all enabled backends concurrently and collect responses.
pub async fn forward_all(
    client: &Client,
    backends: &[Backend],
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Bytes,
    timeout_ms: u64,
) -> Vec<BackendResponse> {
    let futs = build_futures(client, backends, method, path, headers, body, timeout_ms);
    futures::future::join_all(futs)
        .await
        .into_iter()
        .flatten()
        .collect()
}

/// Send to all backends concurrently, return the first successful (2xx) response.
pub async fn forward_first(
    client: &Client,
    backends: &[Backend],
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Bytes,
    timeout_ms: u64,
) -> Option<BackendResponse> {
    let responses = forward_all(client, backends, method, path, headers, body, timeout_ms).await;
    responses.into_iter().find(|r| r.status < 300)
}

/// Race all backends, return whichever responds first.
pub async fn forward_fastest(
    client: &Client,
    backends: &[Backend],
    method: &str,
    path: &str,
    headers: &[(String, String)],
    body: Bytes,
    timeout_ms: u64,
) -> Option<BackendResponse> {
    let mut futs = build_futures(client, backends, method, path, headers, body, timeout_ms);
    if futs.is_empty() {
        return None;
    }

    loop {
        if futs.is_empty() {
            return None;
        }
        let (result, _idx, remaining) = select_all(futs).await;
        futs = remaining;
        if result.is_some() {
            return result;
        }
    }
}
