// Aster — AI chat TUI.
// DeepSeek API streaming chat, built on arbor-tui.
//
// ↑↓/jk:滚动历史  Enter:发送  ^C/q:退出

mod api;
mod chat;
mod markdown;

use std::io::stdout;
use std::time::{Duration, Instant};

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui::app::{App, AppConfig, FrameStats, RenderResult};
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_primitives::cell::Attrs;
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_primitives::layout::RectOffset;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::WidgetNode;
use arbor_tui_widgets::border::builder::Border;
use arbor_tui_widgets::container::builder::Col;
use arbor_tui_widgets::rich_text::builder::RichText;
use arbor_tui_widgets::text::builder::Text;
use arbor_tui_widgets::widget_manager::WidgetManager;

use chat::{Chat, ChatState};

// ── Accumulator (copied from table_demo, for per-frame perf tracking) ──

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

    fn record(&mut self, stats: &FrameStats, result: RenderResult) {
        match result {
            RenderResult::Rendered => {
                self.rendered_frames += 1;
                self.total_layout_us += stats.layout_us;
                self.total_render_us += stats.render_us;
                self.total_diff_us += stats.diff_us;
                self.total_emit_us += stats.emit_us;
                self.total_emit_queue_us += stats.emit_queue_us;
                self.total_emit_flush_us += stats.emit_flush_us;
                self.total_total_us += stats.total_us;
                self.min_frame_us = self.min_frame_us.min(stats.total_us);
                self.max_frame_us = self.max_frame_us.max(stats.total_us);
                self.last_frame_us = stats.total_us;
            }
            RenderResult::Throttled => self.throttled_frames += 1,
            RenderResult::NothingChanged => self.idle_frames += 1,
        }
    }

    fn fps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 { self.rendered_frames as f64 / elapsed } else { 0.0 }
    }

    fn report(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let n = self.rendered_frames.max(1);
        let avg = |total: u64| -> f64 { total as f64 / n as f64 };
        let fmt_time = |us: u64| -> String {
            if us < 1000 { format!("{:>4} µs", us) }
            else if us < 1_000_000 { format!("{:>5.1} ms", us as f64 / 1000.0) }
            else { format!("{:>5.2} s", us as f64 / 1_000_000.0) }
        };

        let sep = "═══════════════════════════════════════════";
        format!(
            "\n{sep}\n Aster — Performance Report\n{sep}\n\
             Elapsed:              {elapsed:>10.3} s\n\
             Frames rendered:      {rendered:>10}\n\
             Throttled (16ms cap): {throttled:>10}\n\
             Idle (no change):     {idle:>10}\n\
             Avg FPS:              {fps:>10.1}\n\
             \n Per-frame timing (n={n}):\n\
               Layout:      avg {avg_layout}\n\
               Render:      avg {avg_render}\n\
               Diff:        avg {avg_diff}\n\
               Emit queue:  avg {avg_emit_queue}\n\
               Emit flush:  avg {avg_emit_flush}\n\
               Emit total:  avg {avg_emit}\n\
               ─────────────────────────────\n\
               Total:       avg {avg_total}   min {min_total}   max {max_total}\n\
             {sep}\n",
            sep = sep,
            elapsed = elapsed,
            rendered = self.rendered_frames,
            throttled = self.throttled_frames,
            idle = self.idle_frames,
            fps = self.fps(),
            n = n,
            avg_layout = fmt_time(avg(self.total_layout_us) as u64),
            avg_render = fmt_time(avg(self.total_render_us) as u64),
            avg_diff = fmt_time(avg(self.total_diff_us) as u64),
            avg_emit_queue = fmt_time(avg(self.total_emit_queue_us) as u64),
            avg_emit_flush = fmt_time(avg(self.total_emit_flush_us) as u64),
            avg_emit = fmt_time(avg(self.total_emit_us) as u64),
            avg_total = fmt_time(avg(self.total_total_us) as u64),
            min_total = fmt_time(if n > 0 { self.min_frame_us } else { 0 }),
            max_total = fmt_time(self.max_frame_us),
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
    // ── Terminal setup ──
    let mut backend = CrosstermBackend::new();
    execute!(stdout(), EnterAlternateScreen)?;
    backend.hide_cursor()?;
    backend.clear()?;

    let _guard = backend.enter_raw_mode()?;
    let input = StdinReader::new();
    let theme = Theme::dark();
    let (cols, rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());
    let wm = WidgetManager::new();

    // ── Chat setup ──
    let client = api::DeepSeekClient::from_env()
        .map_err(|e| {
            execute!(stdout(), LeaveAlternateScreen).ok();
            e
        })?;
    let mut chat = Chat::new(client);
    let mut input_buffer = String::new();
    let mut scroll_offset: usize = 0;
    let mut accumulator = FrameAccumulator::new();

    // ── Event loop ──
    loop {
        // Poll keyboard + streaming tokens
        let events = input.poll_timeout(Duration::from_millis(16));
        let tokens = chat.poll(); // non-blocking — drains any new tokens from the stream
        let tokens_arrived = tokens.map_or(0, |n| n);
        let should_render = tokens_arrived > 0 || !events.is_empty();

        let mut should_quit = false;

        // ── Input handling ──
        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) {
                should_quit = true;
            }
            match &event.key {
                Key::Escape => {
                    // Only quit on Escape when NOT streaming
                    if !matches!(chat.state(), ChatState::Streaming { .. }) {
                        should_quit = true;
                    }
                }
                // ── Scrolling ──
                Key::ArrowUp | Key::Char('k') => {
                    scroll_offset = scroll_offset.saturating_add(1);
                }
                Key::ArrowDown | Key::Char('j') => {
                    scroll_offset = scroll_offset.saturating_sub(1);
                }
                Key::PageUp => scroll_offset += 10,
                Key::PageDown => scroll_offset = scroll_offset.saturating_sub(10),
                Key::Home => scroll_offset = usize::MAX / 2, // max
                Key::End => scroll_offset = 0,
                // ── Input ──
                Key::Char(c) => {
                    if !event.modifiers.ctrl {
                        input_buffer.push(*c);
                    }
                }
                Key::Backspace => {
                    input_buffer.pop();
                }
                Key::Enter => {
                    if matches!(chat.state(), ChatState::Idle) {
                        let msg = std::mem::take(&mut input_buffer);
                        if let Err(e) = chat.send(msg) {
                            eprintln!("[aster] send error: {e}");
                        }
                        scroll_offset = 0; // scroll to bottom
                    }
                }
                _ => {}
            }
        }

        if should_quit {
            break;
        }

        // Auto-scroll to bottom when streaming new tokens
        if tokens_arrived > 0 {
            scroll_offset = 0;
        }

        // Only rebuild UI when something changed
        if should_render || matches!(chat.state(), ChatState::Streaming { .. }) {
            let root = build_ui(
                &wm,
                &theme,
                &chat,
                &input_buffer,
                scroll_offset,
                cols,
                rows,
                accumulator.fps(),
                accumulator.last_frame_us,
            );

            match app.render_widget_tree(&root, &theme, &mut backend) {
                Ok(result) => accumulator.record(&app.last_frame_stats, result),
                Err(e) => {
                    eprintln!("[aster] render error: {e:?}");
                    break;
                }
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;
    println!("{}", accumulator.report());
    Ok(())
}

// ── UI builder ─────────────────────────────────────────────────────

fn build_ui(
    wm: &WidgetManager,
    theme: &Theme,
    chat: &Chat,
    input_buffer: &str,
    scroll_offset: usize,
    cols: u16,
    rows: u16,
    fps: f64,
    last_frame_us: u64,
) -> WidgetNode {
    // ── Title ──
    let title = match chat.state() {
        ChatState::Idle => " Aster — Chat ".to_string(),
        ChatState::Streaming { token_count } => format!(" Aster — {token_count} tokens "),
        ChatState::Error { .. } => " Aster — Error ".to_string(),
    };

    // ── Messages viewport ──
    let all_spans = build_messages_spans(chat, theme);
    // overhead: border(2) + separator(1) + input(1) + footer(1) + message padding(2)
    let overhead: u16 = 7;
    let visible_rows = rows.saturating_sub(overhead).max(1) as usize;
    let total_lines = all_spans.len();
    let max_scroll = total_lines.saturating_sub(visible_rows);
    let scroll = scroll_offset.min(max_scroll);
    let end = total_lines.saturating_sub(scroll);
    let start = end.saturating_sub(visible_rows);
    let visible_spans = all_spans[start..end].to_vec();

    // ── Input ──
    let placeholder = match chat.state() {
        ChatState::Streaming { .. } => "Waiting for response…",
        ChatState::Error { .. } => "Press any key to dismiss error…",
        ChatState::Idle => "Type a message…",
    };

    // ── Footer ──
    let frame_display = if last_frame_us < 1000 {
        format!("{} µs", last_frame_us)
    } else {
        format!("{:.1} ms", last_frame_us as f64 / 1000.0)
    };
    let scroll_pct = if total_lines <= visible_rows {
        String::new()
    } else {
        format!(" [{}%]", (scroll * 100) / max_scroll.max(1))
    };
    let footer_text = format!(
        "{}/{} lines{}  |  FPS: {:.0}  frame: {}  |  ↑↓/jk:scroll  Enter:send  Esc/^C:quit",
        total_lines, visible_rows, scroll_pct, fps, frame_display,
    );

    // ── Inner content (inside border) ──
    let inner = Col::new()
        .children([
            // Messages
            RichText::new()
                .lines(visible_spans)
                .flex(1.0)
                .padding(RectOffset { top: 1, bottom: 1, left: 1, right: 1 })
                .build(wm, theme),
            // Separator
            Text::new(format!("├{}┤", "─".repeat(cols.saturating_sub(6) as usize)))
                .fg(theme.border()).bg(theme.surface())
                .build(wm, theme),
            // Input
            wm.wrap(|id| arbor_tui_widgets::input::widget::InputWidget {
                id,
                props: arbor_tui_primitives::layout::LayoutProps {
                    padding: RectOffset { left: 1, ..Default::default() },
                    ..Default::default()
                },
                buffer: input_buffer.to_string(),
                cursor: input_buffer.chars().count(),
                placeholder: placeholder.to_string(),
                password: false,
                on_change: None,
                on_submit: None,
            }),
            // Footer
            Text::new(footer_text)
                .fg(theme.text_dim()).bg(theme.surface())
                .padding(RectOffset { left: 1, ..Default::default() })
                .build(wm, theme),
        ])
        .build(wm, theme);

    // ── Wrapped in rounded border ──
    Border::new()
        .title(title)
        .rounded()
        .fg(theme.accent())
        .bg(theme.surface())
        .child(inner)
        .build(wm, theme)
}

/// Format chat messages into styled spans for RichTextWidget.
fn build_messages_spans(chat: &Chat, theme: &Theme) -> Vec<Vec<arbor_tui_primitives::cell::Span>> {
    use arbor_tui_primitives::cell::Span;

    let messages = chat.messages();
    if messages.is_empty() {
        return vec![vec![Span::plain("  Welcome to Aster. Type a message and press Enter.")]];
    }

    let mut lines: Vec<Vec<Span>> = Vec::new();

    for msg in messages {
        // Role label
        let (label, label_color) = match msg.role.as_str() {
            "user" => ("You", theme.accent()),
            "assistant" => ("Aster", theme.primary()),
            other => (other, theme.text()),
        };

        lines.push(vec![
            Span::plain("  "),
            Span::new(
                format!("{label}: "),
                label_color,
                theme.surface(),
                Attrs { bold: true, ..Default::default() },
            ),
        ]);

        // Content — render through markdown
        if msg.content.is_empty() {
            lines.push(vec![Span::plain("")]);
        } else {
            let rendered = markdown::render_message(&msg.content, theme);
            for mut span_line in rendered {
                // Indent each content line
                span_line.insert(0, Span::plain("    "));
                lines.push(span_line);
            }
        }

        // Blank line between messages
        lines.push(vec![Span::plain("")]);
    }

    // Error state
    if let ChatState::Error { message } = chat.state() {
        lines.push(vec![Span::new(
            format!("  ⚠ Error: {message}"),
            theme.danger(),
            theme.surface(),
            Attrs::default(),
        )]);
        lines.push(vec![Span::plain("  Press any key to dismiss.")]);
    }

    lines
}
