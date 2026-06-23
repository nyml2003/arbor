#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    pub id: String,
    pub text: String,
}

impl ClipboardItem {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    PasteText(String),
    CloseApp,
}
