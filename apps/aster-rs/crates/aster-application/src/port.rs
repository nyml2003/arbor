use std::sync::mpsc;

use aster_domain::ChatMessage;
use thiserror::Error;

pub type StreamReceiver = mpsc::Receiver<StreamEvent>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StreamEvent {
    Token(String),
    Done,
    Error(String),
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct ChatStreamError {
    message: String,
}

impl ChatStreamError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

pub trait ChatStreamPort {
    fn start_stream(&self, messages: &[ChatMessage]) -> Result<StreamReceiver, ChatStreamError>;
}
