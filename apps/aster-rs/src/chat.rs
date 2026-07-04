// Chat state machine — manages message history, streaming state, and API interaction.
//
// Design:
//   - Owns message history (Vec<Message>) and a DeepSeekClient.
//   - Exposes a simple imperative API: send(), poll(), dismiss_error().
//   - The main event loop calls these methods, then reads the resulting state
//     to build the widget tree.

use std::sync::mpsc;

use crate::api::{DeepSeekClient, Message, StreamEvent};

/// The chat system — all mutable state lives here.
pub struct Chat {
    /// Conversation history. The last message may be a partial streaming response.
    messages: Vec<Message>,
    /// API client.
    client: DeepSeekClient,
    /// Current state.
    state: ChatState,
    /// Receiver for streaming tokens. None when not streaming.
    stream_rx: Option<mpsc::Receiver<StreamEvent>>,
}

/// What the chat system is currently doing.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ChatState {
    /// Waiting for the user to send a message.
    Idle,
    /// Streaming a response from the LLM.
    Streaming {
        /// Number of tokens received so far.
        token_count: usize,
    },
    /// An error occurred (API error, network error, etc.).
    Error {
        message: String,
    },
}

impl Chat {
    /// Create a new chat session.
    pub fn new(client: DeepSeekClient) -> Self {
        Self {
            messages: Vec::new(),
            client,
            state: ChatState::Idle,
            stream_rx: None,
        }
    }

    /// Current state of the chat system.
    pub fn state(&self) -> &ChatState {
        &self.state
    }

    /// All messages in the conversation (including partial streaming response).
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Send a user message. Spawns a background thread for the API call.
    ///
    /// Returns an error only if the API call fails to start (e.g. network error
    /// on connect). Streaming errors arrive later via `poll()`.
    pub fn send(&mut self, content: String) -> anyhow::Result<()> {
        // Ignore empty messages
        if content.trim().is_empty() {
            return Ok(());
        }

        // Add user message to history
        self.messages.push(Message {
            role: "user".to_string(),
            content,
        });

        // Start streaming
        match self.client.chat_stream(self.messages.clone()) {
            Ok(rx) => {
                // Add an empty assistant message that will be filled by streaming
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: String::new(),
                });
                self.stream_rx = Some(rx);
                self.state = ChatState::Streaming { token_count: 0 };
                Ok(())
            }
            Err(e) => {
                self.state = ChatState::Error {
                    message: e.to_string(),
                };
                Err(e)
            }
        }
    }

    /// Poll for new streaming tokens. Call this every frame.
    ///
    /// Returns the number of new tokens received this frame, or None if no
    /// streaming is active.
    pub fn poll(&mut self) -> Option<usize> {
        let rx = match self.stream_rx.as_ref() {
            Some(rx) => rx,
            None => return None,
        };

        let mut new_tokens = 0;
        loop {
            match rx.try_recv() {
                Ok(StreamEvent::Token(t)) => {
                    // Append token to the last message (assistant)
                    if let Some(last) = self.messages.last_mut() {
                        last.content.push_str(&t);
                    }
                    new_tokens += 1;
                }
                Ok(StreamEvent::Done) => {
                    self.stream_rx = None;
                    self.state = ChatState::Idle;
                    break;
                }
                Ok(StreamEvent::Error(e)) => {
                    self.stream_rx = None;
                    self.state = ChatState::Error { message: e };
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => {
                    // No more tokens available this frame
                    break;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    // Channel disconnected — treat as done
                    self.stream_rx = None;
                    self.state = ChatState::Idle;
                    break;
                }
            }
        }

        // Update token count in state
        if let ChatState::Streaming { token_count } = &mut self.state {
            *token_count += new_tokens;
        }

        Some(new_tokens)
    }

    /// Dismiss an error and return to idle state.
    /// Removes the failed assistant message placeholder if present.
    #[allow(dead_code)]
    pub fn dismiss_error(&mut self) {
        // Remove the empty assistant message that was added before the error
        if let Some(last) = self.messages.last() {
            if last.role == "assistant" && last.content.is_empty() {
                self.messages.pop();
            }
        }
        self.state = ChatState::Idle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DeepSeekClient;

    #[test]
    fn new_chat_is_idle_with_no_messages() {
        let client = DeepSeekClient::new("sk-test".into(), "deepseek-chat".into());
        let chat = Chat::new(client);
        assert_eq!(chat.state(), &ChatState::Idle);
        assert!(chat.messages().is_empty());
    }

    #[test]
    fn empty_message_is_ignored() {
        let client = DeepSeekClient::new("sk-test".into(), "deepseek-chat".into());
        let mut chat = Chat::new(client);
        // Empty/whitespace messages should not trigger an API call
        assert!(chat.send("   ".to_string()).is_ok());
        assert!(chat.messages().is_empty());
    }

    #[test]
    fn dismiss_error_removes_empty_assistant_message() {
        let client = DeepSeekClient::new("sk-test".into(), "deepseek-chat".into());
        let mut chat = Chat::new(client);
        // Simulate the error state with an empty assistant placeholder
        chat.messages.push(Message { role: "user".into(), content: "hi".into() });
        chat.messages.push(Message { role: "assistant".into(), content: String::new() });
        chat.state = ChatState::Error { message: "test error".into() };

        chat.dismiss_error();
        assert_eq!(chat.state(), &ChatState::Idle);
        assert_eq!(chat.messages().len(), 1); // only user message remains
    }
}
