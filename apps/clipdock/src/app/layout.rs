use arbor_ui_core::geometry::{Rect, Size};

use super::history::ClipboardHistory;
use super::model::ClipboardItem;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConfig {
    pub padding: f32,
    pub title_height: f32,
    pub status_height: f32,
    pub item_height: f32,
    pub gap: f32,
    pub max_visible_items: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            padding: 12.0,
            title_height: 34.0,
            status_height: 26.0,
            item_height: 44.0,
            gap: 6.0,
            max_visible_items: 8,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    pub surface_rect: Rect,
    pub title_rect: Rect,
    pub status_rect: Rect,
    pub list_rect: Rect,
    pub close_rect: Rect,
    pub items: Vec<LaidOutClipboardItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutClipboardItem {
    pub item: ClipboardItem,
    pub rect: Rect,
}

pub fn compute_layout(
    history: &ClipboardHistory,
    size: Size,
    config: LayoutConfig,
) -> LayoutSnapshot {
    let surface_rect = Rect::new(0.0, 0.0, size.width.max(1.0), size.height.max(1.0));
    let content = surface_rect.inset(config.padding, config.padding);
    let title_rect = Rect::new(content.x, content.y, content.width, config.title_height);
    let close_rect = Rect::new(
        title_rect.right() - 74.0,
        title_rect.y,
        74.0,
        title_rect.height,
    );
    let status_rect = Rect::new(
        content.x,
        title_rect.bottom() + config.gap,
        content.width,
        config.status_height,
    );
    let list_top = status_rect.bottom() + config.gap;
    let list_rect = Rect::new(
        content.x,
        list_top,
        content.width,
        (content.bottom() - list_top).max(1.0),
    );

    let visible = history.items().iter().take(config.max_visible_items);
    let items = visible
        .enumerate()
        .map(|(index, item)| LaidOutClipboardItem {
            item: item.clone(),
            rect: Rect::new(
                list_rect.x,
                list_rect.y + (config.item_height + config.gap) * index as f32,
                list_rect.width,
                config.item_height,
            ),
        })
        .collect();

    LayoutSnapshot {
        surface_rect,
        title_rect,
        status_rect,
        list_rect,
        close_rect,
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_rects_do_not_overlap() {
        let mut history = ClipboardHistory::default();
        for value in ["one", "two", "three"] {
            history = history.push_text(value).history;
        }

        let snapshot = compute_layout(&history, Size::new(420.0, 520.0), LayoutConfig::default());

        for pair in snapshot.items.windows(2) {
            assert!(pair[0].rect.bottom() < pair[1].rect.y);
        }
    }

    #[test]
    fn limits_visible_items() {
        let mut history = ClipboardHistory::default();
        for index in 0..12 {
            history = history.push_text(format!("item-{index}")).history;
        }

        let config = LayoutConfig {
            max_visible_items: 5,
            ..LayoutConfig::default()
        };
        let snapshot = compute_layout(&history, Size::new(420.0, 520.0), config);

        assert_eq!(snapshot.items.len(), 5);
    }
}
