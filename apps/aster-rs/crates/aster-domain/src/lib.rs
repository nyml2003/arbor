// aster-domain - pure chat domain model.
// No HTTP, terminal, markdown, or environment access belongs here.

pub mod conversation;
pub mod message;

pub use conversation::{Conversation, ConversationError, ConversationStatus};
pub use message::{ChatMessage, ChatRole};
