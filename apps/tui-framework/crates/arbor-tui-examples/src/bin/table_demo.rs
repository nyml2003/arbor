// Table demo — 1000-row data table with scrolling.
// ↑↓/jk:导航  PgUp/PgDn:翻页  Home/End:首尾  ^C/q:退出

use std::io::stdout;

use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};

use arbor_tui_primitives::cell::Attrs;
use arbor_tui_primitives::layout::{Direction, LayoutProps, RectOffset};
use arbor_tui_reactive::signal::ReadSignal;
use arbor_tui_primitives::text::{TruncateStrategy, WrapStrategy};
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::WidgetNode;

use arbor_tui::app::{App, AppConfig, FrameStats, RenderResult};
use arbor_tui::TerminalBackend;
use arbor_tui_backend::crossterm_backend::CrosstermBackend;
use arbor_tui_backend::stdin_reader::StdinReader;
use arbor_tui_primitives::input::{InputReader, Key};
use arbor_tui_widgets::box_widget::BoxWidget;
use arbor_tui_widgets::table_widget::{ColumnDef, ColumnWidth, TableWidget};
use arbor_tui_widgets::text_widget::{TextStyle, TextWidget};
use std::time::{Duration, Instant};

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
            RenderResult::Throttled => {
                self.throttled_frames += 1;
            }
            RenderResult::NothingChanged => {
                self.idle_frames += 1;
            }
        }
    }

    fn fps(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.rendered_frames as f64 / elapsed
        } else {
            0.0
        }
    }

    fn report(&self) -> String {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let n = self.rendered_frames.max(1);

        let avg = |total: u64| -> f64 { total as f64 / n as f64 };

        let fmt_time = |us: u64| -> String {
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
            "\n{sep}\n\
             Arbor TUI — Performance Report\n\
             {sep}\n\
             Elapsed:              {elapsed:>10.3} s\n\
             Frames rendered:      {rendered:>10}\n\
             Throttled (16ms cap): {throttled:>10}\n\
             Idle (no change):     {idle:>10}\n\
             Avg FPS:              {fps:>10.1}\n\
             \n\
             Per-frame timing (n={n}):\n\
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

fn main() {
    if let Err(e) = run() {
        let _ = execute!(stdout(), LeaveAlternateScreen);
        eprintln!("[table_demo] fatal error: {e:?}");
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
    let theme = Theme::dark();
    let (cols, rows) = backend.size()?;
    let mut app = App::new(cols, rows, AppConfig::default());

    let cells = generate_data(1000);
    let mut selected: Option<usize> = Some(0);
    let mut scroll_offset: usize = 0;
    let mut accumulator = FrameAccumulator::new();

    loop {
        let events = input.poll_timeout(Duration::from_millis(100));
        let mut should_quit = false;

        // Header (text + table header + separator) + footer = 4 rows overhead
        let overhead: u16 = 4;
        let visible_rows = rows.saturating_sub(overhead) as usize;

        for event in &events {
            if event.modifiers.ctrl && matches!(event.key, Key::Char('c')) {
                should_quit = true;
            }
            match &event.key {
                Key::Char('q') | Key::Escape => should_quit = true,
                Key::ArrowDown | Key::Char('j') => {
                    selected = Some(selected.map_or(0, |s| (s + 1).min(999)));
                }
                Key::ArrowUp | Key::Char('k') => {
                    selected = Some(selected.map_or(999, |s| s.saturating_sub(1)));
                }
                Key::PageDown => {
                    let s = selected.unwrap_or(0);
                    selected = Some((s + visible_rows.max(1)).min(999));
                }
                Key::PageUp => {
                    let s = selected.unwrap_or(0);
                    selected = Some(s.saturating_sub(visible_rows.max(1)));
                }
                Key::Home => selected = Some(0),
                Key::End => selected = Some(999),
                _ => {}
            }
        }
        if should_quit {
            break;
        }

        // Auto-scroll: keep selected row in the visible viewport
        if let Some(s) = selected {
            if s < scroll_offset {
                scroll_offset = s;
            } else if visible_rows > 0 && s >= scroll_offset + visible_rows {
                scroll_offset = s.saturating_sub(visible_rows.saturating_sub(1));
            }
        }

        let fps = accumulator.fps();
        let last_frame_us = accumulator.last_frame_us;
        let root = build_ui(&theme, &cells, selected, scroll_offset, cols, rows, fps, last_frame_us);
        match app.render_widget_tree(&root, &theme, &mut backend) {
            Ok(result) => accumulator.record(&app.last_frame_stats, result),
            Err(e) => {
                eprintln!("[table_demo] render error: {e:?}");
                break;
            }
        }
    }

    execute!(stdout(), LeaveAlternateScreen)?;

    // Print performance report after returning to normal terminal
    println!("{}", accumulator.report());

    Ok(())
}

fn generate_data(count: usize) -> Vec<Vec<String>> {
    let first_names = [
        "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace",
        "Henry", "Iris", "Jack", "Kate", "Leo", "Mia", "Noah", "Olivia",
    ];
    let last_names = [
        "Smith", "Johnson", "Williams", "Brown", "Jones", "Garcia",
        "Miller", "Davis", "Rodriguez", "Martinez", "Anderson", "Taylor",
    ];
    let domains = [
        "example.com", "arbor.dev", "mail.org", "test.io", "demo.net",
    ];
    let statuses = ["Active", "Inactive", "Pending", "Suspended"];

    (0..count)
        .map(|i| {
            let fn_idx = i % first_names.len();
            let ln_idx = (i * 7 + 3) % last_names.len();
            let first = first_names[fn_idx];
            let last = last_names[ln_idx];
            let name = format!("{} {}", first, last);
            let email = format!(
                "{}.{}@{}",
                first.to_lowercase(),
                last.to_lowercase(),
                domains[i % domains.len()]
            );
            let status = statuses[i % statuses.len()].to_string();
            let balance = format!("${:.2}", (i as f64 * 137.5 + 42.0) % 9999.99);
            let year = 2020 + (i % 6);
            let month = ((i * 3) % 12) + 1;
            let day = ((i * 7) % 28) + 1;
            let registered = format!("{}-{:02}-{:02}", year, month, day);
            vec![
                format!("{}", i + 1),
                name,
                email,
                status,
                balance,
                registered,
            ]
        })
        .collect()
}

fn build_ui(
    theme: &Theme,
    cells: &[Vec<String>],
    selected: Option<usize>,
    scroll_offset: usize,
    cols: u16,
    rows: u16,
    fps: f64,
    last_frame_us: u64,
) -> WidgetNode {
    use arbor_tui_widget::widget::WidgetId;

    let columns = vec![
        ColumnDef {
            header: "ID".into(),
            width: ColumnWidth::Fixed(6),
        },
        ColumnDef {
            header: "Name".into(),
            width: ColumnWidth::Fixed(20),
        },
        ColumnDef {
            header: "Email".into(),
            width: ColumnWidth::Flex(1.0),
        },
        ColumnDef {
            header: "Status".into(),
            width: ColumnWidth::Fixed(12),
        },
        ColumnDef {
            header: "Balance".into(),
            width: ColumnWidth::Fixed(12),
        },
        ColumnDef {
            header: "Registered".into(),
            width: ColumnWidth::Fixed(14),
        },
    ];

    let total = cells.len();
    let overhead: u16 = 4;
    let visible = rows.saturating_sub(overhead) as usize;
    let sel_display = selected.map_or("-".into(), |s| format!("{}", s + 1));
    let to_row = (scroll_offset + visible).min(total);
    let frame_display = if last_frame_us < 1000 {
        format!("{} µs", last_frame_us)
    } else {
        format!("{:.1} ms", last_frame_us as f64 / 1000.0)
    };
    let status_text = format!(
        " Row {}–{}/{}  |  {} visible  |  FPS: {:.0}  frame: {}  |  ↑↓/jk:nav  PgUp/PgDn:page  Home/End:first/last  ^C/q:quit ",
        sel_display,
        to_row,
        total,
        visible,
        fps,
        frame_display,
    );

    let header_style = TextStyle {
        fg: theme.surface(),
        bg: theme.accent(),
        attrs: Attrs {
            bold: true,
            ..Default::default()
        },
    };
    let footer_style = TextStyle {
        fg: theme.accent(),
        bg: theme.surface(),
        attrs: Attrs {
            bold: true,
            ..Default::default()
        },
    };

    WidgetNode::new(BoxWidget {
        id: WidgetId(0),
        props: LayoutProps {
            direction: Direction::Column,
            width: Some(cols),
            height: Some(rows),
            padding: RectOffset {
                top: 0,
                bottom: 0,
                left: 1,
                right: 1,
            },
            ..Default::default()
        },
        children: vec![
            // Title bar
            WidgetNode::new(TextWidget {
                id: WidgetId(1),
                props: LayoutProps::default(),
                text: ReadSignal::constant(" Arbor TUI — 1000-Row Table Demo ".to_string()),
                style: ReadSignal::constant(header_style),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
            // Data table
            WidgetNode::new(TableWidget {
                id: WidgetId(2),
                props: LayoutProps {
                    flex: 1.0,
                    ..Default::default()
                },
                columns,
                cells: cells.to_vec(),
                selected,
                scroll_offset,
                on_select: None,
                on_scroll: None,
                render_cell: None,
            }),
            // Status bar
            WidgetNode::new(TextWidget {
                id: WidgetId(3),
                props: LayoutProps::default(),
                text: ReadSignal::constant(status_text),
                style: ReadSignal::constant(footer_style),
                wrap: WrapStrategy::None,
                truncate: TruncateStrategy::End,
            }),
        ],
    })
}
