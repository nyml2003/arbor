// DeepSeek API client — OpenAI-compatible chat completions with SSE streaming.
//
// Uses ureq for sync HTTP. Streaming responses are read on a background thread
// and tokens are delivered via mpsc::Sender to the main thread.

use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::thread;

use serde::{Deserialize, Serialize};

// ── Request types ──────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// ── Response types ─────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Option<Delta>,
    // `message` is for non-streaming responses; present in the JSON but unused
    // in streaming mode. Serde ignores unknown fields by default, but we list
    // it here so the struct stays correct if we add non-streaming support later.
    #[allow(dead_code)]
    message: Option<Delta>,
}

#[derive(Deserialize, Debug)]
struct Delta {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: ApiError,
}

#[derive(Deserialize, Debug)]
struct ApiError {
    message: String,
}

// ── Token event ────────────────────────────────────────────────────

/// A streaming event from the LLM.
#[derive(Clone, Debug)]
pub enum StreamEvent {
    /// A new token of content.
    Token(String),
    /// The stream completed successfully.
    Done,
    /// An error occurred.
    Error(String),
}

// ── Client ─────────────────────────────────────────────────────────

pub struct DeepSeekClient {
    api_key: String,
    base_url: String,
    model: String,
}

impl DeepSeekClient {
    /// Create a new DeepSeek client.
    ///
    /// Reads `DEEPSEEK_API_KEY` from the environment.
    /// Falls back to `OPENAI_API_KEY` if `DEEPSEEK_API_KEY` is not set.
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| anyhow::anyhow!(
                "DEEPSEEK_API_KEY (or OPENAI_API_KEY) environment variable not set"
            ))?;

        let base_url = std::env::var("DEEPSEEK_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_string());

        let model = std::env::var("DEEPSEEK_MODEL")
            .unwrap_or_else(|_| "deepseek-chat".to_string());

        Ok(Self { api_key, base_url, model })
    }

    /// Create a client with explicit configuration.
    #[allow(dead_code)]
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.deepseek.com".to_string(),
            model,
        }
    }

    /// Send a chat completion request with SSE streaming.
    ///
    /// Spawns a background thread that reads the SSE stream and sends
    /// `StreamEvent` tokens through the returned `mpsc::Receiver`.
    /// The main thread polls the receiver in its event loop.
    pub fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> anyhow::Result<mpsc::Receiver<StreamEvent>> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages,
            stream: true,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };

        let url = format!("{}/v1/chat/completions", self.base_url);
        let api_key = self.api_key.clone();

        let (tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("aster-api".into())
            .spawn(move || {
                let result = stream_sse(&url, &api_key, &request, &tx);
                match result {
                    Ok(()) => {
                        let _ = tx.send(StreamEvent::Done);
                    }
                    Err(e) => {
                        let _ = tx.send(StreamEvent::Error(e.to_string()));
                    }
                }
                // tx is dropped here — rx will return None on subsequent recv
            })?;

        Ok(rx)
    }
}

impl Default for DeepSeekClient {
    fn default() -> Self {
        Self::from_env().expect("DeepSeekClient: DEEPSEEK_API_KEY not set")
    }
}

// ── SSE streaming helper ───────────────────────────────────────────

fn stream_sse(
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    tx: &mpsc::Sender<StreamEvent>,
) -> anyhow::Result<()> {
    let response = ureq::post(url)
        .header("Authorization", &format!("Bearer {}", api_key))
        .header("Accept", "text/event-stream")
        .send_json(&request)?;

    let reader: Box<dyn Read + Send> = Box::new(response.into_body().into_reader());
    let buf = BufReader::new(reader);

    for line in buf.lines() {
        let line = line?;

        // SSE lines that start with "data: "
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                break;
            }

            match serde_json::from_str::<ChatResponse>(data) {
                Ok(resp) => {
                    for choice in &resp.choices {
                        if let Some(ref delta) = choice.delta {
                            if let Some(ref content) = delta.content {
                                if tx.send(StreamEvent::Token(content.clone())).is_err() {
                                    // Receiver dropped — main thread exited
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Could be an error response
                    if let Ok(err) = serde_json::from_str::<ErrorResponse>(data) {
                        let msg = format!("API error: {}", err.error.message);
                        let _ = tx.send(StreamEvent::Error(msg));
                        return Ok(());
                    }
                    // Otherwise skip unparseable lines
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_serializes_correctly() {
        let req = ChatRequest {
            model: "deepseek-chat".into(),
            messages: vec![Message {
                role: "user".into(),
                content: "hello".into(),
            }],
            stream: true,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"role\":\"user\""));
    }

    #[test]
    fn sse_data_line_parsing() {
        let data = r#"{"choices":[{"delta":{"content":"Hello"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(data).unwrap();
        let content = resp.choices[0]
            .delta.as_ref()
            .and_then(|d| d.content.as_ref())
            .unwrap();
        assert_eq!(content, "Hello");
    }

    #[test]
    fn sse_done_is_ignored() {
        assert_eq!("data: [DONE]".strip_prefix("data: "), Some("[DONE]"));
    }
}
