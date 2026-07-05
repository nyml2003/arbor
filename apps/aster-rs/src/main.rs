// Aster — AI chat TUI.
// DeepSeek API streaming chat, built on arbor-tui.
//
// ↑↓/jk:滚动历史  Enter:发送  Esc/^C:退出

mod api;
mod chat;
mod markdown;

use std::cell::{Cell, RefCell};
use std::io::stdout;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui::app::{App, AppConfig, FrameStats, RenderResult};
use arbor_tui::event_loop::default_keymap;
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_primitives::cell::Attrs;
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_primitives::layout::RectOffset;
use arbor_tui_reactive::signal::{ReadSignal, Signal};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::WidgetNode;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::container::{Col, Row};
use arbor_tui_widgets::input::Input;
use arbor_tui_widgets::rich_text::RichText;
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_manager::WidgetManager;

use chat::{Chat, ChatState};

// ── Frame accumulator ───────────────────────────────────────────────

struct FrameAccumulator {
    rendered_frames: u64,
    throttled_frames: u64,
    idle_frames: u64,
    total_layout_us: u64,
    total_render_us: u64,
    total_diff_us: u64,
    total_emit_us: u64,
    total_emit_queue_us: u64,
    total_emit_flush_us: u64,
    total_total_us: u64,
    min_frame_us: u64,
    max_frame_us: u64,
    last_frame_us: u64,
    start_time: Instant,
}

impl FrameAccumulator {
    fn new() -> Self {
        Self {
            rendered_frames: 0,
            throttled_frames: 0,
            idle_frames: 0,
            total_layout_us: 0,
            total_render_us: 0,
            total_diff_us: 0,
            total_emit_us: 0,
            total_emit_queue_us: 0,
            total_emit_flush_us: 0,
            total_total_us: 0,
            min_frame_us: u64::MAX,
            max_frame_us: 0,
            last_frame_us: 0,
            start_time: Instant::now(),
        }
    }
    fn record(&mut self, s: &FrameStats, r: RenderResult) {
        match r {
            RenderResult::Rendered => {
                self.rendered_frames += 1;
                self.total_layout_us += s.layout_us;
                self.total_render_us += s.render_us;
                self.total_diff_us += s.diff_us;
                self.total_emit_us += s.emit_us;
                self.total_emit_queue_us += s.emit_queue_us;
                self.total_emit_flush_us += s.emit_flush_us;
                self.total_total_us += s.total_us;
                self.min_frame_us = self.min_frame_us.min(s.total_us);
                self.max_frame_us = self.max_frame_us.max(s.total_us);
                self.last_frame_us = s.total_us;
            }
            RenderResult::Throttled => self.throttled_frames += 1,
            RenderResult::NothingChanged => self.idle_frames += 1,
        }
    }
    fn fps(&self) -> f64 {
        let e = self.start_time.elapsed().as_secs_f64();
        if e > 0.0 {
            self.rendered_frames as f64 / e
        } else {
            0.0
        }
    }
    fn report(&self) -> String {
        let e = self.start_time.elapsed().as_secs_f64();
        let n = self.rendered_frames.max(1);
        let a = |t: u64| -> f64 { t as f64 / n as f64 };
        let ft = |us: u64| -> String {
            if us < 1000 {
                format!("{:>4} µs", us)
            } else if us < 1_000_000 {
                format!("{:>5.1} ms", us as f64 / 1000.0)
            } else {
                format!("{:>5.2} s", us as f64 / 1_000_000.0)
            }
        };
        let sep = "═══════════════════════════════════════════";
        format!(
            "\n{sep}\n Aster — Performance Report\n{sep}\n\
            Elapsed:{e:>17.3} s\n  Frames rendered:{ren:>10}\n\
            Throttled (16ms):{thr:>10}\n  Idle (no change):{idl:>10}\n\
            Avg FPS:{fps:>17.1}\n\n  Per-frame timing (n={n}):\n\
            Layout:      avg {al}\n  Render:      avg {ar}\n\
            Diff:        avg {ad}\n  Emit queue:  avg {aeq}\n\
            Emit flush:  avg {aef}\n  Emit total:  avg {ae}\n\
            ─────────────────────────────\n\
            Total:       avg {at}   min {min}   max {max}\n{sep}\n",
            sep = sep,
            e = e,
            ren = self.rendered_frames,
            thr = self.throttled_frames,
            idl = self.idle_frames,
            fps = self.fps(),
            n = n,
            al = ft(a(self.total_layout_us) as u64),
            ar = ft(a(self.total_render_us) as u64),
            ad = ft(a(self.total_diff_us) as u64),
            aeq = ft(a(self.total_emit_queue_us) as u64),
            aef = ft(a(self.total_emit_flush_us) as u64),
            ae = ft(a(self.total_emit_us) as u64),
            at = ft(a(self.total_total_us) as u64),
            min = ft(if n > 0 { self.min_frame_us } else { 0 }),
            max = ft(self.max_frame_us)
        )
    }
}

// ── Entry point ────────────────────────────────────────────────────

fn main() {
    if let Err(e) = run() {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[aster] fatal error: {e:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let mut backend = CrosstermBackend::new();
    execute!(stdout(), EnterAlternateScreen)?;
    backend.hide_cursor()?;
    backend.clear()?;
    let _guard = backend.enter_raw_mode()?;
    let input = StdinReader::new();
    let theme = Rc::new(RefCell::new(Theme::dark()));
    let (mut cols, mut rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());
    let wm = WidgetManager::new();

    let client = api::DeepSeekClient::from_env().map_err(|e| {
        execute!(stdout(), LeaveAlternateScreen).ok();
        e
    })?;
    let chat = Rc::new(RefCell::new(Chat::new(client)));
    let scroll_sig = Rc::new(Signal::new(0u16));
    let total_lines = Rc::new(Cell::new(0usize));
    let needs_rebuild = Rc::new(Cell::new(true));
    let mut accumulator = FrameAccumulator::new();

    let mut root = build_ui(
        &wm,
        &theme.borrow(),
        &chat.borrow(),
        scroll_sig.read_only(),
        &total_lines,
        cols,
        rows,
        accumulator.fps(),
        accumulator.last_frame_us,
        &chat,
        &scroll_sig,
        &needs_rebuild,
    );

    loop {
        let (nc, nr) = backend.size()?;
        if nc != cols || nr != rows {
            if app.check_resize(nc, nr, 50) {
                cols = nc;
                rows = nr;
                backend.clear()?;
                root = build_ui(
                    &wm,
                    &theme.borrow(),
                    &chat.borrow(),
                    scroll_sig.read_only(),
                    &total_lines,
                    cols,
                    rows,
                    accumulator.fps(),
                    accumulator.last_frame_us,
                    &chat,
                    &scroll_sig,
                    &needs_rebuild,
                );
            }
        }

        let tokens = chat.borrow_mut().poll().map_or(0, |n| n);
        if tokens > 0 {
            scroll_sig.set(u16::MAX, &mut app.dirty_tracker); // auto-scroll to bottom (clamped by layout, &mut app.dirty_tracker)
            needs_rebuild.set(true);
        }

        let was_rebuilt = needs_rebuild.get();
        if was_rebuilt {
            needs_rebuild.set(false);
            root = build_ui(
                &wm,
                &theme.borrow(),
                &chat.borrow(),
                scroll_sig.read_only(),
                &total_lines,
                cols,
                rows,
                accumulator.fps(),
                accumulator.last_frame_us,
                &chat,
                &scroll_sig,
                &needs_rebuild,
            );
        }

        if was_rebuilt || !app.dirty_tracker.is_empty() {
            match app.render_widget_tree(&root, &theme.borrow(), &mut backend) {
                Ok(r) => accumulator.record(&app.last_frame_stats, r),
                Err(e) => {
                    eprintln!("[aster] render: {e:?}");
                    break;
                }
            }
            if was_rebuilt {
                let _ = app.focus_next();
            }
        }

        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('z')) {
                should_quit = true;
            }
            match &event.key {
                Key::Escape => {
                    if !matches!(chat.borrow().state(), ChatState::Streaming { .. }) {
                        should_quit = true;
                    }
                }
                // Scroll keys
                Key::ArrowUp => {
                    let v = scroll_sig.read_only().get().saturating_add(1);
                    scroll_sig.set(v, &mut app.dirty_tracker);
                    needs_rebuild.set(true);
                }
                Key::ArrowDown => {
                    let v = scroll_sig.read_only().get().saturating_sub(1);
                    scroll_sig.set(v, &mut app.dirty_tracker);
                    needs_rebuild.set(true);
                }
                Key::PageUp => {
                    scroll_sig.set(scroll_sig.read_only().get() + 10, &mut app.dirty_tracker);
                    needs_rebuild.set(true);
                }
                Key::PageDown => {
                    scroll_sig.set(
                        scroll_sig.read_only().get().saturating_sub(10),
                        &mut app.dirty_tracker,
                    );
                    needs_rebuild.set(true);
                }
                Key::Home => {
                    scroll_sig.set(u16::MAX, &mut app.dirty_tracker);
                    needs_rebuild.set(true);
                }
                Key::End => {
                    scroll_sig.set(0, &mut app.dirty_tracker);
                    needs_rebuild.set(true);
                }
                Key::Tab if event.modifiers.shift => {
                    let _ = app.focus_prev();
                }
                Key::Tab => {
                    let _ = app.focus_next();
                }
                _ => {
                    if let Some(a) = default_keymap(event) {
                        app.dispatch_action(&mut root, &a);
                    }
                }
            }
        }
        if should_quit {
            break;
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    println!("{}", accumulator.report());
    Ok(())
}

// ── UI builder ─────────────────────────────────────────────────────

fn build_ui(
    wm: &WidgetManager,
    t: &Theme,
    chat: &Chat,
    scroll_read: ReadSignal<u16>,
    total_lines_out: &Cell<usize>,
    cols: u16,
    rows: u16,
    fps: f64,
    last_frame_us: u64,
    chat_rc: &Rc<RefCell<Chat>>,
    scroll_sig: &Rc<Signal<u16>>,
    rebuild_flag: &Rc<Cell<bool>>,
) -> WidgetNode {
    let title = match chat.state() {
        ChatState::Idle => " Aster — Chat ".to_string(),
        ChatState::Streaming { token_count } => format!(" Aster — {token_count} tokens "),
        ChatState::Error { .. } => " Aster — Error ".to_string(),
    };

    // ── Messages: per-message blocks with Border for code ──
    let msg_width = cols.saturating_sub(6); // outer border(2) + msg padding(2) + code border(2)
    let (msg_widgets, line_count) = build_message_blocks(chat, t, wm, msg_width as usize);
    total_lines_out.set(line_count);

    // ── Input ──
    let c = chat_rc.clone();
    let s = scroll_sig.clone();
    let r = rebuild_flag.clone();

    // ── Footer ──
    let fd = if last_frame_us < 1000 {
        format!("{} µs", last_frame_us)
    } else {
        format!("{:.1} ms", last_frame_us as f64 / 1000.0)
    };
    let footer_text = format!(
        "{} lines  |  FPS: {:.0}  frame: {}  |  ↑↓/jk:scroll  Enter:send  Esc/^C:quit",
        line_count, fps, fd,
    );

    // ── Layout ──
    let inner = Col::new()
        .flex(1.0)
        .children([
            Col::new().flex(1.0).children(msg_widgets).build(wm, t),
            Border::new()
                .fg(t.border())
                .bg(t.surface())
                .child(
                    Input::new()
                        .placeholder("Type a message…")
                        .on_submit(move |msg| {
                            if matches!(c.borrow().state(), ChatState::Idle) {
                                if c.borrow_mut().send(msg).is_err() {}
                                // scroll handled by event loop r.set(true);
                            }
                        })
                        .build(wm, t),
                )
                .build(wm, t),
            Text::new(footer_text)
                .fg(t.text_dim())
                .bg(t.surface())
                .padding(RectOffset {
                    left: 1,
                    ..Default::default()
                })
                .build(wm, t),
        ])
        .build(wm, t);

    Border::new()
        .title(title)
        .rounded()
        .padding(RectOffset {
            top: 1,
            bottom: 1,
            left: 1,
            right: 1,
        })
        .fg(t.accent())
        .bg(t.surface())
        .child(inner)
        .build(wm, t)
}

// ── Message blocks → widget nodes ──────────────────────────────────

fn build_message_blocks(
    chat: &Chat,
    t: &Theme,
    wm: &WidgetManager,
    width: usize,
) -> (Vec<WidgetNode>, usize) {
    use arbor_tui_primitives::cell::Span;
    let plain = |s: &str| Span::new(s.to_string(), t.text(), t.surface(), Attrs::default());

    let messages = chat.messages();
    if messages.is_empty() {
        let spans = vec![vec![plain(
            "  Welcome to Aster. Type a message and press Enter.",
        )]];
        let w = RichText::new().lines(spans).build(wm, t);
        return (vec![w], 1);
    }

    let mut widgets: Vec<WidgetNode> = Vec::new();
    let mut total = 0usize;

    for msg in messages {
        let (label, color) = match msg.role.as_str() {
            "user" => ("You", t.accent()),
            "assistant" => ("Aster", t.primary()),
            other => (other, t.text()),
        };

        // Label line
        let label_spans = vec![vec![
            plain("  "),
            Span::new(
                format!("{label}: "),
                color,
                t.surface(),
                Attrs {
                    bold: true,
                    ..Default::default()
                },
            ),
        ]];
        widgets.push(RichText::new().lines(label_spans).build(wm, t));
        total += 1;

        if msg.content.is_empty() {
            total += 1;
            widgets.push(RichText::new().lines(vec![vec![plain("")]]).build(wm, t));
            continue;
        }

        let blocks = markdown::parse_blocks(&msg.content, t);
        for block in blocks {
            match block {
                markdown::Block::Text(spans) => {
                    let text_spans: Vec<Vec<Span>> = spans
                        .into_iter()
                        .map(|mut line| {
                            let indent_bg = line.first().map(|s| s.bg).unwrap_or(t.surface());
                            line.insert(
                                0,
                                Span::new(
                                    "    ".to_string(),
                                    t.text(),
                                    indent_bg,
                                    Attrs::default(),
                                ),
                            );
                            line
                        })
                        .collect();
                    let h = text_spans.len();
                    widgets.push(RichText::new().lines(text_spans).build(wm, t));
                    total += h;
                }
                markdown::Block::Code {
                    lang,
                    lines: code_lines,
                } => {
                    let code_spans: Vec<Vec<Span>> = code_lines;
                    let h = code_spans.len();
                    // Wrap code in Border widget
                    let label = if lang.is_empty() {
                        String::new()
                    } else {
                        format!(" {} ", lang)
                    };
                    let code_widget = Border::new()
                        .title(label)
                        .fg(t.border())
                        .child(
                            RichText::new()
                                .lines(code_spans)
                                .padding(RectOffset {
                                    left: 1,
                                    right: 1,
                                    top: 1,
                                    bottom: 1,
                                })
                                .build(wm, t),
                        )
                        .build(wm, t);
                    widgets.push(code_widget);
                    total += h + 4; // code lines + border top/bottom
                }
            }
        }

        // Gap between messages
        widgets.push(RichText::new().lines(vec![vec![plain("")]]).build(wm, t));
        total += 1;
    }

    // Error state
    if let ChatState::Error { message } = chat.state() {
        let err_spans = vec![
            vec![Span::new(
                format!("  ⚠ Error: {message}"),
                t.danger(),
                t.surface(),
                Attrs::default(),
            )],
            vec![plain("  Press any key to dismiss.")],
        ];
        widgets.push(RichText::new().lines(err_spans).build(wm, t));
        total += 2;
    }

    (widgets, total)
}
