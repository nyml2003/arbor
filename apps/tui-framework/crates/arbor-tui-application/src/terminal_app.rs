use std::time::Duration;

use anyhow::{bail, Context};
use arbor_tui_domain::backend::{TerminalBackend, TerminalGuard};
use arbor_tui_domain::focus::mount_tree;
use arbor_tui_domain::input::{InputReader, KeyEvent};
use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;

use crate::app::App;
use crate::runtime::{runtime_step, RuntimeInput};
use crate::terminal::install_panic_hook;

pub type RebuildFn = Box<dyn FnMut(u16, u16, &Theme) -> WidgetNode>;
pub type BeforeEventsFn = Box<dyn FnMut(&mut App, &Theme, &mut Vec<KeyEvent>) -> bool>;
pub type BeforeRenderFn = Box<dyn FnMut(&mut App, &mut WidgetNode, &mut Theme) -> bool>;
pub type TerminalAppResult<T> = anyhow::Result<T>;

pub struct TerminalApp {
    theme: Theme,
    root: Option<WidgetNode>,
    rebuild: Option<RebuildFn>,
    before_events: Option<BeforeEventsFn>,
    before_render: Option<BeforeRenderFn>,
    poll_timeout: Duration,
}

impl TerminalApp {
    pub fn new(root: WidgetNode, theme: Theme) -> Self {
        Self {
            theme,
            root: Some(root),
            rebuild: None,
            before_events: None,
            before_render: None,
            poll_timeout: Duration::from_millis(100),
        }
    }

    pub fn with_builder(
        theme: Theme,
        build: impl FnMut(u16, u16, &Theme) -> WidgetNode + 'static,
    ) -> Self {
        Self {
            theme,
            root: None,
            rebuild: Some(Box::new(build)),
            before_events: None,
            before_render: None,
            poll_timeout: Duration::from_millis(100),
        }
    }

    pub fn with_rebuild(
        mut self,
        rebuild: impl FnMut(u16, u16, &Theme) -> WidgetNode + 'static,
    ) -> Self {
        self.rebuild = Some(Box::new(rebuild));
        self
    }

    pub fn before_events(
        mut self,
        callback: impl FnMut(&mut App, &Theme, &mut Vec<KeyEvent>) -> bool + 'static,
    ) -> Self {
        self.before_events = Some(Box::new(callback));
        self
    }

    pub fn before_render(
        mut self,
        callback: impl FnMut(&mut App, &mut WidgetNode, &mut Theme) -> bool + 'static,
    ) -> Self {
        self.before_render = Some(Box::new(callback));
        self
    }

    pub fn poll_timeout(mut self, timeout: Duration) -> Self {
        self.poll_timeout = timeout;
        self
    }

    pub fn run(
        mut self,
        backend: &mut dyn TerminalBackend,
        input: &dyn InputReader,
    ) -> TerminalAppResult<()> {
        install_panic_hook();

        let mut entered_terminal = false;
        let mut raw_mode: Option<Box<dyn TerminalGuard>> = None;
        let result = (|| -> TerminalAppResult<()> {
            backend
                .enter_alternate_screen()
                .context("failed to enter alternate screen")?;
            entered_terminal = true;
            backend.hide_cursor().context("failed to hide cursor")?;
            backend.clear().context("failed to clear terminal")?;
            raw_mode = Some(
                backend
                    .enter_raw_mode()
                    .context("failed to enter raw mode")?,
            );
            self.run_loop(backend, input)
        })();

        input.shutdown();
        drop(raw_mode.take());

        let show_cursor = if entered_terminal {
            backend.show_cursor().context("failed to show cursor")
        } else {
            Ok(())
        };
        let exit_screen = if entered_terminal {
            backend
                .exit_alternate_screen()
                .context("failed to exit alternate screen")
        } else {
            Ok(())
        };

        match result {
            Ok(()) => {
                show_cursor?;
                exit_screen?;
                Ok(())
            }
            Err(err) => {
                let _ = show_cursor;
                let _ = exit_screen;
                Err(err)
            }
        }
    }

    fn run_loop(
        &mut self,
        backend: &mut dyn TerminalBackend,
        input: &dyn InputReader,
    ) -> TerminalAppResult<()> {
        let (cols, rows) = backend.size()?;
        let mut app = App::new(cols, rows);
        app.run();
        let mut root = self.initial_root(cols, rows)?;
        mount_tree(&mut root);

        let mut first_frame = true;
        let mut needs_render = true;
        while app.is_running() {
            let mut events = input.poll_timeout(self.poll_timeout);
            if let Some(before_events) = self.before_events.as_mut() {
                if before_events(&mut app, &self.theme, &mut events) {
                    app.request_render();
                    needs_render = true;
                }
            }
            let runtime_input = if first_frame {
                RuntimeInput::first_frame_with_events(events)
            } else {
                RuntimeInput::new(events)
            };
            let step = runtime_step(&mut app, &mut root, backend, runtime_input)?;

            if step.resized {
                if let Some(rebuild) = self.rebuild.as_mut() {
                    let (cols, rows) = app.screen_size();
                    root = rebuild(cols, rows, &self.theme);
                    mount_tree(&mut root);
                }
                needs_render = true;
            }

            if step.should_clear {
                backend.clear()?;
            }

            if let Some(before_render) = self.before_render.as_mut() {
                if before_render(&mut app, &mut root, &mut self.theme) {
                    mount_tree(&mut root);
                    app.request_render();
                    needs_render = true;
                }
            }

            if first_frame || needs_render || step.should_render {
                app.render_widget_tree(&root, &self.theme, backend)
                    .context("render failed")?;
                if first_frame {
                    let _ = app.focus_next();
                }
                needs_render = false;
            }

            first_frame = false;
            if step.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn initial_root(&mut self, cols: u16, rows: u16) -> TerminalAppResult<WidgetNode> {
        if let Some(root) = self.root.take() {
            return Ok(root);
        }
        if let Some(rebuild) = self.rebuild.as_mut() {
            return Ok(rebuild(cols, rows, &self.theme));
        }
        bail!("terminal app has no root widget or builder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_adapters::simulated_backend::SimulatedBackend;
    use arbor_tui_adapters::simulated_input::SimulatedInput;
    use arbor_tui_domain::backend::BackendResult;
    use arbor_tui_domain::diff::DirtyRegion;
    use arbor_tui_domain::input::{Key, KeyEvent, KeyEventKind, Modifiers};
    use arbor_tui_domain::screen::VirtualScreen;
    use arbor_tui_widgets::text::Text;
    use arbor_tui_widgets::widget_factory::WidgetFactory;
    use std::cell::Cell as StdCell;
    use std::rc::Rc;

    #[test]
    fn terminal_app_renders_first_frame() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let root = Text::new("hello").build(&factory, &theme);
        let mut backend = SimulatedBackend::new(20, 3);
        let input = SimulatedInput::new();
        input.push(ctrl_char('c'));

        TerminalApp::new(root, theme)
            .run(&mut backend, &input)
            .expect("terminal app should run");

        assert!(screen_contains(&backend, "hello"));
    }

    #[test]
    fn terminal_app_builder_uses_backend_size() {
        let theme = Theme::dark();
        let mut backend = SimulatedBackend::new(30, 4);
        let input = SimulatedInput::new();
        input.push(ctrl_char('c'));

        TerminalApp::with_builder(theme, |cols, rows, theme| {
            let factory = WidgetFactory::new();
            Text::new(format!("{cols}x{rows}")).build(&factory, theme)
        })
        .run(&mut backend, &input)
        .expect("terminal app should run");

        assert!(screen_contains(&backend, "30x4"));
    }

    #[test]
    fn terminal_app_before_render_can_update_theme_and_root() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let root = Text::new("dark").build(&factory, &theme);
        let mut backend = SimulatedBackend::new(20, 3);
        let input = SimulatedInput::new();
        input.push(ctrl_char('c'));
        let mut switched = false;

        TerminalApp::new(root, theme)
            .before_render(move |_app, root, theme| {
                if switched {
                    return false;
                }
                switched = true;
                *theme = Theme::light();
                let factory = WidgetFactory::new();
                *root = Text::new("light").build(&factory, theme);
                true
            })
            .run(&mut backend, &input)
            .expect("terminal app should run");

        assert!(screen_contains(&backend, "light"));
    }

    #[test]
    fn terminal_app_before_events_can_filter_events() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let root = Text::new("events").build(&factory, &theme);
        let mut backend = SimulatedBackend::new(20, 3);
        let input = SimulatedInput::new();
        input.push_batch([KeyEvent::char('x'), ctrl_char('c')]);
        let filtered = Rc::new(StdCell::new(false));
        let filtered_for_hook = Rc::clone(&filtered);

        TerminalApp::new(root, theme)
            .before_events(move |_app, _theme, events| {
                let before = events.len();
                events.retain(|event| event.key != Key::Char('x'));
                let changed = events.len() != before;
                filtered_for_hook.set(changed);
                changed
            })
            .run(&mut backend, &input)
            .expect("terminal app should run");

        assert!(filtered.get());
        assert!(screen_contains(&backend, "events"));
    }

    #[test]
    fn terminal_app_normal_exit_drops_raw_guard_without_emergency_restore() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let root = Text::new("guard").build(&factory, &theme);
        let restore_calls = Rc::new(StdCell::new(0));
        let drop_calls = Rc::new(StdCell::new(0));
        let mut backend = GuardProbeBackend::new(Rc::clone(&restore_calls), Rc::clone(&drop_calls));
        let input = SimulatedInput::new();
        input.push(ctrl_char('c'));

        TerminalApp::new(root, theme)
            .run(&mut backend, &input)
            .expect("terminal app should run");

        assert_eq!(restore_calls.get(), 0);
        assert_eq!(drop_calls.get(), 1);
        assert!(backend.show_cursor_called.get());
        assert!(backend.exit_alternate_screen_called.get());
    }

    fn ctrl_char(c: char) -> KeyEvent {
        KeyEvent {
            key: Key::Char(c),
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
            kind: KeyEventKind::Press,
        }
    }

    fn screen_contains(backend: &SimulatedBackend, needle: &str) -> bool {
        let chars = needle.chars().collect::<Vec<_>>();
        for row in 0..backend.screen().rows() {
            for col in 0..backend.screen().cols() {
                if col + chars.len() as u16 > backend.screen().cols() {
                    continue;
                }
                if chars.iter().enumerate().all(|(offset, ch)| {
                    backend.screen().cell_at(col + offset as u16, row).ch == *ch
                }) {
                    return true;
                }
            }
        }
        false
    }

    struct GuardProbeBackend {
        screen: VirtualScreen,
        restore_calls: Rc<StdCell<u32>>,
        drop_calls: Rc<StdCell<u32>>,
        show_cursor_called: Rc<StdCell<bool>>,
        exit_alternate_screen_called: Rc<StdCell<bool>>,
    }

    impl GuardProbeBackend {
        fn new(restore_calls: Rc<StdCell<u32>>, drop_calls: Rc<StdCell<u32>>) -> Self {
            Self {
                screen: VirtualScreen::new(20, 3),
                restore_calls,
                drop_calls,
                show_cursor_called: Rc::new(StdCell::new(false)),
                exit_alternate_screen_called: Rc::new(StdCell::new(false)),
            }
        }
    }

    impl TerminalBackend for GuardProbeBackend {
        fn enter_raw_mode(&self) -> BackendResult<Box<dyn TerminalGuard>> {
            Ok(Box::new(GuardProbe {
                restore_calls: Rc::clone(&self.restore_calls),
                drop_calls: Rc::clone(&self.drop_calls),
            }))
        }

        fn size(&self) -> BackendResult<(u16, u16)> {
            Ok((self.screen.cols(), self.screen.rows()))
        }

        fn emit(&mut self, _regions: &[DirtyRegion], screen: &VirtualScreen) -> BackendResult<()> {
            self.screen = screen.clone();
            Ok(())
        }

        fn hide_cursor(&mut self) -> BackendResult<()> {
            Ok(())
        }

        fn show_cursor(&mut self) -> BackendResult<()> {
            self.show_cursor_called.set(true);
            Ok(())
        }

        fn enter_alternate_screen(&mut self) -> BackendResult<()> {
            Ok(())
        }

        fn exit_alternate_screen(&mut self) -> BackendResult<()> {
            self.exit_alternate_screen_called.set(true);
            Ok(())
        }

        fn clear(&mut self) -> BackendResult<()> {
            Ok(())
        }

        fn flush(&mut self) -> BackendResult<()> {
            Ok(())
        }
    }

    struct GuardProbe {
        restore_calls: Rc<StdCell<u32>>,
        drop_calls: Rc<StdCell<u32>>,
    }

    impl TerminalGuard for GuardProbe {
        fn restore(&mut self) {
            self.restore_calls
                .set(self.restore_calls.get().saturating_add(1));
        }
    }

    impl Drop for GuardProbe {
        fn drop(&mut self) {
            self.drop_calls.set(self.drop_calls.get().saturating_add(1));
        }
    }
}
