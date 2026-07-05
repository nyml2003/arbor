use aster_application::{ChatSession, ChatStreamPort};
use aster_domain::ConversationStatus;

pub struct AppState<C> {
    chat: ChatSession<C>,
    changed: bool,
}

impl<C: ChatStreamPort> AppState<C> {
    pub fn new(client: C) -> Self {
        Self {
            chat: ChatSession::new(client),
            changed: true,
        }
    }

    pub fn chat(&self) -> &ChatSession<C> {
        &self.chat
    }

    pub fn submit_message(&mut self, message: String) {
        if matches!(self.chat.state(), ConversationStatus::Idle) {
            let _ = self.chat.send(message);
            self.changed = true;
        }
    }

    pub fn dismiss_error(&mut self) {
        if matches!(self.chat.state(), ConversationStatus::Error { .. }) {
            self.chat.dismiss_error();
            self.changed = true;
        }
    }

    pub fn poll_stream(&mut self) -> usize {
        let tokens = self.chat.poll();
        if tokens > 0 {
            self.changed = true;
        }
        tokens
    }

    pub fn take_changed(&mut self) -> bool {
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

    impl ChatStreamPort for FakeClient {
        fn start_stream(
            &self,
            _messages: &[ChatMessage],
        ) -> Result<StreamReceiver, ChatStreamError> {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamEvent::Token("hello".to_string())).unwrap();
            tx.send(StreamEvent::Done).unwrap();
            Ok(rx)
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
        assert_eq!(state.poll_stream(), 1);

        assert!(state.take_changed());
    }
}
