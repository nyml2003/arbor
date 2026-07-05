use std::cell::RefCell;
use std::rc::Rc;

use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::widget_factory::WidgetFactory;

use crate::app::AppContext;
use crate::ui::{build_root, ActionSink};
use crate::{Node, Ui};

type UpdateFn<State, Action> = Box<dyn FnMut(&mut State, Action, &mut AppContext<Action>)>;
type ViewFn<State, Action> = Box<dyn FnMut(&State, &Ui<Action>) -> Node<Action>>;

pub struct TestApp<State, Action> {
    state: Rc<RefCell<State>>,
    update: UpdateFn<State, Action>,
    view: Rc<RefCell<ViewFn<State, Action>>>,
    theme: Theme,
    factory: Rc<WidgetFactory>,
    actions: ActionSink<Action>,
    running: bool,
}

impl<State, Action> TestApp<State, Action>
where
    State: 'static,
    Action: 'static,
{
    pub fn new(
        state: State,
        update: impl FnMut(&mut State, Action, &mut AppContext<Action>) + 'static,
        view: impl FnMut(&State, &Ui<Action>) -> Node<Action> + 'static,
    ) -> Self {
        Self {
            state: Rc::new(RefCell::new(state)),
            update: Box::new(update),
            view: Rc::new(RefCell::new(Box::new(view))),
            theme: Theme::dark(),
            factory: Rc::new(WidgetFactory::new()),
            actions: ActionSink::new(),
            running: true,
        }
    }

    pub fn theme(mut self, theme: Theme) -> Self {
        self.theme = theme;
        self
    }

    pub fn dispatch(&mut self, action: Action) -> &mut Self {
        self.actions.push(action);
        self.process_actions();
        self
    }

    pub fn render(&mut self, cols: u16, rows: u16) -> TestFrame {
        self.process_actions();
        let ui = Ui::new(
            Rc::clone(&self.factory),
            self.theme.clone(),
            self.actions.clone(),
        );
        let node = (self.view.borrow_mut())(&self.state.borrow(), &ui);
        let root = build_root(&self.factory, &self.theme, cols, rows, node);
        TestFrame {
            harness: WidgetHarness::render(&root, cols, rows, &self.theme),
        }
    }

    pub fn state(&self) -> std::cell::Ref<'_, State> {
        self.state.borrow()
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    fn process_actions(&mut self) {
        while let Some(action) = self.actions.pop() {
            let mut ctx = AppContext::new(self.actions.clone());
            (self.update)(&mut self.state.borrow_mut(), action, &mut ctx);
            if let Some(theme) = ctx.take_theme() {
                self.theme = theme;
            }
            if ctx.should_quit() {
                self.running = false;
            }
        }
    }
}

pub struct TestFrame {
    harness: WidgetHarness,
}

impl TestFrame {
    pub fn assert_text(&self, text: &str) -> &Self {
        assert!(
            !self.harness.find_text(text).is_empty(),
            "expected screen to contain {text:?}\n{}",
            self.visible_text()
        );
        self
    }

    pub fn assert_no_default_bg(&self) -> &Self {
        self.harness
            .assert_no_black_bg_on_text()
            .expect("visible text should not use the default black background");
        self
    }

    pub fn find_text(&self, text: &str) -> Vec<(u16, u16)> {
        self.harness.find_text(text)
    }

    pub fn visible_text(&self) -> String {
        let mut text = String::new();
        for row in 0..self.harness.rows() {
            for col in 0..self.harness.cols() {
                text.push(self.harness.cell_at(col, row).ch);
            }
            if row + 1 < self.harness.rows() {
                text.push('\n');
            }
        }
        text
    }
}
