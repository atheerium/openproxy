//! DeepSeek Web executor — `chat.deepseek.com` session-based chat API.
//!
//! Port of OmniRoute `open-sse/executors/deepseek-web.ts` (1167 lines):
//! OpenAI chat body → single prompt string, proof-of-work challenge solving,
//! browser-fingerprint headers (X-Client-Bundle-Id, X-Client-Version), SSE
//! response stream with patch-based fragment updates → OpenAI SSE chunks,
//! `reasoning_content` for thinking models, and search citations.
//!
//! Auth: user's `userToken` (from chat.deepseek.com localStorage) stored in
//! `credentials.api_key`. Exchanged for a short-lived `accessToken` via the
//! `/api/v0/users/current` endpoint, cached for 1 hour.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::http;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE, COOKIE};
use reqwest::Body as ReqwestBody;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::core::proxy::ProxyTarget;
use crate::types::ProviderConnection;

use super::{ClientPool, TransportKind, UpstreamResponse};

const DEEPSEEK_BASE: &str = "https://chat.deepseek.com";
const DEEPSEEK_USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Safari/537.36";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

pub struct DeepSeekWebExecutionRequest {
    pub model: String,
    pub body: Value,
    pub stream: bool,
    pub credentials: ProviderConnection,
    pub proxy: Option<ProxyTarget>,
}

#[derive(Debug)]
pub enum DeepSeekWebExecutorError {
    MissingCredentials(String),
    InvalidCredentials(String),
    Serialize(serde_json::Error),
    Request(reqwest::Error),
    PoWFailed(String),
}

impl From<reqwest::Error> for DeepSeekWebExecutorError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error)
    }
}

impl From<serde_json::Error> for DeepSeekWebExecutorError {
    fn from(error: serde_json::Error) -> Self {
        Self::Serialize(error)
    }
}

impl std::fmt::Display for DeepSeekWebExecutorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingCredentials(p) => write!(f, "Missing credentials for {p}"),
            Self::InvalidCredentials(msg) => write!(f, "Invalid credentials: {msg}"),
            Self::Serialize(e) => write!(f, "Serialization error: {e}"),
            Self::Request(e) => write!(f, "Request error: {e}"),
            Self::PoWFailed(msg) => write!(f, "PoW failed: {msg}"),
        }
    }
}

impl std::error::Error for DeepSeekWebExecutorError {}

pub struct DeepSeekWebExecutorResponse {
    pub response: UpstreamResponse,
    pub url: String,
    pub headers: HeaderMap,
    pub transformed_body: Value,
    pub transport: TransportKind,
}

impl std::fmt::Debug for DeepSeekWebExecutorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeepSeekWebExecutorResponse")
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("transformed_body", &self.transformed_body)
            .field("transport", &self.transport)
            .finish()
    }
}

pub struct DeepSeekWebExecutor {
    pool: Arc<ClientPool>,
}

// ---------------------------------------------------------------------------
// Access token cache (userToken → accessToken, 1h TTL)
// ---------------------------------------------------------------------------

static ACCESS_TOKEN_CACHE: once_cell::sync::Lazy<
    tokio::sync::RwLock<HashMap<String, CachedToken>>,
> = once_cell::sync::Lazy::new(|| tokio::sync::RwLock::new(HashMap::new()));

struct CachedToken {
    token: String,
    expires_at: SystemTime,
}

async fn acquire_access_token(
    pool: &ClientPool,
    user_token: &str,
    proxy: Option<&ProxyTarget>,
) -> Result<String, DeepSeekWebExecutorError> {
    // Check cache
    {
        let cache = ACCESS_TOKEN_CACHE.read().await;
        if let Some(cached) = cache.get(user_token) {
            if cached.expires_at > SystemTime::now() {
                return Ok(cached.token.clone());
            }
        }
    }

    // Exchange userToken for accessToken
    let client = pool.get("deepseek-web", proxy)?;
    let resp = client
        .get(format!("{DEEPSEEK_BASE}/api/v0/users/current"))
        .header(AUTHORIZATION, format!("Bearer {user_token}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", DEEPSEEK_USER_AGENT)
        .send()
        .await?;

    let status = resp.status().as_u16();
    if status == 401 || status == 403 {
        return Err(DeepSeekWebExecutorError::InvalidCredentials(
            "DeepSeek auth failed — userToken may be expired. Re-paste your token from chat.deepseek.com."
                .to_string(),
        ));
    }
    if !(200..300).contains(&status) {
        let body = resp.text().await.unwrap_or_default();
        return Err(DeepSeekWebExecutorError::InvalidCredentials(format!(
            "DeepSeek token exchange failed (HTTP {status}): {body}"
        )));
    }

    let json: Value = resp.json().await?;
    let access_token = json
        .pointer("/data/access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            DeepSeekWebExecutorError::InvalidCredentials(
                "No access_token in DeepSeek response".to_string(),
            )
        })?
        .to_string();

    // Cache for 55 minutes (accessToken lives 1h)
    {
        let mut cache = ACCESS_TOKEN_CACHE.write().await;
        cache.insert(
            user_token.to_string(),
            CachedToken {
                token: access_token.clone(),
                expires_at: SystemTime::now() + Duration::from_secs(55 * 60),
            },
        );
    }

    Ok(access_token)
}

// ---------------------------------------------------------------------------
// Proof of Work (DeepSeekHashV1)
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct PowChallenge {
    #[serde(rename = "algorithm")]
    algorithm: String,
    #[serde(rename = "challenge")]
    challenge: String,
    #[serde(rename = "salt")]
    salt: String,
    #[serde(rename = "difficulty")]
    difficulty: u32,
}

fn solve_pow(
    challenge: &str,
    salt: &str,
    difficulty: u32,
) -> Result<String, DeepSeekWebExecutorError> {
    let prefix_zeros = difficulty as usize;
    for nonce in 0u64.. {
        let input = format!("{challenge}{salt}{nonce}");
        let hash = Sha256::digest(input.as_bytes());
        let hex_str = hex::encode(hash);

        // Check leading zeros
        if hex_str.chars().take(prefix_zeros).all(|c| c == '0') {
            return Ok(format!("{challenge}{salt}{nonce}"));
        }

        // Safety limit
        if nonce > 50_000_000 {
            return Err(DeepSeekWebExecutorError::PoWFailed(format!(
                "Could not solve PoW after {nonce} iterations (difficulty={difficulty})"
            )));
        }
    }
    unreachable!()
}

async fn get_pow_response(
    pool: &ClientPool,
    access_token: &str,
    proxy: Option<&ProxyTarget>,
) -> Result<String, DeepSeekWebExecutorError> {
    let client = pool.get("deepseek-web", proxy)?;
    let resp = client
        .post(format!("{DEEPSEEK_BASE}/api/v0/chat/create_pow_challenge"))
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", DEEPSEEK_USER_AGENT)
        .send()
        .await?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(DeepSeekWebExecutorError::PoWFailed(format!(
            "PoW challenge request failed: HTTP {status}"
        )));
    }

    let json: Value = resp.json().await?;
    let challenge: PowChallenge =
        serde_json::from_value(json.pointer("/data").cloned().unwrap_or(Value::Null))
            .or_else(|_| serde_json::from_value(json.clone()))
            .map_err(|e| {
                DeepSeekWebExecutorError::PoWFailed(format!("Invalid PoW challenge: {e}"))
            })?;

    solve_pow(&challenge.challenge, &challenge.salt, challenge.difficulty)
}

// ---------------------------------------------------------------------------
// Session management
// ---------------------------------------------------------------------------

async fn create_session(
    pool: &ClientPool,
    access_token: &str,
    proxy: Option<&ProxyTarget>,
) -> Result<String, DeepSeekWebExecutorError> {
    let client = pool.get("deepseek-web", proxy)?;
    let resp = client
        .post(format!("{DEEPSEEK_BASE}/api/v0/chat_session/create"))
        .header(AUTHORIZATION, format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .header("User-Agent", DEEPSEEK_USER_AGENT)
        .send()
        .await?;

    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(DeepSeekWebExecutorError::InvalidCredentials(format!(
            "DeepSeek session creation failed: HTTP {status}"
        )));
    }

    let json: Value = resp.json().await?;
    let session_id = json
        .pointer("/data/id")
        .and_then(Value::as_str)
        .or_else(|| json.pointer("/id").and_then(Value::as_str))
        .ok_or_else(|| {
            DeepSeekWebExecutorError::InvalidCredentials(
                "No session ID in DeepSeek response".to_string(),
            )
        })?
        .to_string();

    Ok(session_id)
}

async fn delete_session(
    pool: &ClientPool,
    access_token: &str,
    session_id: &str,
    proxy: Option<&ProxyTarget>,
) {
    if let Ok(client) = pool.get("deepseek-web", proxy) {
        let _ = client
            .delete(format!("{DEEPSEEK_BASE}/api/v0/chat_session/{session_id}"))
            .header(AUTHORIZATION, format!("Bearer {access_token}"))
            .header("Content-Type", "application/json")
            .header("User-Agent", DEEPSEEK_USER_AGENT)
            .send()
            .await;
    }
}

// ---------------------------------------------------------------------------
// Message formatting
// ---------------------------------------------------------------------------

fn flatten_messages(messages: &[Value]) -> String {
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
            conversation_parts.push(format!("<|{role}|>\n{content}\n"));
        }
    }

    let mut prompt = system_parts.join("\n\n");
    if !conversation_parts.is_empty() {
        if !prompt.is_empty() {
            prompt.push_str("\n\n");
        }
        // DeepSeek uses a rolling window of 20 messages for multi-turn
        let window: Vec<_> = conversation_parts
            .into_iter()
            .rev()
            .take(20)
            .rev()
            .collect();
        prompt.push_str(&window.join("\n"));
    }
    prompt
}

fn is_thinking_model(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("r1") || m.contains("think") || m.contains("reason")
}

fn is_expert_model(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("pro") || m.contains("expert")
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

/// Parse a DeepSeek SSE `data:` line (which is actually a JSON object with patch ops).
///
/// DeepSeek returns events like:
/// ```json
/// {"p":"response/fragments","o":"M","v":{"fragments":[{"type":"ANSWER","content":"Hello"}]}}
/// ```
/// where `p` is the patch path, `o` is the operation, and `v` is the value.
enum DeepSeekEvent {
    TextDelta(String),
    ThinkDelta(String),
    Done,
    SearchResults(Value),
    Error(String),
    Skip,
}

fn parse_deepseek_event(line: &str) -> DeepSeekEvent {
    let Ok(val) = serde_json::from_str::<Value>(line) else {
        return DeepSeekEvent::Skip;
    };

    // Check for error
    if let Some(err) = val.get("error").or_else(|| val.get("message")) {
        let msg = err
            .get("content")
            .or_else(|| err.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Unknown DeepSeek error");
        return DeepSeekEvent::Error(msg.to_string());
    }

    let path = val.get("p").and_then(Value::as_str).unwrap_or("");
    let value = val.get("v");

    match path {
        "response/fragments" => {
            if let Some(v) = value {
                if let Some(fragments) = v.get("fragments").and_then(Value::as_array) {
                    let mut text_delta = String::new();
                    let mut think_delta = String::new();
                    for frag in fragments {
                        let content = frag.get("content").and_then(Value::as_str).unwrap_or("");
                        let frag_type =
                            frag.get("type").and_then(Value::as_str).unwrap_or("ANSWER");
                        match frag_type {
                            "THINK" | "THINKING" => think_delta.push_str(content),
                            _ => text_delta.push_str(content),
                        }
                    }
                    if !text_delta.is_empty() && !think_delta.is_empty() {
                        // Both present — return think first, text will come next
                        return DeepSeekEvent::ThinkDelta(think_delta);
                    }
                    if !think_delta.is_empty() {
                        return DeepSeekEvent::ThinkDelta(think_delta);
                    }
                    if !text_delta.is_empty() {
                        return DeepSeekEvent::TextDelta(text_delta);
                    }
                }
            }
            DeepSeekEvent::Skip
        }
        "response/search_results" => {
            if let Some(v) = value {
                DeepSeekEvent::SearchResults(v.clone())
            } else {
                DeepSeekEvent::Skip
            }
        }
        "response/status" => {
            let status = value.and_then(Value::as_str).unwrap_or("");
            if status == "FINISHED" || status == "DONE" {
                DeepSeekEvent::Done
            } else {
                DeepSeekEvent::Skip
            }
        }
        _ => DeepSeekEvent::Skip,
    }
}

async fn convert_deepseek_response(
    response: reqwest::Response,
    model: &str,
    cid: &str,
    created: i64,
    is_thinking: bool,
    stream: bool,
) -> Result<UpstreamResponse, DeepSeekWebExecutorError> {
    use futures_util::StreamExt;

    let mut content = String::new();
    let mut reasoning = String::new();
    let mut search_citations: Vec<Value> = Vec::new();

    let mut byte_stream = response.bytes_stream();
    let mut line_buf = String::new();

    while let Some(chunk_result) = byte_stream.next().await {
        let bytes = chunk_result.map_err(DeepSeekWebExecutorError::Request)?;
        line_buf.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(newline_pos) = line_buf.find('\n') {
            let line = line_buf[..newline_pos].trim().to_string();
            line_buf = line_buf[newline_pos + 1..].to_string();

            if line.is_empty() || !line.starts_with("data:") {
                continue;
            }
            let data = line[5..].trim();
            if data == "[DONE]" {
                break;
            }

            match parse_deepseek_event(data) {
                DeepSeekEvent::TextDelta(delta) => content.push_str(&delta),
                DeepSeekEvent::ThinkDelta(delta) => reasoning.push_str(&delta),
                DeepSeekEvent::SearchResults(results) => {
                    if let Some(arr) = results.as_array() {
                        search_citations.extend(arr.iter().cloned());
                    }
                }
                DeepSeekEvent::Done => break,
                DeepSeekEvent::Error(msg) => {
                    return Err(DeepSeekWebExecutorError::InvalidCredentials(msg));
                }
                DeepSeekEvent::Skip => {}
            }
        }
    }

    // Strip thinking tokens if present (DeepSeek sometimes wraps thinking in `<` tags)
    let reasoning_clean = reasoning
        .lines()
        .filter(|l| !l.starts_with('<'))
        .collect::<Vec<_>>()
        .join("\n");

    if stream {
        // Build SSE stream
        let mut chunks = Vec::new();

        if is_thinking && !reasoning_clean.is_empty() {
            // Emit thinking deltas
            for line in reasoning_clean.lines() {
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

        // Append search citations footer
        if !search_citations.is_empty() {
            let mut footer = String::from("\n\n---\n**Sources:**\n");
            for (i, citation) in search_citations.iter().enumerate() {
                if let Some(title) = citation.get("title").and_then(Value::as_str) {
                    let url = citation.get("url").and_then(Value::as_str).unwrap_or("");
                    footer.push_str(&format!("{}. [{}]({})\n", i + 1, title, url));
                }
            }
            chunks.push(sse_chunk(
                cid,
                created,
                model,
                json!({"content": footer}),
                None,
            ));
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
        if is_thinking && !reasoning_clean.is_empty() {
            message["reasoning_content"] = Value::String(reasoning_clean);
        }

        let mut body = json!({
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

        // Attach search citations if present
        if !search_citations.is_empty() {
            body["citations"] = Value::Array(search_citations);
        }

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

impl DeepSeekWebExecutor {
    pub fn new(pool: Arc<ClientPool>) -> Self {
        Self { pool }
    }

    pub async fn execute_request(
        &self,
        request: DeepSeekWebExecutionRequest,
    ) -> Result<DeepSeekWebExecutorResponse, DeepSeekWebExecutorError> {
        let url = format!("{DEEPSEEK_BASE}/api/v0/chat/completion");

        // Get credentials
        let user_token = request.credentials.api_key.as_deref().ok_or_else(|| {
            DeepSeekWebExecutorError::MissingCredentials(
                "deepseek-web requires a userToken in api_key".to_string(),
            )
        })?;

        // Step 1: Exchange userToken for accessToken
        let access_token =
            acquire_access_token(&self.pool, user_token, request.proxy.as_ref()).await?;

        // Step 2: Get proof-of-work solution
        let pow_response =
            get_pow_response(&self.pool, &access_token, request.proxy.as_ref()).await?;

        // Step 3: Create session
        let session_id = create_session(&self.pool, &access_token, request.proxy.as_ref()).await?;

        // Build prompt from messages
        let messages = request.body.get("messages").and_then(Value::as_array);
        let prompt = match messages {
            Some(msgs) => flatten_messages(msgs),
            None => String::new(),
        };

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
                .unwrap_or(false)
            || is_thinking_model(&request.model);

        let expert = is_expert_model(&request.model);

        let search = request
            .body
            .get("search")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        // Build request body
        let payload = json!({
            "chat_session_id": session_id,
            "parent_message_id": null,
            "model_type": if expert { "expert" } else { "default" },
            "prompt": prompt,
            "ref_file_ids": [],
            "thinking_enabled": thinking,
            "search_enabled": search,
            "preempt": false,
        });

        // Build fingerprint headers
        let headers = self.build_headers(&access_token, &pow_response)?;

        let body_bytes = serde_json::to_vec(&payload)?;

        let client = self.pool.get("deepseek-web", request.proxy.as_ref())?;
        let response = client
            .post(&url)
            .headers(headers.clone())
            .body(body_bytes)
            .send()
            .await?;

        let status = response.status().as_u16();
        if !(200..300).contains(&status) {
            let err_body = response.text().await.unwrap_or_default();
            let (msg, code): (&str, String) = match status {
                401 | 403 => (
                    "DeepSeek auth failed — token may be expired. Re-paste your token from chat.deepseek.com.",
                    format!("HTTP_{status}"),
                ),
                429 => (
                    "DeepSeek rate limited. Wait a moment and retry.",
                    format!("HTTP_{status}"),
                ),
                _ => (
                    &format!("DeepSeek returned HTTP {status}: {err_body}")[..],
                    format!("HTTP_{status}"),
                ),
            };

            // Cleanup session on error
            delete_session(
                &self.pool,
                &access_token,
                &session_id,
                request.proxy.as_ref(),
            )
            .await;

            return Ok(DeepSeekWebExecutorResponse {
                response: json_error(status, msg, "upstream_error", Some(&code)),
                url,
                headers,
                transformed_body: payload,
                transport: TransportKind::Reqwest,
            });
        }

        // Convert response
        let cid = format!("chatcmpl-ds-{}", &Uuid::new_v4().simple().to_string()[..12]);
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let converted = convert_deepseek_response(
            response,
            &request.model,
            &cid,
            created,
            thinking,
            request.stream,
        )
        .await?;

        // Cleanup session
        delete_session(
            &self.pool,
            &access_token,
            &session_id,
            request.proxy.as_ref(),
        )
        .await;

        Ok(DeepSeekWebExecutorResponse {
            response: converted,
            url,
            headers,
            transformed_body: payload,
            transport: TransportKind::Reqwest,
        })
    }

    fn build_headers(
        &self,
        access_token: &str,
        pow_response: &str,
    ) -> Result<HeaderMap, DeepSeekWebExecutorError> {
        let mut headers = HeaderMap::new();

        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {access_token}")).map_err(|_| {
                DeepSeekWebExecutorError::InvalidCredentials("Invalid auth header".to_string())
            })?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        headers.insert(
            reqwest::header::ACCEPT_ENCODING,
            HeaderValue::from_static("gzip, deflate, br, zstd"),
        );

        // Browser fingerprint headers
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_static(DEEPSEEK_USER_AGENT),
        );
        headers.insert(
            "X-Client-Bundle-Id",
            HeaderValue::from_static("com.deepseek.chat"),
        );
        headers.insert("X-Client-Version", HeaderValue::from_static("2.0.0"));
        headers.insert("X-Client-Platform", HeaderValue::from_static("web"));
        headers.insert(
            reqwest::header::ORIGIN,
            HeaderValue::from_static(DEEPSEEK_BASE),
        );
        headers.insert(
            reqwest::header::REFERER,
            HeaderValue::from_static("https://chat.deepseek.com/"),
        );

        // Fake cookie (required by DeepSeek)
        headers.insert(COOKIE, HeaderValue::from_static("_evil☛"));

        // PoW response
        headers.insert(
            "X-Ds-Pow-Response",
            HeaderValue::from_str(pow_response).map_err(|_| {
                DeepSeekWebExecutorError::InvalidCredentials("Invalid PoW response".to_string())
            })?,
        );

        Ok(headers)
    }
}
