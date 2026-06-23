use super::animation::ripple_visuals_for_button;
use super::KeyboardSurface;
use crate::app::keyboard::{KeyBehavior, KeySpec};
use arbor_ui_core::geometry::Rect;
use arbor_ui_core::theme::ColorToken;
use arbor_ui_core::view::components::{
    self as c, Align, ButtonIntent, ButtonState, Primitive, RippleVisual, TextWeight,
};
use arbor_ui_core::ViewSnapshot;

impl KeyboardSurface {
    pub(super) fn view_snapshot(&self) -> ViewSnapshot {
        ViewSnapshot {
            surface_rect: self.snapshot.surface_rect,
            primitive_tree: self.primitive_tree(),
        }
    }

    pub(super) fn primitive_tree(&self) -> Primitive {
        let rows = self
            .snapshot
            .row_rects
            .iter()
            .enumerate()
            .map(|(row_index, row_rect)| self.row_primitive(row_index, *row_rect))
            .collect::<Vec<_>>();

        let children = vec![
            self.dock_handle_primitive(),
            self.status_primitive(),
            c::surface(
                "keyboard-body",
                Rect::new(
                    self.snapshot.surface_rect.x,
                    self.snapshot.status_rect.bottom() + self.config.row_gap,
                    self.snapshot.surface_rect.width,
                    self.snapshot.surface_rect.bottom()
                        - self.snapshot.status_rect.bottom()
                        - self.config.row_gap,
                ),
            )
            .children(rows)
            .build(),
        ];

        c::surface("keyboard-surface", self.snapshot.surface_rect)
            .background(ColorToken::Surface)
            .border(ColorToken::Border, 1.0)
            .radius(8.0)
            .children(children)
            .build()
    }

    fn dock_handle_primitive(&self) -> Primitive {
        let close_rect = self.close_title_rect();
        c::row("dock-handle", self.snapshot.title_rect)
            .gap(self.config.key_gap)
            .align(Align::Center)
            .children([
                c::text("title-text", self.snapshot.title_rect)
                    .content("KeyDock")
                    .color(ColorToken::TextPrimary)
                    .size(15.0)
                    .weight(TextWeight::Semibold)
                    .align(Align::Start)
                    .build(),
                c::button("key-close-title", close_rect)
                    .state(ButtonState::Normal)
                    .intent(ButtonIntent::Action)
                    .ripples(self.ripples_for_button("key-close-title", close_rect))
                    .child(
                        c::image("close-icon", close_rect.inset(22.0, 8.0))
                            .tint(ColorToken::TextPrimary)
                            .opacity(1.0)
                            .build(),
                    )
                    .build(),
            ])
            .build()
    }

    fn status_primitive(&self) -> Primitive {
        let labels = [
            ("status-shift", "Shift", self.state.shift_latched),
            ("status-ctrl", "Ctrl", self.state.ctrl_active),
            ("status-alt", "Alt", self.state.alt_active),
        ];
        let width = 72.0;
        let children: Vec<Primitive> = labels
            .iter()
            .enumerate()
            .map(|(index, (id, label, active))| {
                let rect = Rect::new(
                    self.snapshot.status_rect.x + (width + self.config.key_gap) * index as f32,
                    self.snapshot.status_rect.y,
                    width,
                    self.snapshot.status_rect.height,
                );
                c::button(*id, rect)
                    .state(if *active {
                        ButtonState::Active
                    } else {
                        ButtonState::Disabled
                    })
                    .intent(ButtonIntent::Modifier)
                    .child(
                        c::text(format!("{id}-text"), rect)
                            .content(*label)
                            .color(if *active {
                                ColorToken::TextPrimary
                            } else {
                                ColorToken::TextMuted
                            })
                            .size(12.0)
                            .weight(TextWeight::Regular)
                            .align(Align::Center)
                            .build(),
                    )
                    .build()
            })
            .collect();

        c::row("status-indicator", self.snapshot.status_rect)
            .gap(self.config.key_gap)
            .align(Align::Start)
            .children(children)
            .build()
    }

    fn row_primitive(&self, row_index: usize, row_rect: Rect) -> Primitive {
        let children: Vec<Primitive> = self
            .snapshot
            .keys
            .iter()
            .filter(|key| key.row_index == row_index)
            .map(|key| self.key_button(&key.spec, key.rect))
            .collect();

        c::row(format!("key-row-{row_index}"), row_rect)
            .gap(self.config.key_gap)
            .align(Align::Center)
            .children(children)
            .build()
    }

    fn key_button(&self, spec: &KeySpec, rect: Rect) -> Primitive {
        let state = self.button_state(spec);
        let intent = match spec.behavior {
            KeyBehavior::Modifier(_) => ButtonIntent::Modifier,
            KeyBehavior::Action(_) => ButtonIntent::Action,
            KeyBehavior::Character { .. } | KeyBehavior::Space => ButtonIntent::Standard,
        };
        c::button(spec.id.clone(), rect)
            .state(state)
            .intent(intent)
            .ripples(self.ripples_for_button(&spec.id, rect))
            .child(
                c::text(format!("{}-text", spec.id), rect.inset(4.0, 2.0))
                    .content(self.key_label(spec))
                    .color(ColorToken::TextPrimary)
                    .size(14.0)
                    .weight(TextWeight::Regular)
                    .align(Align::Center)
                    .build(),
            )
            .build()
    }

    fn button_state(&self, spec: &KeySpec) -> ButtonState {
        if self.state.pressed_key.as_deref() == Some(spec.id.as_str()) {
            return ButtonState::Pressed;
        }
        if let KeyBehavior::Modifier(modifier) = spec.behavior {
            if self.state.is_modifier_active(modifier) {
                return ButtonState::Active;
            }
        }
        if self.state.hovered_key.as_deref() == Some(spec.id.as_str()) {
            return ButtonState::Hovered;
        }
        ButtonState::Normal
    }

    fn key_label(&self, spec: &KeySpec) -> String {
        if self.state.shift_latched {
            if let Some(label) = &spec.shifted_label {
                return label.clone();
            }
        }
        spec.label.clone()
    }

    fn ripples_for_button(&self, key_id: &str, rect: Rect) -> Vec<RippleVisual> {
        ripple_visuals_for_button(&self.ripples, key_id, rect)
    }
}
