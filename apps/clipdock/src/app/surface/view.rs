use super::animation::ripple_visuals_for_button;
use super::{close_target_id, ClipDockSurface};
use arbor_ui_core::geometry::Rect;
use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::view::components::{
    self as c, Align, ButtonIntent, ButtonState, Primitive, TextWeight,
};
use arbor_ui_core::ViewSnapshot;

impl ClipDockSurface {
    pub(super) fn view_snapshot(&self) -> ViewSnapshot {
        ViewSnapshot {
            surface_rect: self.snapshot.surface_rect,
            primitive_tree: self.primitive_tree(),
        }
    }

    pub(super) fn primitive_tree(&self) -> Primitive {
        let mut children = vec![self.title_primitive(), self.status_primitive()];

        let list_children = if self.snapshot.items.is_empty() {
            vec![self.empty_state_primitive()]
        } else {
            self.snapshot
                .items
                .iter()
                .map(|item| self.item_button(&item.item.id, &item.item.text, item.rect))
                .collect::<Vec<_>>()
        };
        children.push(
            c::surface("clip-list", self.snapshot.list_rect)
                .children(list_children)
                .build(),
        );

        c::surface("clipdock-surface", self.snapshot.surface_rect)
            .background(ColorToken::Surface)
            .border(ColorToken::Border, 1.0)
            .radius(8.0)
            .children(children)
            .build()
    }

    fn title_primitive(&self) -> Primitive {
        c::row("clip-title", self.snapshot.title_rect)
            .gap(self.config.gap)
            .align(Align::Center)
            .children([
                c::text("clip-title-text", self.snapshot.title_rect)
                    .content("ClipDock")
                    .color(ColorToken::TextPrimary)
                    .size(15.0)
                    .weight(TextWeight::Semibold)
                    .align(Align::Start)
                    .build(),
                c::button(close_target_id(), self.snapshot.close_rect)
                    .state(self.button_state(close_target_id()))
                    .intent(ButtonIntent::Action)
                    .ripples(self.ripples_for_button(close_target_id(), self.snapshot.close_rect))
                    .child(
                        c::text("clip-close-text", self.snapshot.close_rect)
                            .content("Close")
                            .color(ColorToken::TextPrimary)
                            .size(12.0)
                            .weight(TextWeight::Regular)
                            .align(Align::Center)
                            .build(),
                    )
                    .build(),
            ])
            .build()
    }

    fn status_primitive(&self) -> Primitive {
        let count = self.history.items().len();
        let label = if count == 1 {
            "1 text clip".to_string()
        } else {
            format!("{count} text clips")
        };

        c::text("clip-status-text", self.snapshot.status_rect)
            .content(label)
            .color(ColorToken::TextMuted)
            .size(12.0)
            .weight(TextWeight::Regular)
            .align(Align::Start)
            .build()
    }

    fn empty_state_primitive(&self) -> Primitive {
        c::text("clip-empty-text", self.snapshot.list_rect.inset(4.0, 8.0))
            .content("Copy text to start")
            .color(ColorToken::TextMuted)
            .size(13.0)
            .weight(TextWeight::Regular)
            .align(Align::Center)
            .build()
    }

    fn item_button(&self, id: &str, text: &str, rect: Rect) -> Primitive {
        c::button(id.to_string(), rect)
            .state(self.button_state(id))
            .intent(ButtonIntent::Standard)
            .ripples(self.ripples_for_button(id, rect))
            .child(
                c::text(format!("{id}-text"), rect.inset(12.0, 4.0))
                    .content(compact_text(text))
                    .color(ColorToken::TextPrimary)
                    .size(13.0)
                    .weight(TextWeight::Regular)
                    .align(Align::Start)
                    .build(),
            )
            .build()
    }

    fn button_state(&self, target_id: &str) -> ButtonState {
        if self.pressed_target.as_deref() == Some(target_id) {
            return ButtonState::Pressed;
        }
        if self.hovered_target.as_deref() == Some(target_id) {
            return ButtonState::Hovered;
        }
        ButtonState::Normal
    }

    fn ripples_for_button(
        &self,
        target_id: &str,
        rect: Rect,
    ) -> Vec<arbor_ui_core::view::components::RippleVisual> {
        ripple_visuals_for_button(&self.ripples, target_id, rect)
    }
}

fn compact_text(text: &str) -> String {
    let value = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_CHARS: usize = 72;
    if value.chars().count() <= MAX_CHARS {
        return value;
    }

    let mut compact = value.chars().take(MAX_CHARS - 1).collect::<String>();
    compact.push_str("...");
    compact
}
