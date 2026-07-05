#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Other(String),
}

impl ChatRole {
    pub fn as_str(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Other(role) => role.as_str(),
        }
    }
}

impl From<&str> for ChatRole {
    fn from(value: &str) -> Self {
        match value {
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "system" => Self::System,
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatMessage {
    role: ChatRole,
    content: String,
}

impl ChatMessage {
    pub fn new(role: ChatRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new(ChatRole::User, content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new(ChatRole::Assistant, content)
    }

    pub fn role(&self) -> &ChatRole {
        &self.role
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn append_content(&mut self, token: &str) {
        self.content.push_str(token);
    }

    pub fn is_empty_assistant(&self) -> bool {
        self.role == ChatRole::Assistant && self.content.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_roles_use_wire_names() {
        assert_eq!(ChatRole::User.as_str(), "user");
        assert_eq!(ChatRole::Assistant.as_str(), "assistant");
        assert_eq!(ChatRole::System.as_str(), "system");
    }

    #[test]
    fn unknown_roles_round_trip_as_other() {
        let role = ChatRole::from("tool");

        assert_eq!(role, ChatRole::Other("tool".to_string()));
        assert_eq!(role.as_str(), "tool");
    }
}
