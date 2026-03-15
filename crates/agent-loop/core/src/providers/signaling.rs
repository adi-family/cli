/// LLM provider that connects to a signaling server and forwards
/// completion requests to the LLM proxy AdiService running in a cocoon.
///
/// Flow: AgentLoop → SignalingLlmProvider → WebSocket → Signaling Server
///       → sync_data relay → Cocoon → AdiRouter → LlmProxyService::complete
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use lib_signaling_protocol::SignalingMessage;
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, oneshot};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use uuid::Uuid;

use crate::error::{AgentError, Result};
use crate::llm::{LlmConfig, LlmProvider, LlmResponse, TokenUsage};
use crate::tool::ToolSchema;
use crate::types::{Message, ToolCall};

/// Configuration for connecting to a signaling server.
#[derive(Debug, Clone)]
pub struct SignalingConfig {
    /// WebSocket URL of the signaling server (e.g. `wss://kakun.example.com/ws`)
    pub signaling_url: String,
    /// JWT access token for signaling authentication
    pub access_token: String,
    /// LLM proxy token for authenticating with the LLM proxy service
    pub proxy_token: String,
    /// Target device ID of the cocoon running the LLM proxy (optional — uses pairing if absent)
    pub device_id: Option<String>,
    /// Pairing code to pair with a cocoon (used when device_id is absent)
    pub pairing_code: Option<String>,
}

/// Envelope for AdiService requests sent via sync_data relay.
#[derive(Debug, Serialize)]
struct AdiServiceRequest {
    adi_service: AdiRequestPayload,
}

#[derive(Debug, Serialize)]
struct AdiRequestPayload {
    id: String,
    plugin: String,
    method: String,
    params: serde_json::Value,
}

/// Envelope for AdiService responses received via sync_data relay.
#[derive(Debug, Deserialize)]
struct AdiServiceResponse {
    adi_service_response: AdiResponsePayload,
}

#[derive(Debug, Deserialize)]
struct AdiResponsePayload {
    id: String,
    status: String,
    #[serde(default)]
    data: serde_json::Value,
    #[serde(default)]
    error: Option<String>,
}

type PendingRequests = Arc<Mutex<HashMap<String, oneshot::Sender<AdiResponsePayload>>>>;

/// Handle to a signaling connection for sending messages.
struct SignalingHandle {
    tx: mpsc::UnboundedSender<WsMessage>,
    pending: PendingRequests,
}

/// LLM provider that routes requests through the signaling protocol.
pub struct SignalingLlmProvider {
    handle: Arc<SignalingHandle>,
    proxy_token: String,
}

impl SignalingLlmProvider {
    /// Connect to the signaling server and set up the provider.
    pub async fn connect(config: SignalingConfig) -> Result<Self> {
        let (ws, _) = tokio_tungstenite::connect_async(&config.signaling_url)
            .await
            .map_err(|e| AgentError::SignalingError(format!("WebSocket connect failed: {e}")))?;

        let (mut sink, mut stream) = ws.split();
        let pending: PendingRequests = Arc::new(Mutex::new(HashMap::new()));

        // Channel for outbound messages
        let (tx, mut rx) = mpsc::unbounded_channel::<WsMessage>();

        // Spawn writer task
        let writer_tx = tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
            let _ = sink.close().await;
        });

        // Wait for auth_hello from server
        Self::wait_for_auth_hello(&mut stream).await?;

        // Authenticate
        let auth_msg = SignalingMessage::AuthAuthenticate {
            access_token: config.access_token.clone(),
        };
        writer_tx
            .send(WsMessage::Text(serde_json::to_string(&auth_msg).unwrap().into()))
            .map_err(|_| AgentError::SignalingError("Send failed".into()))?;

        // Wait for auth response
        Self::wait_for_auth_response(&mut stream).await?;

        // Pair with cocoon if pairing_code provided, or use device_id
        if let Some(code) = &config.pairing_code {
            let pair_msg = SignalingMessage::PairingUseCode {
                code: code.clone(),
            };
            writer_tx
                .send(WsMessage::Text(serde_json::to_string(&pair_msg).unwrap().into()))
                .map_err(|_| AgentError::SignalingError("Send failed".into()))?;

            Self::wait_for_pairing(&mut stream).await?;
        }

        // Spawn reader task that routes sync_data responses to pending requests
        let reader_pending = pending.clone();
        tokio::spawn(async move {
            while let Some(Ok(WsMessage::Text(text))) = stream.next().await {
                if let Ok(msg) = serde_json::from_str::<serde_json::Value>(&text) {
                    // Check if it's a sync_data message containing an adi_service_response
                    if let Some(payload) = msg.get("payload").or_else(|| {
                        // Direct sync_data message
                        if msg.get("type").and_then(|t| t.as_str()) == Some("sync_data") {
                            msg.get("payload")
                        } else {
                            None
                        }
                    }) {
                        if let Ok(resp) =
                            serde_json::from_value::<AdiServiceResponse>(payload.clone())
                        {
                            let mut pending = reader_pending.lock().await;
                            if let Some(sender) = pending.remove(&resp.adi_service_response.id) {
                                let _ = sender.send(resp.adi_service_response);
                            }
                        }
                    }
                    // Also try parsing the whole message as sync_data
                    if let Ok(SignalingMessage::SyncData { payload }) =
                        serde_json::from_str::<SignalingMessage>(&text)
                    {
                        if let Ok(resp) =
                            serde_json::from_value::<AdiServiceResponse>(payload)
                        {
                            let mut pending = reader_pending.lock().await;
                            if let Some(sender) = pending.remove(&resp.adi_service_response.id) {
                                let _ = sender.send(resp.adi_service_response);
                            }
                        }
                    }
                }
            }
        });

        let handle = Arc::new(SignalingHandle { tx, pending });

        Ok(Self {
            handle,
            proxy_token: config.proxy_token,
        })
    }

    async fn wait_for_auth_hello(
        stream: &mut (impl StreamExt<Item = std::result::Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
                 + Unpin),
    ) -> Result<()> {
        while let Some(msg) = stream.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                if let Ok(SignalingMessage::AuthHello { .. }) =
                    serde_json::from_str::<SignalingMessage>(&text)
                {
                    return Ok(());
                }
            }
        }
        Err(AgentError::SignalingError(
            "Connection closed before auth_hello".into(),
        ))
    }

    async fn wait_for_auth_response(
        stream: &mut (impl StreamExt<Item = std::result::Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
                 + Unpin),
    ) -> Result<()> {
        while let Some(msg) = stream.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                if let Ok(SignalingMessage::AuthAuthenticateResponse { .. }) =
                    serde_json::from_str::<SignalingMessage>(&text)
                {
                    return Ok(());
                }
                if let Ok(SignalingMessage::SystemError { message }) =
                    serde_json::from_str::<SignalingMessage>(&text)
                {
                    return Err(AgentError::SignalingError(format!(
                        "Auth failed: {message}"
                    )));
                }
            }
        }
        Err(AgentError::SignalingError(
            "Connection closed during authentication".into(),
        ))
    }

    async fn wait_for_pairing(
        stream: &mut (impl StreamExt<Item = std::result::Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
                 + Unpin),
    ) -> Result<()> {
        while let Some(msg) = stream.next().await {
            if let Ok(WsMessage::Text(text)) = msg {
                if let Ok(SignalingMessage::PairingUseCodeResponse { .. }) =
                    serde_json::from_str::<SignalingMessage>(&text)
                {
                    return Ok(());
                }
                if let Ok(SignalingMessage::PairingFailed { reason }) =
                    serde_json::from_str::<SignalingMessage>(&text)
                {
                    return Err(AgentError::SignalingError(format!(
                        "Pairing failed: {reason}"
                    )));
                }
            }
        }
        Err(AgentError::SignalingError(
            "Connection closed during pairing".into(),
        ))
    }

    /// Send an AdiService request via sync_data and wait for the response.
    async fn adi_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let request_id = Uuid::new_v4().to_string();

        let (resp_tx, resp_rx) = oneshot::channel();

        {
            let mut pending = self.handle.pending.lock().await;
            pending.insert(request_id.clone(), resp_tx);
        }

        let request = AdiServiceRequest {
            adi_service: AdiRequestPayload {
                id: request_id.clone(),
                plugin: "adi.llm-proxy".to_string(),
                method: method.to_string(),
                params,
            },
        };

        let sync_msg = SignalingMessage::SyncData {
            payload: serde_json::to_value(&request)
                .map_err(|e| AgentError::SerializationError(e))?,
        };

        self.handle
            .tx
            .send(WsMessage::Text(
                serde_json::to_string(&sync_msg).unwrap().into(),
            ))
            .map_err(|_| AgentError::SignalingError("WebSocket connection closed".into()))?;

        let response = resp_rx
            .await
            .map_err(|_| AgentError::SignalingError("Response channel closed".into()))?;

        if response.status == "success" {
            Ok(response.data)
        } else {
            Err(AgentError::LlmError(
                response
                    .error
                    .unwrap_or_else(|| "Unknown AdiService error".into()),
            ))
        }
    }
}

#[async_trait]
impl LlmProvider for SignalingLlmProvider {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSchema],
        config: &LlmConfig,
    ) -> Result<LlmResponse> {
        let openai_messages = messages_to_openai(messages);
        let openai_tools = tools_to_openai(tools);

        let mut body = serde_json::json!({
            "model": config.model,
            "messages": openai_messages,
            "temperature": config.temperature,
            "max_tokens": config.max_tokens,
        });

        if !openai_tools.is_empty() {
            body["tools"] = serde_json::json!(openai_tools);
        }
        if let Some(top_p) = config.top_p {
            body["top_p"] = serde_json::json!(top_p);
        }
        if let Some(ref stops) = config.stop_sequences {
            body["stop"] = serde_json::json!(stops);
        }

        let params = serde_json::json!({
            "proxy_token": self.proxy_token,
            "endpoint": "/v1/chat/completions",
            "body": body,
        });

        let response_body = self.adi_request("complete", params).await?;

        parse_openai_response(response_body)
    }

    fn name(&self) -> &str {
        "signaling"
    }

    fn supports_tools(&self) -> bool {
        true
    }
}

/// Convert agent-loop messages to OpenAI chat completion format.
fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
    messages
        .iter()
        .map(|msg| match msg {
            Message::System { content, .. } => serde_json::json!({
                "role": "system",
                "content": content,
            }),
            Message::User { content, .. } => serde_json::json!({
                "role": "user",
                "content": content,
            }),
            Message::Assistant {
                content,
                tool_calls,
                ..
            } => {
                let mut obj = serde_json::json!({ "role": "assistant" });
                if let Some(c) = content {
                    obj["content"] = serde_json::json!(c);
                }
                if let Some(calls) = tool_calls {
                    obj["tool_calls"] = calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                }
                obj
            }
            Message::Tool {
                tool_call_id,
                content,
                ..
            } => serde_json::json!({
                "role": "tool",
                "tool_call_id": tool_call_id,
                "content": content,
            }),
        })
        .collect()
}

/// Convert agent-loop tool schemas to OpenAI tool format.
fn tools_to_openai(tools: &[ToolSchema]) -> Vec<serde_json::Value> {
    tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.parameters,
                }
            })
        })
        .collect()
}

/// Parse an OpenAI chat completion response into an LlmResponse.
fn parse_openai_response(body: serde_json::Value) -> Result<LlmResponse> {
    let choice = body
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first())
        .ok_or_else(|| AgentError::LlmError("No choices in response".into()))?;

    let finish_reason = choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .map(String::from);

    let msg = choice
        .get("message")
        .ok_or_else(|| AgentError::LlmError("No message in choice".into()))?;

    let content = msg.get("content").and_then(|c| c.as_str()).map(String::from);

    let tool_calls: Option<Vec<ToolCall>> = msg
        .get("tool_calls")
        .and_then(|tc| tc.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|tc| {
                    let id = tc.get("id")?.as_str()?.to_string();
                    let func = tc.get("function")?;
                    let name = func.get("name")?.as_str()?.to_string();
                    let args_str = func.get("arguments")?.as_str()?;
                    let arguments = serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                    Some(ToolCall {
                        id,
                        name,
                        arguments,
                    })
                })
                .collect()
        });

    let message = if let Some(calls) = tool_calls.filter(|c| !c.is_empty()) {
        Message::Assistant {
            content,
            tool_calls: Some(calls),
            timestamp: Some(chrono::Utc::now()),
        }
    } else {
        Message::Assistant {
            content,
            tool_calls: None,
            timestamp: Some(chrono::Utc::now()),
        }
    };

    let usage_obj = body.get("usage");
    let usage = TokenUsage {
        prompt_tokens: usage_obj
            .and_then(|u| u.get("prompt_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        completion_tokens: usage_obj
            .and_then(|u| u.get("completion_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
        total_tokens: usage_obj
            .and_then(|u| u.get("total_tokens"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize,
    };

    Ok(LlmResponse {
        message,
        usage,
        stop_reason: finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_messages_to_openai() {
        let messages = vec![
            Message::system("You are helpful"),
            Message::user("Hello"),
            Message::assistant("Hi there"),
        ];
        let openai = messages_to_openai(&messages);
        assert_eq!(openai.len(), 3);
        assert_eq!(openai[0]["role"], "system");
        assert_eq!(openai[1]["role"], "user");
        assert_eq!(openai[2]["role"], "assistant");
    }

    #[test]
    fn test_tools_to_openai() {
        let tools = vec![ToolSchema {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                }
            }),
            category: None,
        }];
        let openai = tools_to_openai(&tools);
        assert_eq!(openai.len(), 1);
        assert_eq!(openai[0]["type"], "function");
        assert_eq!(openai[0]["function"]["name"], "read_file");
    }

    #[test]
    fn test_parse_openai_response_text() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        });
        let resp = parse_openai_response(body).unwrap();
        assert!(resp.message.is_terminal());
        assert_eq!(resp.usage.prompt_tokens, 10);
        assert_eq!(resp.usage.completion_tokens, 5);
    }

    #[test]
    fn test_parse_openai_response_tool_calls() {
        let body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "read_file",
                            "arguments": "{\"path\":\"/test.txt\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {
                "prompt_tokens": 20,
                "completion_tokens": 10,
                "total_tokens": 30
            }
        });
        let resp = parse_openai_response(body).unwrap();
        assert!(!resp.message.is_terminal());
        match resp.message {
            Message::Assistant { tool_calls, .. } => {
                let calls = tool_calls.unwrap();
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "read_file");
            }
            _ => panic!("Expected assistant message"),
        }
    }
}
