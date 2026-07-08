use std::collections::VecDeque;

use crate::Element;

pub trait ThornApp {
    type Action;

    fn update(&mut self, action: Self::Action, ctx: &mut AppContext<Self::Action>);
    fn view(&self) -> Element<Self::Action>;
}

#[derive(Debug, Clone)]
pub struct AppContext<Action> {
    actions: VecDeque<Action>,
    render_requested: bool,
    quit_requested: bool,
}

impl<Action> Default for AppContext<Action> {
    fn default() -> Self {
        Self {
            actions: VecDeque::new(),
            render_requested: false,
            quit_requested: false,
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

    pub fn pop_action(&mut self) -> Option<Action> {
        self.actions.pop_front()
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

    pub fn is_quit_requested(&self) -> bool {
        self.quit_requested
    }
}
