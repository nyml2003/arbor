use aster_application::{ChatSession, ChatStreamPort};
use aster_domain::ConversationStatus;

pub struct AppState {
    chat: ChatSession<Box<dyn ChatStreamPort>>,
    changed: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StreamPollOutcome {
    pub streamed_tokens: usize,
    pub state_changed: bool,
}

impl AppState {
    pub fn new(client: impl ChatStreamPort + 'static) -> Self {
        Self {
            chat: ChatSession::new(Box::new(client)),
            changed: true,
        }
    }

    pub fn chat(&self) -> &ChatSession<Box<dyn ChatStreamPort>> {
        &self.chat
    }

    pub fn submit_message(&mut self, message: String) {
        if matches!(self.chat.state(), ConversationStatus::Idle) {
            let _ = self.chat.send(message);
            self.changed = true;
        }
    }

    pub fn cancel_stream(&mut self) {
        if matches!(self.chat.state(), ConversationStatus::Streaming { .. }) {
            self.chat.cancel_stream();
            self.changed = true;
        }
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.chat.state(), ConversationStatus::Error { .. }) {
            self.chat.dismiss_error();
            self.changed = true;
        }
    }

    pub fn poll_stream_and_take_changed(&mut self) -> StreamPollOutcome {
        let before = self.chat.state().clone();
        let streamed_tokens = self.chat.poll();
        if streamed_tokens > 0 || self.chat.state() != &before {
            self.changed = true;
        }

        let state_changed = self.changed;
        self.changed = false;
        StreamPollOutcome {
            streamed_tokens,
            state_changed,
        }
    }

    #[cfg(test)]
    fn take_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aster_application::{ChatStreamError, StreamEvent, StreamReceiver};
    use aster_domain::ChatMessage;
    use std::sync::mpsc;

    #[derive(Clone)]
    struct FakeClient;

    #[derive(Clone)]
    struct DoneOnlyClient;

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamEvent::Token("hello".to_string())).unwrap();
            tx.send(StreamEvent::Done).unwrap();
            Ok(StreamReceiver::new(rx))
        }
    }

    impl ChatStreamPort for DoneOnlyClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamEvent::Done).unwrap();
            Ok(StreamReceiver::new(rx))
        }
    }

    #[test]
    fn submitting_message_marks_state_changed() {
        let mut state = AppState::new(FakeClient);

        state.take_changed();
        state.submit_message("hi".to_string());

        assert!(state.take_changed());
    }

    #[test]
    fn polling_tokens_marks_state_changed() {
        let mut state = AppState::new(FakeClient);

        state.submit_message("hi".to_string());
        state.take_changed();
        let outcome = state.poll_stream_and_take_changed();

        assert_eq!(outcome.streamed_tokens, 1);
        assert!(outcome.state_changed);
    }

    #[test]
    fn polling_done_marks_state_changed_even_without_new_tokens() {
        let mut state = AppState::new(DoneOnlyClient);

        state.submit_message("hi".to_string());
        state.take_changed();
        let outcome = state.poll_stream_and_take_changed();

        assert_eq!(outcome.streamed_tokens, 0);
        assert_eq!(state.chat().state(), &ConversationStatus::Idle);
        assert!(outcome.state_changed);
    }

    #[test]
    fn cancel_stream_marks_state_changed_and_returns_to_idle() {
        let mut state = AppState::new(FakeClient);

        state.submit_message("hi".to_string());
        state.take_changed();
        state.cancel_stream();

        assert_eq!(state.chat().state(), &ConversationStatus::Idle);
        assert!(state.take_changed());
    }
}
