use std::sync::mpsc;

use aster_domain::{ChatMessage, Conversation, ConversationError, ConversationStatus};
use thiserror::Error;

use crate::port::{
    ChatRequestOptions, ChatStreamError, ChatStreamPort, StreamEvent, StreamReceiver,
};

pub struct ChatSession<C> {
    conversation: Conversation,
    client: C,
    stream_rx: Option<StreamReceiver>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChatSessionError {
    #[error(transparent)]
    Conversation(#[from] ConversationError),
    #[error(transparent)]
    StreamStart(#[from] ChatStreamError),
}

impl<C: ChatStreamPort> ChatSession<C> {
    pub fn new(client: C) -> Self {
        Self {
            conversation: Conversation::new(),
            client,
            stream_rx: None,
        }
    }

    pub fn state(&self) -> &ConversationStatus {
        self.conversation.status()
    }

    pub fn messages(&self) -> &[ChatMessage] {
        self.conversation.messages()
    }

    pub fn send(
        &mut self,
        content: String,
        options: ChatRequestOptions,
    ) -> Result<(), ChatSessionError> {
        match self.conversation.push_user_message(content) {
            Ok(()) => {}
            Err(ConversationError::EmptyUserMessage) => return Ok(()),
            Err(error) => return Err(error.into()),
        }

        match self
            .client
            .start_stream(self.conversation.messages(), &options)
        {
            Ok(rx) => {
                self.conversation.start_assistant_stream()?;
                self.stream_rx = Some(rx);
                Ok(())
            }
            Err(error) => {
                self.conversation.record_error(error.to_string());
                Err(error.into())
            }
        }
    }

    pub fn poll(&mut self) -> usize {
        let Some(rx) = self.stream_rx.as_ref() else {
            return 0;
        };

        let mut new_tokens = 0;
        loop {
            match rx.try_recv() {
                Ok(StreamEvent::Token(token)) => {
                    if let Err(error) = self.conversation.append_assistant_token(&token) {
                        self.conversation.record_error(error.to_string());
                        self.stream_rx = None;
                        break;
                    }
                    new_tokens += 1;
                }
                Ok(StreamEvent::Done) => {
                    self.stream_rx = None;
                    self.conversation.finish_stream();
                    break;
                }
                Ok(StreamEvent::Error(message)) => {
                    self.stream_rx = None;
                    self.conversation.record_error(message);
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.stream_rx = None;
                    self.conversation.finish_stream();
                    break;
                }
            }
        }

        new_tokens
    }

    pub fn cancel_stream(&mut self) {
        if let Some(rx) = self.stream_rx.take() {
            rx.cancel();
            self.conversation.finish_stream();
        }
    }

    pub fn dismiss_error(&mut self) {
        self.conversation.dismiss_error();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::StreamEvent;

    #[derive(Clone)]
    struct FakeClient {
        events: Vec<StreamEvent>,
    }

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            _options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            for event in self.events.clone() {
                tx.send(event).unwrap();
            }
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn empty_message_is_ignored() {
        let mut session = ChatSession::new(FakeClient { events: vec![] });

        session
            .send("   ".to_string(), ChatRequestOptions::new("test-model"))
            .unwrap();

        assert!(session.messages().is_empty());
        assert_eq!(session.state(), &ConversationStatus::Idle);
    }

    #[test]
    fn send_starts_stream_and_poll_appends_tokens() {
        let mut session = ChatSession::new(FakeClient {
            events: vec![
                StreamEvent::Token("hel".to_string()),
                StreamEvent::Token("lo".to_string()),
                StreamEvent::Done,
            ],
        });

        session
            .send("hi".to_string(), ChatRequestOptions::new("test-model"))
            .unwrap();
        let count = session.poll();

        assert_eq!(count, 2);
        assert_eq!(session.messages()[0].content(), "hi");
        assert_eq!(session.messages()[1].content(), "hello");
        assert_eq!(session.state(), &ConversationStatus::Idle);
    }

    #[test]
    fn stream_error_moves_session_to_error_state() {
        let mut session = ChatSession::new(FakeClient {
            events: vec![StreamEvent::Error("network down".to_string())],
        });

        session
            .send("hi".to_string(), ChatRequestOptions::new("test-model"))
            .unwrap();
        session.poll();

        assert_eq!(
            session.state(),
            &ConversationStatus::Error {
                message: "network down".to_string()
            }
        );
    }

    #[test]
    fn cancel_stream_returns_session_to_idle() {
        let mut session = ChatSession::new(FakeClient {
            events: vec![StreamEvent::Token("hello".to_string())],
        });

        session
            .send("hi".to_string(), ChatRequestOptions::new("test-model"))
            .unwrap();
        session.cancel_stream();

        assert_eq!(session.state(), &ConversationStatus::Idle);
        assert_eq!(session.poll(), 0);
    }

    #[derive(Clone)]
    struct RecordingClient {
        tx: mpsc::Sender<String>,
    }

    impl ChatStreamPort for RecordingClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
            options: &ChatRequestOptions,
        ) -> Result<StreamReceiver, ChatStreamError> {
            self.tx.send(options.model.clone()).unwrap();
            let (_tx, rx) = mpsc::channel();
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn send_passes_request_options_to_client() {
        let (tx, rx) = mpsc::channel();
        let mut session = ChatSession::new(RecordingClient { tx });

        session
            .send(
                "hi".to_string(),
                ChatRequestOptions::new("deepseek-reasoner"),
            )
            .unwrap();

        assert_eq!(rx.try_recv().unwrap(), "deepseek-reasoner");
    }
}
