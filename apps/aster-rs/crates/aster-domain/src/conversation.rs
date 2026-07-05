use thiserror::Error;

use crate::message::ChatMessage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConversationStatus {
    Idle,
    Streaming { token_count: usize },
    Error { message: String },
}

impl ConversationStatus {
    pub fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConversationError {
    #[error("message is empty")]
    EmptyUserMessage,
    #[error("conversation is busy")]
    Busy,
    #[error("stream token arrived without an assistant message")]
    MissingAssistantMessage,
}

#[derive(Clone, Debug, Default)]
pub struct Conversation {
    messages: Vec<ChatMessage>,
    status: ConversationStatus,
}

impl Default for ConversationStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl Conversation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn messages(&self) -> &[ChatMessage] {
        &self.messages
    }

    pub fn status(&self) -> &ConversationStatus {
        &self.status
    }

    pub fn push_user_message(&mut self, content: String) -> Result<(), ConversationError> {
        if !self.status.is_idle() {
            return Err(ConversationError::Busy);
        }

        let trimmed = content.trim();
        if trimmed.is_empty() {
            return Err(ConversationError::EmptyUserMessage);
        }

        self.messages.push(ChatMessage::user(trimmed.to_string()));
        Ok(())
    }

    pub fn start_assistant_stream(&mut self) -> Result<(), ConversationError> {
        if !self.status.is_idle() {
            return Err(ConversationError::Busy);
        }

        self.messages.push(ChatMessage::assistant(String::new()));
        self.status = ConversationStatus::Streaming { token_count: 0 };
        Ok(())
    }

    pub fn append_assistant_token(&mut self, token: &str) -> Result<(), ConversationError> {
        match self.messages.last_mut() {
            Some(message) if message.role().as_str() == "assistant" => {
                message.append_content(token);
            }
            _ => return Err(ConversationError::MissingAssistantMessage),
        }

        if let ConversationStatus::Streaming { token_count } = &mut self.status {
            *token_count += 1;
        }

        Ok(())
    }

    pub fn finish_stream(&mut self) {
        self.status = ConversationStatus::Idle;
    }

    pub fn record_error(&mut self, message: impl Into<String>) {
        self.status = ConversationStatus::Error {
            message: message.into(),
        };
    }

    pub fn dismiss_error(&mut self) {
        if self
            .messages
            .last()
            .is_some_and(ChatMessage::is_empty_assistant)
        {
            self.messages.pop();
        }

        self.status = ConversationStatus::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_conversation_is_idle_with_no_messages() {
        let conversation = Conversation::new();

        assert_eq!(conversation.status(), &ConversationStatus::Idle);
        assert!(conversation.messages().is_empty());
    }

    #[test]
    fn empty_user_message_is_rejected() {
        let mut conversation = Conversation::new();

        assert_eq!(
            conversation.push_user_message("   ".to_string()),
            Err(ConversationError::EmptyUserMessage)
        );
        assert!(conversation.messages().is_empty());
    }

    #[test]
    fn streaming_tokens_update_assistant_message_and_count() {
        let mut conversation = Conversation::new();

        conversation.push_user_message("hi".to_string()).unwrap();
        conversation.start_assistant_stream().unwrap();
        conversation.append_assistant_token("he").unwrap();
        conversation.append_assistant_token("llo").unwrap();

        assert_eq!(conversation.messages()[1].content(), "hello");
        assert_eq!(
            conversation.status(),
            &ConversationStatus::Streaming { token_count: 2 }
        );
    }

    #[test]
    fn dismiss_error_removes_empty_assistant_placeholder() {
        let mut conversation = Conversation::new();

        conversation.push_user_message("hi".to_string()).unwrap();
        conversation.start_assistant_stream().unwrap();
        conversation.record_error("failed");
        conversation.dismiss_error();

        assert_eq!(conversation.status(), &ConversationStatus::Idle);
        assert_eq!(conversation.messages().len(), 1);
    }
}
