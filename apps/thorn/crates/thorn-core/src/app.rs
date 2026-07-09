use std::collections::VecDeque;

use crate::{BackendCapabilities, Element, KeyAction, KeyIntent, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirtyKind {
    Render,
    Layout,
    Structure,
    Theme,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameInvalidation {
    kind: DirtyKind,
}

impl FrameInvalidation {
    pub const fn new(kind: DirtyKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> DirtyKind {
        self.kind
    }

    pub fn merge(&mut self, other: Self) {
        self.kind = self.kind.max(other.kind);
    }
}

pub trait ThornApp {
    type Action;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>);
    fn view(&self) -> Element<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct AppContext<Action> {
    actions: VecDeque<Action>,
    key_intents: VecDeque<KeyIntent>,
    key_actions: VecDeque<KeyAction<Action>>,
    invalidation: Option<FrameInvalidation>,
    quit_requested: bool,
    backend_capabilities: BackendCapabilities,
    theme: Theme,
}

impl<Action> Default for AppContext<Action> {
    fn default() -> Self {
        Self {
            actions: VecDeque::new(),
            key_intents: VecDeque::new(),
            key_actions: VecDeque::new(),
            invalidation: None,
            quit_requested: false,
            backend_capabilities: BackendCapabilities::text_only(),
            theme: Theme::default(),
        }
    }
}

impl<Action> AppContext<Action> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatch(&mut self, action: Action) {
        self.actions.push_back(action);
    }

    pub fn dispatch_key_intent(&mut self, intent: KeyIntent) {
        self.key_intents.push_back(intent);
    }

    pub fn dispatch_key_action(&mut self, action: KeyAction<Action>) {
        self.key_actions.push_back(action);
    }

    pub fn pop_action(&mut self) -> Option<Action> {
        self.actions.pop_front()
    }

    pub fn pop_key_intent(&mut self) -> Option<KeyIntent> {
        self.key_intents.pop_front()
    }

    pub fn pop_key_action(&mut self) -> Option<KeyAction<Action>> {
        self.key_actions.pop_front()
    }

    pub fn request_render(&mut self) {
        self.request_invalidation(DirtyKind::Render);
    }

    pub fn request_invalidation(&mut self, kind: DirtyKind) {
        match self.invalidation.as_mut() {
            Some(invalidation) => invalidation.merge(FrameInvalidation::new(kind)),
            None => self.invalidation = Some(FrameInvalidation::new(kind)),
        }
    }

    pub fn take_invalidation(&mut self) -> Option<FrameInvalidation> {
        self.invalidation.take()
    }

    pub fn quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn set_backend_capabilities(&mut self, capabilities: BackendCapabilities) {
        self.backend_capabilities = capabilities;
    }

    pub fn backend_capabilities(&self) -> &BackendCapabilities {
        &self.backend_capabilities
    }

    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.theme = theme;
            self.request_invalidation(DirtyKind::Theme);
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidation_merge_keeps_strongest_kind() {
        let mut invalidation = FrameInvalidation::new(DirtyKind::Render);

        invalidation.merge(FrameInvalidation::new(DirtyKind::Layout));
        invalidation.merge(FrameInvalidation::new(DirtyKind::Theme));

        assert_eq!(invalidation.kind(), DirtyKind::Theme);
    }

    #[test]
    fn app_context_merges_requested_invalidations() {
        let mut ctx = AppContext::<()>::new();

        ctx.request_invalidation(DirtyKind::Render);
        ctx.request_invalidation(DirtyKind::Structure);

        assert_eq!(
            ctx.take_invalidation(),
            Some(FrameInvalidation::new(DirtyKind::Structure))
        );
        assert_eq!(ctx.take_invalidation(), None);
    }

    #[test]
    fn set_theme_requests_theme_invalidation_once_per_change() {
        let mut ctx = AppContext::<()>::new();
        let theme = Theme::new(crate::PaintStyle {
            background: Some(crate::PaintColor::Indexed(3)),
            ..crate::PaintStyle::default()
        });

        ctx.set_theme(theme.clone());
        assert_eq!(ctx.theme(), &theme);
        assert_eq!(
            ctx.take_invalidation(),
            Some(FrameInvalidation::new(DirtyKind::Theme))
        );

        ctx.set_theme(theme);
        assert_eq!(ctx.take_invalidation(), None);
    }

    #[test]
    fn invalidation_merge_can_escalate_to_full() {
        let mut invalidation = FrameInvalidation::new(DirtyKind::Render);

        invalidation.merge(FrameInvalidation::new(DirtyKind::Full));

        assert_eq!(invalidation.kind(), DirtyKind::Full);
    }
}
