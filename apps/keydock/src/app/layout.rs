use super::error::{AppError, AppResult};
use super::keyboard::{ActionKind, KeySpec, ModifierKind};
use arbor_ui_core::geometry::{Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConfig {
    pub padding: f32,
    pub title_height: f32,
    pub status_height: f32,
    pub row_gap: f32,
    pub key_gap: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            padding: 12.0,
            title_height: 32.0,
            status_height: 28.0,
            row_gap: 8.0,
            key_gap: 6.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyboardLayout {
    pub rows: Vec<Vec<KeySpec>>,
}

impl KeyboardLayout {
    pub fn qwerty() -> Self {
        Self {
            rows: vec![
                vec![
                    KeySpec::action("key-esc", "Esc", 1.1, ActionKind::Escape),
                    KeySpec::character("1", '1', '!'),
                    KeySpec::character("2", '2', '@'),
                    KeySpec::character("3", '3', '#'),
                    KeySpec::character("4", '4', '$'),
                    KeySpec::character("5", '5', '%'),
                    KeySpec::character("6", '6', '^'),
                    KeySpec::character("7", '7', '&'),
                    KeySpec::character("8", '8', '*'),
                    KeySpec::character("9", '9', '('),
                    KeySpec::character("0", '0', ')'),
                    KeySpec::action("key-backspace", "Backspace", 1.8, ActionKind::Backspace),
                ],
                "qwertyuiop"
                    .chars()
                    .map(|value| {
                        KeySpec::character(
                            &value.to_ascii_uppercase().to_string(),
                            value,
                            value.to_ascii_uppercase(),
                        )
                    })
                    .collect(),
                {
                    let mut row: Vec<KeySpec> = "asdfghjkl"
                        .chars()
                        .map(|value| {
                            KeySpec::character(
                                &value.to_ascii_uppercase().to_string(),
                                value,
                                value.to_ascii_uppercase(),
                            )
                        })
                        .collect();
                    row.push(KeySpec::action(
                        "key-enter",
                        "Enter",
                        1.7,
                        ActionKind::Enter,
                    ));
                    row
                },
                {
                    let mut row = vec![KeySpec::modifier(
                        "key-shift",
                        "Shift",
                        1.8,
                        ModifierKind::Shift,
                    )];
                    row.extend("zxcvbnm".chars().map(|value| {
                        KeySpec::character(
                            &value.to_ascii_uppercase().to_string(),
                            value,
                            value.to_ascii_uppercase(),
                        )
                    }));
                    row
                },
                vec![
                    KeySpec::modifier("key-ctrl", "Ctrl", 1.3, ModifierKind::Control),
                    KeySpec::modifier("key-alt", "Alt", 1.3, ModifierKind::Alt),
                    KeySpec::space(5.8),
                    KeySpec::action("key-close", "Close", 1.4, ActionKind::Close),
                ],
            ],
        }
    }

    pub fn validate(&self) -> AppResult<()> {
        if self.rows.is_empty() {
            return Err(AppError::EmptyLayout);
        }
        for (row_index, row) in self.rows.iter().enumerate() {
            if row.is_empty() {
                return Err(AppError::EmptyRow(row_index));
            }
            for key in row {
                if key.width_units <= 0.0 {
                    return Err(AppError::InvalidKeyWidth(key.id.clone()));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSnapshot {
    pub surface_rect: Rect,
    pub title_rect: Rect,
    pub status_rect: Rect,
    pub row_rects: Vec<Rect>,
    pub keys: Vec<LaidOutKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaidOutKey {
    pub spec: KeySpec,
    pub rect: Rect,
    pub row_index: usize,
}

pub fn compute_layout(
    layout: &KeyboardLayout,
    size: Size,
    config: LayoutConfig,
) -> AppResult<LayoutSnapshot> {
    layout.validate()?;

    let surface_rect = Rect::new(0.0, 0.0, size.width.max(1.0), size.height.max(1.0));
    let content = surface_rect.inset(config.padding, config.padding);
    let title_rect = Rect::new(content.x, content.y, content.width, config.title_height);
    let status_rect = Rect::new(
        content.x,
        title_rect.bottom() + config.row_gap,
        content.width,
        config.status_height,
    );
    let keyboard_top = status_rect.bottom() + config.row_gap;
    let keyboard_height = (content.bottom() - keyboard_top).max(1.0);
    let row_count = layout.rows.len() as f32;
    let total_row_gaps = config.row_gap * (row_count - 1.0).max(0.0);
    let row_height = ((keyboard_height - total_row_gaps) / row_count).max(1.0);

    let mut row_rects = Vec::new();
    let mut keys = Vec::new();

    for (row_index, row) in layout.rows.iter().enumerate() {
        let row_y = keyboard_top + (row_height + config.row_gap) * row_index as f32;
        let row_rect = Rect::new(content.x, row_y, content.width, row_height);
        row_rects.push(row_rect);

        let total_units: f32 = row.iter().map(|key| key.width_units).sum();
        let total_gaps = config.key_gap * (row.len().saturating_sub(1) as f32);
        let unit_width = ((row_rect.width - total_gaps) / total_units).max(1.0);
        let mut x = row_rect.x;
        for spec in row {
            let width = spec.width_units * unit_width;
            keys.push(LaidOutKey {
                spec: spec.clone(),
                rect: Rect::new(x, row_rect.y, width, row_rect.height),
                row_index,
            });
            x += width + config.key_gap;
        }
    }

    Ok(LayoutSnapshot {
        surface_rect,
        title_rect,
        status_rect,
        row_rects,
        keys,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qwerty_contains_required_v1_keys() {
        let layout = KeyboardLayout::qwerty();
        let ids: Vec<&str> = layout
            .rows
            .iter()
            .flatten()
            .map(|key| key.id.as_str())
            .collect();

        for expected in [
            "key-esc",
            "key-backspace",
            "key-enter",
            "key-shift",
            "key-ctrl",
            "key-alt",
            "key-space",
        ] {
            assert!(ids.contains(&expected), "{expected} should exist");
        }
    }

    #[test]
    fn layout_produces_non_overlapping_keys() {
        let snapshot = compute_layout(
            &KeyboardLayout::qwerty(),
            Size::new(900.0, 320.0),
            LayoutConfig::default(),
        )
        .unwrap();

        for row_index in 0..KeyboardLayout::qwerty().rows.len() {
            let row_keys: Vec<_> = snapshot
                .keys
                .iter()
                .filter(|key| key.row_index == row_index)
                .collect();
            for pair in row_keys.windows(2) {
                assert!(pair[0].rect.right() < pair[1].rect.x);
            }
        }
    }

    #[test]
    fn space_is_wider_than_character_key() {
        let snapshot = compute_layout(
            &KeyboardLayout::qwerty(),
            Size::new(900.0, 320.0),
            LayoutConfig::default(),
        )
        .unwrap();
        let space = snapshot
            .keys
            .iter()
            .find(|key| key.spec.id == "key-space")
            .unwrap();
        let a = snapshot
            .keys
            .iter()
            .find(|key| key.spec.id == "key-a")
            .unwrap();

        assert!(space.rect.width > a.rect.width);
    }
}
