use std::io::{BufRead, BufReader, Read};
use std::sync::mpsc;
use std::thread;

use aster_application::{ChatStreamError, ChatStreamPort, StreamEvent, StreamReceiver};
use aster_domain::ChatMessage;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ApiMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

#[derive(Serialize, Clone)]
struct ApiMessage {
    role: String,
    content: String,
}

impl From<&ChatMessage> for ApiMessage {
    fn from(message: &ChatMessage) -> Self {
        Self {
            role: message.role().as_str().to_string(),
            content: message.content().to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
struct Choice {
    delta: Option<Delta>,
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

pub struct DeepSeekClient {
    api_key: String,
    base_url: String,
    model: String,
}

impl DeepSeekClient {
    pub fn from_env() -> Result<Self, ChatStreamError> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .map_err(|_| {
                ChatStreamError::new(
                    "DEEPSEEK_API_KEY or OPENAI_API_KEY environment variable not set",
                )
            })?;

        let base_url = std::env::var("DEEPSEEK_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com".to_string());
        let model = std::env::var("DEEPSEEK_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());

        Ok(Self {
            api_key,
            base_url,
            model,
        })
    }

    pub fn new(api_key: String, model: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.deepseek.com".to_string(),
            model,
        }
    }
}

impl ChatStreamPort for DeepSeekClient {
    fn start_stream(&self, messages: &[ChatMessage]) -> Result<StreamReceiver, ChatStreamError> {
        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.iter().map(ApiMessage::from).collect(),
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
                    Err(error) => {
                        let _ = tx.send(StreamEvent::Error(error.to_string()));
                    }
                }
            })
            .map_err(|error| ChatStreamError::new(error.to_string()))?;

        Ok(rx)
    }
}

fn stream_sse(
    url: &str,
    api_key: &str,
    request: &ChatRequest,
    tx: &mpsc::Sender<StreamEvent>,
) -> Result<(), ChatStreamError> {
    let response = ureq::post(url)
        .header("Authorization", &format!("Bearer {api_key}"))
        .header("Accept", "text/event-stream")
        .send_json(request)
        .map_err(|error| ChatStreamError::new(error.to_string()))?;

    let reader: Box<dyn Read + Send> = Box::new(response.into_body().into_reader());
    let buf = BufReader::new(reader);

    for line in buf.lines() {
        let line = line.map_err(|error| ChatStreamError::new(error.to_string()))?;
        if let Some(event) = parse_sse_data_line(&line)? {
            match event {
                StreamEvent::Done => break,
                StreamEvent::Token(_) | StreamEvent::Error(_) => {
                    if tx.send(event).is_err() {
                        return Ok(());
                    }
                }
            }
        }
    }

    Ok(())
}

fn parse_sse_data_line(line: &str) -> Result<Option<StreamEvent>, ChatStreamError> {
    let Some(data) = line.strip_prefix("data: ") else {
        return Ok(None);
    };

    if data == "[DONE]" {
        return Ok(Some(StreamEvent::Done));
    }

    if let Ok(response) = serde_json::from_str::<ChatResponse>(data) {
        let token = response
            .choices
            .into_iter()
            .filter_map(|choice| choice.delta)
            .filter_map(|delta| delta.content)
            .collect::<String>();

        if token.is_empty() {
            return Ok(None);
        }

        return Ok(Some(StreamEvent::Token(token)));
    }

    if let Ok(error) = serde_json::from_str::<ErrorResponse>(data) {
        return Ok(Some(StreamEvent::Error(format!(
            "API error: {}",
            error.error.message
        ))));
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_domain::{ChatMessage, ChatRole};

    #[test]
    fn request_serializes_wire_roles() {
        let req = ChatRequest {
            model: "deepseek-chat".into(),
            messages: vec![ApiMessage::from(&ChatMessage::new(ChatRole::User, "hello"))],
            stream: true,
            max_tokens: Some(100),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&req).unwrap();

        assert!(json.contains("\"stream\":true"));
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"hello\""));
    }

    #[test]
    fn parses_token_sse_line() {
        let event =
            parse_sse_data_line(r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#).unwrap();

        assert_eq!(event, Some(StreamEvent::Token("Hello".to_string())));
    }

    #[test]
    fn parses_done_sse_line() {
        assert_eq!(
            parse_sse_data_line("data: [DONE]").unwrap(),
            Some(StreamEvent::Done)
        );
    }

    #[test]
    fn parses_api_error_sse_line() {
        let event = parse_sse_data_line(r#"data: {"error":{"message":"bad key"}}"#).unwrap();

        assert_eq!(
            event,
            Some(StreamEvent::Error("API error: bad key".to_string()))
        );
    }
}
