use std::collections::HashMap;

use crate::layout::{LayoutInfo, Rect};
use crate::reactive::Scope;
use crate::render::{diff, render_tree, DirtyRegion, Screen};
use crate::theme::{Color, Theme};
use crate::view::{NodeId, PrimitiveNode, View};

pub struct TestApp<Action = ()> {
    scope: Scope,
    root: View<Action>,
    theme: Theme,
    screen: Option<Screen>,
    layout: HashMap<NodeId, LayoutInfo>,
    dirty_regions: Vec<DirtyRegion>,
}

impl<Action> TestApp<Action> {
    pub fn new(root: impl FnOnce(&Scope) -> View<Action>) -> Self {
        let scope = Scope::new();
        let root = scope.enter(|| root(&scope));
        Self {
            scope,
            root,
            theme: Theme::dark(),
            screen: None,
            layout: HashMap::new(),
            dirty_regions: Vec::new(),
        }
    }

    pub fn with_theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn render(&mut self, width: u16, height: u16) {
        let (next_screen, next_layout) = render_tree(&self.root, width, height, &self.theme);
        self.dirty_regions = self
            .screen
            .as_ref()
            .map(|old| diff(old, &next_screen))
            .unwrap_or_else(|| {
                vec![DirtyRegion {
                    rect: Rect::new(0, 0, width, height),
                }]
            });
        self.screen = Some(next_screen);
        self.layout = next_layout;
    }

    pub fn assert_text(&self, text: &str) {
        let Some(screen) = &self.screen else {
            panic!("render must run before asserting text");
        };
        assert!(
            screen.contains_text(text),
            "screen did not contain text `{text}`"
        );
    }

    pub fn press_button(&self, label: &str) {
        let Some(node) = find_button(self.root.node(), label) else {
            panic!("button `{label}` not found");
        };
        for handler in node.on_press_handlers() {
            handler();
        }
    }

    pub fn layout_of(&self, key_or_text: &str) -> Rect {
        let Some(node) = find_by_key_or_text(self.root.node(), key_or_text) else {
            panic!("node `{key_or_text}` not found");
        };
        self.layout
            .get(&node.id())
            .map(|info| info.rect)
            .unwrap_or_else(|| panic!("node `{key_or_text}` has no layout info"))
    }

    pub fn dirty_regions(&self) -> &[DirtyRegion] {
        &self.dirty_regions
    }

    pub fn screen(&self) -> Option<&Screen> {
        self.screen.as_ref()
    }

    pub fn assert_no_default_bg_on_text(&self) {
        let Some(screen) = &self.screen else {
            panic!("render must run before asserting backgrounds");
        };
        for y in 0..screen.height() {
            for x in 0..screen.width() {
                let cell = screen.get(x, y);
                if cell.ch != ' ' {
                    assert_ne!(
                        cell.bg,
                        Color::Palette(0),
                        "text cell at ({x}, {y}) leaked default black background"
                    );
                }
            }
        }
    }

    pub fn dispose(&self) {
        self.scope.dispose();
    }
}

fn find_button<'a, Action>(
    node: &'a PrimitiveNode<Action>,
    label: &str,
) -> Option<&'a PrimitiveNode<Action>> {
    if node.text().as_deref() == Some(label) && !node.on_press_handlers().is_empty() {
        return Some(node);
    }

    node.children()
        .iter()
        .find_map(|child| find_button(child, label))
}

fn find_by_key_or_text<'a, Action>(
    node: &'a PrimitiveNode<Action>,
    key_or_text: &str,
) -> Option<&'a PrimitiveNode<Action>> {
    if node.key() == Some(key_or_text) || node.text().as_deref() == Some(key_or_text) {
        return Some(node);
    }

    node.children()
        .iter()
        .find_map(|child| find_by_key_or_text(child, key_or_text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn harness_renders_text_and_reads_layout() {
        let mut app: TestApp = TestApp::new(|_| panel(text("hello")).id("panel"));
        app.render(20, 4);

        app.assert_text("hello");
        assert_eq!(app.layout_of("panel").w, 20);
    }

    #[test]
    fn light_theme_has_no_default_black_text_background() {
        let mut app: TestApp = TestApp::new(|_| text("hello")).with_theme(Theme::light());
        app.render(20, 4);

        app.assert_no_default_bg_on_text();
    }
}
