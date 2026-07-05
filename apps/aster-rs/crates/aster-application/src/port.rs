use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use aster_domain::ChatMessage;
use thiserror::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatRequestOptions {
    pub model: String,
}

impl ChatRequestOptions {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

pub struct StreamReceiver {
    rx: mpsc::Receiver<StreamEvent>,
    cancel: StreamCancelToken,
}

impl StreamReceiver {
    pub fn new(rx: mpsc::Receiver<StreamEvent>) -> Self {
        Self {
            rx,
            cancel: StreamCancelToken::new(),
        }
    }

    pub fn with_cancel(rx: mpsc::Receiver<StreamEvent>, cancel: StreamCancelToken) -> Self {
        Self { rx, cancel }
    }

    pub fn try_recv(&self) -> Result<StreamEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}

#[derive(Clone, Default)]
pub struct StreamCancelToken {
    cancelled: Arc<AtomicBool>,
}

impl StreamCancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

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
    fn start_stream(
        &self,
        messages: &[ChatMessage],
        options: &ChatRequestOptions,
    ) -> Result<StreamReceiver, ChatStreamError>;
}

impl<T: ChatStreamPort + ?Sized> ChatStreamPort for Box<T> {
    fn start_stream(
        &self,
        messages: &[ChatMessage],
        options: &ChatRequestOptions,
    ) -> Result<StreamReceiver, ChatStreamError> {
        (**self).start_stream(messages, options)
    }
}
