//! Kimi Web executor — `www.kimi.ai` Connect-RPC chat API.
//!
//! Port of OmniRoute `open-sse/executors/kimi-web.ts` (623 lines):
//! OpenAI chat body → single prompt string, Connect-RPC wire protocol
//! (5-byte framed JSON), browser-fingerprint headers, response stream with
//! `op`/`mask`/`block` pattern → OpenAI SSE chunks, `reasoning_content` for
//! thinking models.
//!
//! Auth: user's `access_token` (from kimi.ai localStorage) stored in
//! `credentials.api_key`. Sent as `Authorization: Bearer <token>`.
//!
//! No tool support — errors if tools are passed in the request body.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::http;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Body as ReqwestBody;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::core::proxy::ProxyTarget;
use crate::types::ProviderConnection;

use super::{ClientPool, TransportKind, UpstreamResponse};

const KIMI_BASE: &str = "https://www.kimi.ai";
const KIMI_CHAT_PATH: &str = "/apiv2/kimi.gateway.chat.v1.ChatService/Chat";
const KIMI_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// Kimi model config registry
// ---------------------------------------------------------------------------

struct KimiModelConfig {
    scenario: &'static str,
    kimi_plus_id: Option<&'static str>,
    context_length: u32,
    reasoning_effort: Option<&'static str>,
}

fn kimi_model_registry() -> &'static [(&'static str, KimiModelConfig)] {
    &[
        (
            "kimi",
            KimiModelConfig {
                scenario: "kimi",
                kimi_plus_id: None,
                context_length: 128_000,
                reasoning_effort: None,
            },
        ),
        (
            "kimi-k2",
            KimiModelConfig {
                scenario: "kimi",
                kimi_plus_id: None,
                context_length: 128_000,
                reasoning_effort: Some("high"),
            },
        ),
        (
            "moonshot-v1-8k",
            KimiModelConfig {
                scenario: "kimi",
                kimi_plus_id: None,
                context_length: 8_000,
                reasoning_effort: None,
            },
        ),
        (
            "moonshot-v1-32k",
            KimiModelConfig {
                scenario: "kimi",
                kimi_plus_id: None,
                context_length: 32_000,
                reasoning_effort: None,
            },
        ),
        (
            "moonshot-v1-128k",
            KimiModelConfig {
                scenario: "kimi",
                kimi_plus_id: None,
                context_length: 128_000,
                reasoning_effort: None,
            },
        ),
    ]
}

fn resolve_kimi_model(model: &str) -> Option<&'static KimiModelConfig> {
    kimi_model_registry()
        .iter()
        .find(|(id, _)| *id == model)
        .map(|(_, cfg)| cfg)
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub struct KimiWebExecutionRequest {
    pub model: String,
    pub body: Value,
    pub stream: bool,
    pub credentials: ProviderConnection,
    pub proxy: Option<ProxyTarget>,
}

#[derive(Debug)]
pub enum KimiWebExecutorError {
    MissingCredentials(String),
    InvalidCredentials(String),
    Serialize(serde_json::Error),
    Request(reqwest::Error),
    UnsupportedModel(String),
    ConnectProtocol(String),
}

impl From<reqwest::Error> for KimiWebExecutorError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error)
    }
}

impl From<serde_json::Error> for KimiWebExecutorError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

impl std::fmt::Display for KimiWebExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials(p) => write!(f, "Missing credentials for {p}"),
            Self::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            Self::Serialize(e) => write!(f, "Serialization error: {e}"),
            Self::Request(e) => write!(f, "Request error: {e}"),
            Self::UnsupportedModel(m) => write!(f, "Unsupported Kimi model: {m}"),
            Self::ConnectProtocol(msg) => write!(f, "Connect protocol error: {msg}"),
        }
    }
}

impl std::error::Error for KimiWebExecutorError {}

pub struct KimiWebExecutorResponse {
    pub response: UpstreamResponse,
    pub url: String,
    pub headers: HeaderMap,
    pub transformed_body: Value,
    pub transport: TransportKind,
}

impl std::fmt::Debug for KimiWebExecutorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KimiWebExecutorResponse")
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("transformed_body", &self.transformed_body)
            .field("transport", &self.transport)
            .finish()
    }
}

pub struct KimiWebExecutor {
    pool: Arc<ClientPool>,
}

// ---------------------------------------------------------------------------
// Connect-RPC framing
// ---------------------------------------------------------------------------

/// Encode a JSON message with Connect-RPC framing.
///
/// Connect-RPC wire format: 5-byte header (1 byte flags + 4 bytes big-endian length)
/// followed by the JSON body.
///
/// Flags:
/// - 0x00 = normal message
/// - 0x02 = end of stream
fn encode_connect_frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u32;
    let mut frame = Vec::with_capacity(5 + payload.len());
    frame.push(0x00); // flags: normal message
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

/// Decode a Connect-RPC frame from a byte buffer.
///
/// Returns (flags, payload, bytes_consumed) or None if not enough data.
fn decode_connect_frame(buf: &[u8]) -> Option<(u8, &[u8], usize)> {
    if buf.len() < 5 {
        return None;
    }
    let flags = buf[0];
    let len = u32::from_be_bytes([buf[1], buf[2], buf[3], buf[4]]) as usize;
    if buf.len() < 5 + len {
        return None;
    }
    Some((flags, &buf[5..5 + len], 5 + len))
}

// ---------------------------------------------------------------------------
// Message formatting
// ---------------------------------------------------------------------------

fn fold_messages(messages: &[Value]) -> (String, String) {
    let mut system_parts = Vec::new();
    let mut conversation_parts = Vec::new();

    for msg in messages {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("user");
        let content = match msg.get("content") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Array(parts)) => parts
                .iter()
                .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|p| p.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            _ => continue,
        };
        if content.trim().is_empty() {
            continue;
        }
        if role == "system" || role == "developer" {
            system_parts.push(content);
        } else {
            conversation_parts.push(format!("<|{role}|>\n{content}"));
        }
    }

    let system_prompt = system_parts.join("\n\n");
    let prompt = conversation_parts.join("\n\n");

    (system_prompt, prompt)
}

// ---------------------------------------------------------------------------
// Response conversion
// ---------------------------------------------------------------------------

/// OpenAI-compatible SSE chunk frame
fn sse_chunk(
    cid: &str,
    created: i64,
    model: &str,
    delta: Value,
    finish_reason: Option<&str>,
) -> String {
    format!(
        "data: {}\n\n",
        serde_json::to_string(&json!({
            "id": cid,
            "object": "chat.completion.chunk",
            "created": created,
            "model": model,
            "system_fingerprint": Value::Null,
            "choices": [{
                "index": 0,
                "delta": delta,
                "finish_reason": finish_reason
                    .map(|s| Value::String(s.to_string()))
                    .unwrap_or(Value::Null),
                "logprobs": Value::Null,
            }],
        }))
        .unwrap_or_default()
    )
}

/// Parse a Connect-RPC JSON message to extract text/think deltas.
///
/// Kimi messages have the shape:
/// ```json
/// {"op":"append","path":"block.text.content","value":"Hello"}
/// ```
/// or
/// ```json
/// {"op":"append","path":"block.think.content","reasoning":"..."}
/// ```
enum KimiDelta {
    Text(String),
    Think(String),
    Done,
    Error(String),
    Skip,
}

fn extract_kimi_delta(msg: &Value) -> KimiDelta {
    let op = msg.get("op").and_then(Value::as_str).unwrap_or("");
    let path = msg.get("path").and_then(Value::as_str).unwrap_or("");

    match op {
        "append" => {
            if path.contains("block.text.content") {
                if let Some(value) = msg.get("value").and_then(Value::as_str) {
                    return KimiDelta::Text(value.to_string());
                }
            }
            if path.contains("block.think.content") {
                if let Some(reasoning) = msg.get("reasoning").and_then(Value::as_str) {
                    return KimiDelta::Think(reasoning.to_string());
                }
                if let Some(value) = msg.get("value").and_then(Value::as_str) {
                    return KimiDelta::Think(value.to_string());
                }
            }
            KimiDelta::Skip
        }
        "set" => {
            // Initial content set — often has the full content so far
            if path.contains("block.text.content") || path.contains("block.think") {
                KimiDelta::Skip // We'll get deltas from append
            } else {
                KimiDelta::Skip
            }
        }
        _ => KimiDelta::Skip,
    }
}

async fn convert_kimi_response(
    response: reqwest::Response,
    model: &str,
    cid: &str,
    created: i64,
    is_thinking: bool,
    stream: bool,
) -> Result<UpstreamResponse, KimiWebExecutorError> {
    use futures_util::StreamExt;

    let mut content = String::new();
    let mut reasoning = String::new();

    let mut byte_stream = response.bytes_stream();
    let mut frame_buf: Vec<u8> = Vec::new();

    while let Some(chunk_result) = byte_stream.next().await {
        let bytes = chunk_result.map_err(KimiWebExecutorError::Request)?;
        frame_buf.extend_from_slice(&bytes);

        // Process complete frames
        while let Some((flags, payload, consumed)) = decode_connect_frame(&frame_buf) {
            let payload = payload.to_vec(); // Copy before draining
            frame_buf.drain(..consumed);

            // End-of-stream flag
            if flags & 0x02 != 0 {
                // Check for error in the end frame
                if let Ok(msg) = serde_json::from_slice::<Value>(&payload) {
                    if let Some(err) = msg.get("error") {
                        let err_msg = err
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("Kimi stream ended with error");
                        return Err(KimiWebExecutorError::ConnectProtocol(err_msg.to_string()));
                    }
                }
                break;
            }

            // Parse the JSON payload
            if let Ok(msg) = serde_json::from_slice::<Value>(&payload) {
                match extract_kimi_delta(&msg) {
                    KimiDelta::Text(delta) => content.push_str(&delta),
                    KimiDelta::Think(delta) => reasoning.push_str(&delta),
                    KimiDelta::Done => break,
                    KimiDelta::Error(msg) => {
                        return Err(KimiWebExecutorError::ConnectProtocol(msg));
                    }
                    KimiDelta::Skip => {}
                }
            }
        }
    }

    if stream {
        let mut chunks = Vec::new();

        // Emit thinking deltas
        if is_thinking && !reasoning.is_empty() {
            for line in reasoning.lines() {
                if !line.is_empty() {
                    chunks.push(sse_chunk(
                        cid,
                        created,
                        model,
                        json!({"reasoning_content": line}),
                        None,
                    ));
                }
            }
        }

        // Emit content deltas
        if !content.is_empty() {
            for line in content.lines() {
                if !line.is_empty() {
                    chunks.push(sse_chunk(
                        cid,
                        created,
                        model,
                        json!({"content": line}),
                        None,
                    ));
                }
            }
        }

        // Final done chunk
        chunks.push(sse_chunk(cid, created, model, json!({}), Some("stop")));
        chunks.push("data: [DONE]\n\n".to_string());

        let body = chunks.join("");
        let bytes = body.into_bytes();
        let mut http_resp = http::Response::new(ReqwestBody::from(bytes));
        *http_resp.status_mut() = reqwest::StatusCode::OK;
        http_resp.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        Ok(UpstreamResponse::Reqwest(reqwest::Response::from(
            http_resp,
        )))
    } else {
        // Non-streaming: build chat.completion JSON
        let mut message = json!({"role": "assistant", "content": content});
        if is_thinking && !reasoning.is_empty() {
            message["reasoning_content"] = Value::String(reasoning);
        }

        let body = json!({
            "id": cid,
            "object": "chat.completion",
            "created": created,
            "model": model,
            "system_fingerprint": Value::Null,
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": "stop",
                "logprobs": Value::Null,
            }],
            "usage": {
                "prompt_tokens": 0,
                "completion_tokens": 0,
                "total_tokens": 0,
            },
        });

        let bytes = serde_json::to_vec(&body).unwrap_or_default();
        let mut http_resp = http::Response::new(ReqwestBody::from(bytes));
        *http_resp.status_mut() = reqwest::StatusCode::OK;
        http_resp.headers_mut().insert(
            reqwest::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        Ok(UpstreamResponse::Reqwest(reqwest::Response::from(
            http_resp,
        )))
    }
}

/// JSON error response builder
fn json_error(status: u16, message: &str, err_type: &str, code: Option<&str>) -> UpstreamResponse {
    let mut body = json!({ "error": { "message": message, "type": err_type } });
    if let Some(code) = code {
        body["error"]["code"] = Value::String(code.to_string());
    }
    let bytes = serde_json::to_vec(&body).unwrap_or_default();
    let mut http_resp = http::Response::new(ReqwestBody::from(bytes));
    *http_resp.status_mut() =
        reqwest::StatusCode::from_u16(status).unwrap_or(reqwest::StatusCode::BAD_GATEWAY);
    http_resp.headers_mut().insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    UpstreamResponse::Reqwest(reqwest::Response::from(http_resp))
}

// ---------------------------------------------------------------------------
// Executor
// ---------------------------------------------------------------------------

impl KimiWebExecutor {
    pub fn new(pool: Arc<ClientPool>) -> Self {
        Self { pool }
    }

    pub async fn execute_request(
        &self,
        request: KimiWebExecutionRequest,
    ) -> Result<KimiWebExecutorResponse, KimiWebExecutorError> {
        let url = format!("{KIMI_BASE}{KIMI_CHAT_PATH}");

        // Validate model
        let model_config = resolve_kimi_model(&request.model).ok_or_else(|| {
            KimiWebExecutorError::UnsupportedModel(format!(
                "Model '{}' is not in the Kimi web registry. Supported: {}",
                request.model,
                kimi_model_registry()
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        })?;

        // Get credentials
        let access_token = request
            .credentials
            .api_key
            .as_deref()
            .or_else(|| request.credentials.access_token.as_deref())
            .ok_or_else(|| {
                KimiWebExecutorError::MissingCredentials(
                    "kimi-web requires an access_token in api_key or access_token".to_string(),
                )
            })?;

        // Fold messages into system prompt + user prompt
        let messages = request.body.get("messages").and_then(Value::as_array);
        let (system_prompt, prompt) = match messages {
            Some(msgs) => fold_messages(msgs),
            None => (String::new(), String::new()),
        };

        // Check for tools (not supported)
        if let Some(tools) = request.body.get("tools") {
            if tools.as_array().map_or(false, |a| !a.is_empty()) {
                return Err(KimiWebExecutorError::UnsupportedModel(
                    "Kimi web does not support tool calling. Remove tools from the request."
                        .to_string(),
                ));
            }
        }

        // Build the Connect-RPC request body
        let thinking = request
            .body
            .get("thinking")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || request
                .body
                .get("reasoning_effort")
                .and_then(Value::as_str)
                .map(|e| e != "none")
                .unwrap_or(false);

        let mut options = json!({
            "thinking": thinking,
            "enable_plugin": false,
        });
        if !system_prompt.is_empty() {
            options["system_prompt"] = Value::String(system_prompt);
        }
        if let Some(effort) = model_config.reasoning_effort {
            options["reasoning_effort"] = Value::String(effort.to_string());
        }
        options["context_length"] = Value::Number(model_config.context_length.into());

        let mut request_body = json!({
            "chat_id": "",
            "scenario": model_config.scenario,
            "tools": [],
            "message": {
                "id": "",
                "parent_id": "",
                "role": "user",
                "blocks": [{
                    "text": {
                        "content": prompt
                    }
                }],
                "scenario": model_config.scenario,
            },
            "options": options,
        });

        if let Some(kpi_id) = model_config.kimi_plus_id {
            request_body["kimiplus_id"] = Value::String(kpi_id.to_string());
        }

        // Build headers
        let headers = self.build_headers(access_token)?;

        // Encode with Connect-RPC framing
        let json_bytes = serde_json::to_vec(&request_body)?;
        let framed = encode_connect_frame(&json_bytes);

        let client = self.pool.get("kimi-web", request.proxy.as_ref())?;
        let response = client
            .post(&url)
            .headers(headers.clone())
            .body(framed)
            .send()
            .await?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let err_body = response.text().await.unwrap_or_default();
            let (msg, code): (&str, String) = match status {
                401 | 403 => (
                    "Kimi auth failed — access_token may be expired. Re-paste your token from kimi.ai.",
                    format!("HTTP_{status}"),
                ),
                429 => (
                    "Kimi rate limited. Wait a moment and retry.",
                    format!("HTTP_{status}"),
                ),
                _ => (
                    &format!("Kimi returned HTTP {status}: {err_body}")[..],
                    format!("HTTP_{status}"),
                ),
            };
            return Ok(KimiWebExecutorResponse {
                response: json_error(status, msg, "upstream_error", Some(&code)),
                url,
                headers,
                transformed_body: request_body,
                transport: TransportKind::Reqwest,
            });
        }

        // Convert response
        let cid = format!(
            "chatcmpl-kimi-{}",
            &Uuid::new_v4().simple().to_string()[..12]
        );
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let converted = convert_kimi_response(
            response,
            &request.model,
            &cid,
            created,
            thinking,
            request.stream,
        )
        .await?;

        Ok(KimiWebExecutorResponse {
            response: converted,
            url,
            headers,
            transformed_body: request_body,
            transport: TransportKind::Reqwest,
        })
    }

    fn build_headers(&self, access_token: &str) -> Result<HeaderMap, KimiWebExecutorError> {
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}")).map_err(|_| {
                KimiWebExecutorError::InvalidCredentials("Invalid auth header".to_string())
            })?,
        );
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/connect+json"),
        );
        headers.insert(ACCEPT, HeaderValue::from_static("*/*"));
        headers.insert(
            reqwest::header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br, zstd"),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(KIMI_USER_AGENT),
        );
        headers.insert(reqwest::header::ORIGIN, HeaderValue::from_static(KIMI_BASE));
        headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static("https://www.kimi.ai/"),
        );
        headers.insert("connect-protocol-version", HeaderValue::from_static("1"));

        Ok(headers)
    }
}
