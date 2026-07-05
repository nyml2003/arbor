use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Context;
use arbor_tui_adapters::crossterm_backend::CrosstermBackend;
use arbor_tui_adapters::stdin_reader::StdinReader;
use arbor_tui_application::app::App;
use arbor_tui_application::runtime::{default_keymap, runtime_step, RuntimeInput};
use arbor_tui_application::TerminalBackend;
use arbor_tui_domain::input::{InputReader, Key};
use arbor_tui_domain::signal::Signal;
use arbor_tui_domain::theme::Theme;
use arbor_tui_widgets::widget_factory::WidgetFactory;
use aster_adapters::DeepSeekClient;
use aster_domain::ConversationStatus;

use crate::frame_stats::FrameAccumulator;
use crate::state::AppState;
use crate::ui::{build_ui, UiMetrics};

pub fn run() -> anyhow::Result<()> {
    let mut backend = CrosstermBackend::new();
    backend.enter_alternate_screen()?;
    backend.hide_cursor()?;
    backend.clear()?;
    let _raw_mode = backend.enter_raw_mode()?;

    let input = StdinReader::new();
    let theme = Theme::dark();
    let (mut cols, mut rows) = backend.size()?;
    let mut app = App::new(cols, rows);
    let factory = WidgetFactory::new();
    let client = DeepSeekClient::from_env().context("failed to create DeepSeek client")?;
    let state = Rc::new(RefCell::new(AppState::new(client)));
    let scroll_y = Rc::new(Signal::new(0u16));
    let mut frames = FrameAccumulator::new();
    let mut first = true;
    let mut needs_rebuild = true;
    app.run();

    let mut root = build_ui(
        &factory,
        &theme,
        &state,
        scroll_y.read_only(),
        cols,
        rows,
        UiMetrics {
            fps: frames.fps(),
            last_frame_us: frames.last_frame_us(),
        },
    );

    while app.is_running() {
        let events = input.poll_timeout(Duration::from_millis(100));
        let runtime_input = if first {
            RuntimeInput::first_frame_with_events(Vec::new())
        } else {
            RuntimeInput::new(Vec::new())
        };
        let step = runtime_step(&mut app, &mut root, &backend, runtime_input)?;

        if step.resized {
            (cols, rows) = app.screen_size();
            needs_rebuild = true;
        }

        if step.should_clear {
            backend.clear()?;
        }

        let streamed_tokens = state.borrow_mut().poll_stream();
        if streamed_tokens > 0 {
            app.update_signal(&scroll_y, u16::MAX);
            needs_rebuild = true;
        }

        for event in &events {
            match &event.key {
                Key::Char('c') if event.modifiers.ctrl => app.quit(),
                Key::Char('q') if event.modifiers.ctrl => app.quit(),
                Key::Escape => {
                    if matches!(
                        state.borrow().chat().state(),
                        ConversationStatus::Streaming { .. }
                    ) {
                        state.borrow_mut().dismiss_error();
                        needs_rebuild = true;
                    } else {
                        app.quit();
                    }
                }
                Key::ArrowUp => {
                    let value = scroll_y.get().saturating_sub(1);
                    app.update_signal(&scroll_y, value);
                    needs_rebuild = true;
                }
                Key::ArrowDown => {
                    let value = scroll_y.get().saturating_add(1);
                    app.update_signal(&scroll_y, value);
                    needs_rebuild = true;
                }
                Key::PageUp => {
                    let value = scroll_y.get().saturating_sub(10);
                    app.update_signal(&scroll_y, value);
                    needs_rebuild = true;
                }
                Key::PageDown => {
                    let value = scroll_y.get().saturating_add(10);
                    app.update_signal(&scroll_y, value);
                    needs_rebuild = true;
                }
                Key::Home => {
                    app.update_signal(&scroll_y, 0);
                    needs_rebuild = true;
                }
                Key::End => {
                    app.update_signal(&scroll_y, u16::MAX);
                    needs_rebuild = true;
                }
                Key::Tab if event.modifiers.shift => {
                    let _ = app.focus_prev();
                }
                Key::Tab => {
                    let _ = app.focus_next();
                }
                _ => {
                    if let Some(action) = default_keymap(event) {
                        app.dispatch_action(&mut root, &action);
                    }
                }
            }
        }

        if state.borrow_mut().take_changed() {
            needs_rebuild = true;
        }

        if needs_rebuild {
            root = build_ui(
                &factory,
                &theme,
                &state,
                scroll_y.read_only(),
                cols,
                rows,
                UiMetrics {
                    fps: frames.fps(),
                    last_frame_us: frames.last_frame_us(),
                },
            );
        }

        if first || needs_rebuild || step.should_render {
            let render_result = app.render_widget_tree(&root, &theme, &mut backend)?;
            frames.record(app.last_frame_stats(), render_result);
            if first {
                let _ = app.focus_next();
            }
            needs_rebuild = false;
        }

        first = false;
    }

    input.shutdown();
    backend.show_cursor()?;
    backend.exit_alternate_screen()?;
    println!("{}", frames.report());
    Ok(())
}
