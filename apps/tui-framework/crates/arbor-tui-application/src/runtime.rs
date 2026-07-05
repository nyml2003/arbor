use arbor_tui_domain::backend::TerminalBackend;
use arbor_tui_domain::input::{Key, KeyEvent};
use arbor_tui_domain::widget::{WidgetAction, WidgetNode};

use crate::app::App;
use crate::signal_manager::check_resize;

pub struct RuntimeInput {
    events: Vec<KeyEvent>,
    first_frame: bool,
    resize: Option<(u16, u16)>,
}

impl RuntimeInput {
    pub fn new(events: Vec<KeyEvent>) -> Self {
        Self {
            events,
            first_frame: false,
            resize: None,
        }
    }

    pub fn first_frame_with_events(events: Vec<KeyEvent>) -> Self {
        Self {
            events,
            first_frame: true,
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

/// Merge repeatable navigation events before they reach widgets.
///
/// Text input is not mergeable. Repeated characters are real user input.
pub fn merge_events(events: &[KeyEvent]) -> Vec<KeyEvent> {
    if events.is_empty() {
        return vec![];
    }
    let mut merged: Vec<KeyEvent> = Vec::with_capacity(events.len());
    merged.push(events[0].clone());
    for next in &events[1..] {
        let last = merged
            .last()
            .expect("merged is non-empty after initial push");
        if can_merge(last, next) {
            *merged.last_mut().expect("merged is non-empty") = next.clone();
        } else {
            merged.push(next.clone());
        }
    }
    merged
}

fn can_merge(a: &KeyEvent, b: &KeyEvent) -> bool {
    if a.key != b.key || a.modifiers != b.modifiers {
        return false;
    }
    matches!(
        &a.key,
        Key::ArrowUp
            | Key::ArrowDown
            | Key::ArrowLeft
            | Key::ArrowRight
            | Key::PageUp
            | Key::PageDown
    )
}

/// Map a physical key event to a logical widget action.
///
/// Widgets receive actions, not terminal key events.
pub fn default_keymap(event: &KeyEvent) -> Option<WidgetAction> {
    if event.modifiers.ctrl || event.modifiers.alt {
        return None;
    }
    match &event.key {
        Key::ArrowUp => Some(WidgetAction::NavigateUp),
        Key::ArrowDown => Some(WidgetAction::NavigateDown),
        Key::ArrowLeft => Some(WidgetAction::NavigateLeft),
        Key::ArrowRight => Some(WidgetAction::NavigateRight),
        Key::Enter => Some(WidgetAction::Activate),
        Key::Escape => Some(WidgetAction::Cancel),
        Key::Home => Some(WidgetAction::Home),
        Key::End => Some(WidgetAction::End),
        Key::PageUp => Some(WidgetAction::PageUp),
        Key::PageDown => Some(WidgetAction::PageDown),
        Key::Delete => Some(WidgetAction::Delete),
        Key::Backspace => Some(WidgetAction::Backspace),
        Key::Tab => None,
        Key::Char(c) => Some(WidgetAction::TypeChar(*c)),
        Key::Insert => None,
        Key::F(_) => None,
    }
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
            if app.has_pending_render() {
                result.should_render = true;
            }
        }
    }

    if app.has_pending_render() {
        result.should_render = true;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_adapters::simulated_backend::SimulatedBackend;
    use arbor_tui_domain::input::{Key, KeyEvent, KeyEventKind, Modifiers};
    use arbor_tui_domain::theme::Theme;
    use arbor_tui_widgets::input::Input;
    use arbor_tui_widgets::widget_factory::WidgetFactory;

    #[test]
    fn typing_marks_runtime_for_render() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let mut root = Input::new().build(&factory, &theme);
        let mut app = App::new(20, 1);
        let backend = SimulatedBackend::new(20, 1);

        app.run();
        app.rebuild_focus(&root);
        app.focus_next().unwrap();
        app.take_dirty_widgets();

        let result = runtime_step(
            &mut app,
            &mut root,
            &backend,
            RuntimeInput::new(vec![KeyEvent::char('x')]),
        )
        .unwrap();

        assert!(result.should_render);
    }

    fn ke(key: Key) -> KeyEvent {
        KeyEvent {
            key,
            modifiers: Modifiers::default(),
            kind: KeyEventKind::Press,
        }
    }

    #[test]
    fn merge_repeated_arrows() {
        let events = vec![ke(Key::ArrowUp), ke(Key::ArrowUp), ke(Key::ArrowUp)];
        assert_eq!(merge_events(&events).len(), 1);
    }

    #[test]
    fn merge_keeps_repeated_text_input() {
        let events = vec![
            KeyEvent::char('a'),
            KeyEvent::char('a'),
            KeyEvent::char('a'),
        ];
        assert_eq!(merge_events(&events).len(), 3);
    }

    #[test]
    fn merge_does_not_merge_enter() {
        let events = vec![ke(Key::Enter), ke(Key::Enter)];
        assert_eq!(merge_events(&events).len(), 2);
    }

    #[test]
    fn merge_chain_breaks_on_different_keys() {
        let events = vec![
            KeyEvent::char('a'),
            KeyEvent::char('a'),
            ke(Key::ArrowUp),
            KeyEvent::char('a'),
        ];
        assert_eq!(merge_events(&events).len(), 4);
    }
}
