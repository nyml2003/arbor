use super::model::ClipboardItem;

pub const HISTORY_CAPACITY: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardHistory {
    items: Vec<ClipboardItem>,
    next_id: u64,
}

impl Default for ClipboardHistory {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_id: 1,
        }
    }
}

impl ClipboardHistory {
    pub fn items(&self) -> &[ClipboardItem] {
        &self.items
    }

    pub fn push_text(&self, text: impl Into<String>) -> HistoryUpdate {
        let normalized = normalize_text(text.into());
        if normalized.is_empty() {
            return HistoryUpdate {
                history: self.clone(),
                changed: false,
            };
        }

        let mut items = self
            .items
            .iter()
            .filter(|item| item.text != normalized)
            .cloned()
            .collect::<Vec<_>>();
        items.insert(
            0,
            ClipboardItem::new(format!("clip-{}", self.next_id), normalized),
        );
        items.truncate(HISTORY_CAPACITY);

        HistoryUpdate {
            history: Self {
                items,
                next_id: self.next_id + 1,
            },
            changed: true,
        }
    }

    pub fn item_text(&self, id: &str) -> Option<&str> {
        self.items
            .iter()
            .find(|item| item.id == id)
            .map(|item| item.text.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryUpdate {
    pub history: ClipboardHistory,
    pub changed: bool,
}

fn normalize_text(text: String) -> String {
    text.trim_matches(|value: char| value == '\0' || value.is_whitespace())
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_blank_text() {
        let history = ClipboardHistory::default();
        let update = history.push_text(" \n\t ");

        assert!(!update.changed);
        assert!(update.history.items().is_empty());
    }

    #[test]
    fn puts_new_text_first() {
        let history = ClipboardHistory::default().push_text("alpha").history;
        let history = history.push_text("beta").history;

        assert_eq!(history.items()[0].text, "beta");
        assert_eq!(history.items()[1].text, "alpha");
    }

    #[test]
    fn deduplicates_existing_text() {
        let history = ClipboardHistory::default().push_text("alpha").history;
        let history = history.push_text("beta").history;
        let history = history.push_text("alpha").history;

        assert_eq!(history.items().len(), 2);
        assert_eq!(history.items()[0].text, "alpha");
        assert_eq!(history.items()[1].text, "beta");
    }

    #[test]
    fn caps_history() {
        let mut history = ClipboardHistory::default();
        for index in 0..(HISTORY_CAPACITY + 3) {
            history = history.push_text(format!("item-{index}")).history;
        }

        assert_eq!(history.items().len(), HISTORY_CAPACITY);
        assert_eq!(history.items()[0].text, "item-22");
        assert_eq!(history.items()[HISTORY_CAPACITY - 1].text, "item-3");
    }
}
