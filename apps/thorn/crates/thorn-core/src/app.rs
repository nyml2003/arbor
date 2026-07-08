use std::collections::VecDeque;

use crate::{BackendCapabilities, Element, KeyAction, KeyIntent};

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
    render_requested: bool,
    quit_requested: bool,
    theme: Theme,
    backend_capabilities: BackendCapabilities,
}

impl<Action> Default for AppContext<Action> {
    fn default() -> Self {
        Self {
            actions: VecDeque::new(),
            key_intents: VecDeque::new(),
            key_actions: VecDeque::new(),
            render_requested: false,
            quit_requested: false,
            theme: Theme::default(),
            backend_capabilities: BackendCapabilities::text_only(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
}

impl Default for Theme {
    fn default() -> Self {
        Self { name: "default" }
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
        self.render_requested = true;
    }

    pub fn take_render_requested(&mut self) -> bool {
        let requested = self.render_requested;
        self.render_requested = false;
        requested
    }

    pub fn quit(&mut self) {
        self.quit_requested = true;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
        self.request_render();
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub fn set_backend_capabilities(&mut self, capabilities: BackendCapabilities) {
        self.backend_capabilities = capabilities;
    }

    pub fn backend_capabilities(&self) -> &BackendCapabilities {
        &self.backend_capabilities
    }

    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested
    }
}
