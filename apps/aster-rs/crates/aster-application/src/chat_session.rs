use std::sync::mpsc;

use aster_domain::{ChatMessage, Conversation, ConversationError, ConversationStatus};
use thiserror::Error;

use crate::port::{ChatStreamError, ChatStreamPort, StreamEvent, StreamReceiver};

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

    pub fn send(&mut self, content: String) -> Result<(), ChatSessionError> {
        match self.conversation.push_user_message(content) {
            Ok(()) => {}
            Err(ConversationError::EmptyUserMessage) => return Ok(()),
            Err(error) => return Err(error.into()),
        }

        match self.client.start_stream(self.conversation.messages()) {
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

        session.send("   ".to_string()).unwrap();

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

        session.send("hi".to_string()).unwrap();
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

        session.send("hi".to_string()).unwrap();
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

        session.send("hi".to_string()).unwrap();
        session.cancel_stream();

        assert_eq!(session.state(), &ConversationStatus::Idle);
        assert_eq!(session.poll(), 0);
    }
}
