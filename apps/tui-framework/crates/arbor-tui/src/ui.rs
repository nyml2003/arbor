use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::PhantomData;
use std::rc::Rc;

use arbor_tui_domain::identity::{NodeIdentity, WidgetKey};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::{assign_tree_identity, WidgetNode};
use arbor_tui_widgets::stack::Col;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::component::UiComponent;

pub(crate) struct ActionSink<Action> {
    queue: Rc<RefCell<VecDeque<Action>>>,
}

impl<Action> Clone for ActionSink<Action> {
    fn clone(&self) -> Self {
        Self {
            queue: Rc::clone(&self.queue),
        }
    }
}

impl<Action> ActionSink<Action> {
    pub(crate) fn new() -> Self {
        Self {
            queue: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    pub(crate) fn push(&self, action: Action) {
        self.queue.borrow_mut().push_back(action);
    }

    pub(crate) fn pop(&self) -> Option<Action> {
        self.queue.borrow_mut().pop_front()
    }
}

pub struct Node<Action> {
    widget: WidgetNode,
    action: PhantomData<fn() -> Action>,
}

impl<Action> Node<Action> {
    pub fn from_widget(widget: WidgetNode) -> Self {
        Self {
            widget,
            action: PhantomData,
        }
    }

    pub fn into_widget(self) -> WidgetNode {
        self.widget
    }

    pub fn key(mut self, key: impl Into<WidgetKey>) -> Self {
        self.widget.set_key(key);
        self
    }

    pub fn identity(&self) -> Option<NodeIdentity> {
        self.widget.identity()
    }
}

pub struct Ui<Action> {
    factory: Rc<WidgetFactory>,
    theme: Theme,
    actions: ActionSink<Action>,
}

impl<Action> Clone for Ui<Action> {
    fn clone(&self) -> Self {
        Self {
            factory: Rc::clone(&self.factory),
            theme: self.theme.clone(),
            actions: self.actions.clone(),
        }
    }
}

impl<Action: 'static> Ui<Action> {
    pub(crate) fn new(
        factory: Rc<WidgetFactory>,
        theme: Theme,
        actions: ActionSink<Action>,
    ) -> Self {
        Self {
            factory,
            theme,
            actions,
        }
    }

    pub fn component<Component>(&self, component: Component) -> Node<Action>
    where
        Component: UiComponent<Action>,
    {
        component.render(self)
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    pub(crate) fn factory(&self) -> &WidgetFactory {
        &self.factory
    }

    pub(crate) fn action_callback<T: 'static>(
        &self,
        callback: Box<dyn Fn(T) -> Action>,
    ) -> impl Fn(T) + 'static {
        let actions = self.actions.clone();
        move |value| {
            actions.push(callback(value));
        }
    }
}

pub(crate) fn build_root<Action: 'static>(
    factory: &Rc<WidgetFactory>,
    theme: &Theme,
    cols: u16,
    rows: u16,
    node: Node<Action>,
) -> WidgetNode {
    let mut root = Col::new()
        .size(cols, rows)
        .children([node.into_widget()])
        .build(factory, theme);
    assign_tree_identity(&mut root).expect("widget tree identity assignment failed");
    root
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_widgets::text::Text;

    #[test]
    fn node_key_sets_widget_identity_metadata() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let node =
            Node::<()>::from_widget(Text::new("hello").build(&factory, &theme)).key("greeting");

        assert_eq!(
            node.identity(),
            Some(NodeIdentity::Keyed(WidgetKey::new("greeting")))
        );
    }
}
