use arbor_tui_primitives::input::{Key, KeyEvent};
use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_widget::widget::WidgetNode;

use crate::app::App;
use crate::event_loop::{default_keymap, merge_events};
use crate::signal_manager::check_resize;

pub struct RuntimeInput {
    pub events: Vec<KeyEvent>,
    pub first_frame: bool,
    pub resize: Option<(u16, u16)>,
}

impl RuntimeInput {
    pub fn new(events: Vec<KeyEvent>) -> Self {
        Self {
            events,
            first_frame: false,
            resize: None,
        }
    }

    pub fn first_frame() -> Self {
        Self {
            events: Vec::new(),
            first_frame: true,
            resize: None,
        }
    }

    pub fn resize(cols: u16, rows: u16) -> Self {
        Self {
            events: Vec::new(),
            first_frame: false,
            resize: Some((cols, rows)),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct RuntimeStepResult {
    pub should_render: bool,
    pub should_clear: bool,
    pub should_quit: bool,
    pub resized: bool,
}

pub fn runtime_step(
    app: &mut App,
    root: &mut WidgetNode,
    backend: &dyn TerminalBackend,
    input: RuntimeInput,
) -> anyhow::Result<RuntimeStepResult> {
    let mut result = RuntimeStepResult {
        should_render: input.first_frame,
        ..Default::default()
    };

    if let Some((cols, rows)) = input.resize {
        app.apply_resize(cols, rows);
        result.should_clear = true;
        result.should_render = true;
        result.resized = true;
    } else if check_resize(app, backend, 50)? {
        result.should_clear = true;
        result.should_render = true;
        result.resized = true;
    }

    let merged = merge_events(&input.events);
    for event in &merged {
        match &event.key {
            Key::Char('c') if event.modifiers.ctrl => {
                app.quit();
                result.should_quit = true;
                continue;
            }
            Key::Char('q') if event.modifiers.ctrl => {
                app.quit();
                result.should_quit = true;
                continue;
            }
            Key::Escape => {
                app.quit();
                result.should_quit = true;
                continue;
            }
            Key::Tab if event.modifiers.shift => {
                app.focus_prev()?;
                result.should_render = true;
                continue;
            }
            Key::Tab => {
                app.focus_next()?;
                result.should_render = true;
                continue;
            }
            _ => {}
        }

        if let Some(action) = default_keymap(event) {
            app.dispatch_action(root, &action);
            if !app.dirty_tracker.is_empty() {
                result.should_render = true;
            }
        }
    }

    if !app.dirty_tracker.is_empty() {
        result.should_render = true;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_backend::simulated_backend::SimulatedBackend;
    use arbor_tui_primitives::input::KeyEvent;
    use arbor_tui_render::theme::Theme;
    use arbor_tui_widgets::input::Input;
    use arbor_tui_widgets::widget_factory::WidgetFactory;

    use crate::app::AppConfig;

    #[test]
    fn typing_marks_runtime_for_render() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let mut root = Input::new().build(&factory, &theme);
        let mut app = App::new(20, 1, AppConfig::default());
        let backend = SimulatedBackend::new(20, 1);

        app.run();
        app.focus_manager.rebuild(&root);
        app.focus_next().unwrap();
        app.dirty_tracker.drain();

        let result = runtime_step(
            &mut app,
            &mut root,
            &backend,
            RuntimeInput::new(vec![KeyEvent::char('x')]),
        )
        .unwrap();

        assert!(result.should_render);
    }
}
