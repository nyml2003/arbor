// App — the TUI application runtime.
// Owns the widget tree, dirty tracker, theme, and coordinates the event/render loops.
// All fallible operations propagate errors via anyhow::Result.

use std::time::Instant;

use anyhow::Context;

use arbor_tui_render::backend::TerminalBackend;
use arbor_tui_render::diff::{diff, merge_regions};
use arbor_tui_reactive::dirty::DirtyTracker;
use arbor_tui_widget::focus::{find_widget_mut, FocusManager};
use arbor_tui_primitives::layout::{Rect, Size};
use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
use arbor_tui_widget::render::render_tree;
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::widget::{WidgetAction, WidgetId, WidgetNode};

/// Frame rate cap — 60fps = ~16.67ms minimum interval.
const MIN_FRAME_INTERVAL_MS: u64 = 16;

/// Outcome of a render pass.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RenderResult {
    /// A frame was rendered and emitted to the terminal.
    Rendered,
    /// Skipped — less than 16ms since the last frame (frame rate cap).
    Throttled,
    /// Skipped — diff found no changes from the previous frame.
    NothingChanged,
}

/// Per-frame performance measurements.
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub frame_seq: u64,
    pub layout_us: u64,
    pub render_us: u64,
    pub diff_us: u64,
    pub emit_us: u64,
    pub emit_queue_us: u64,
    pub emit_flush_us: u64,
    pub total_us: u64,
    pub dirty_widgets: usize,
    pub dirty_regions: usize,
}

/// Application configuration.
pub struct AppConfig {
    pub theme: Theme,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self { theme: Theme::dark() }
    }
}

/// The TUI application runtime.
pub struct App {
    pub config: AppConfig,
    pub dirty_tracker: DirtyTracker,
    pub focus_manager: FocusManager,
    pub last_frame_stats: FrameStats,
    screen: VirtualScreen,
    last_frame_time: Instant,
    frame_seq: u64,
    running: bool,
    next_widget_id: u64,
    // Resize debounce state
    pending_resize: Option<(u16, u16)>,
    last_resize_seen: Instant,
}

impl App {
    pub fn new(cols: u16, rows: u16, config: AppConfig) -> Self {
        Self {
            screen: VirtualScreen::new(cols, rows),
            dirty_tracker: DirtyTracker::new(),
            focus_manager: FocusManager::new(),
            last_frame_stats: FrameStats::default(),
            config,
            last_frame_time: Instant::now(),
            frame_seq: 0,
            running: false,
            next_widget_id: 1,
            pending_resize: None,
            last_resize_seen: Instant::now(),
        }
    }

    pub fn next_widget_id(&mut self) -> WidgetId {
        let id = WidgetId(self.next_widget_id);
        self.next_widget_id += 1;
        id
    }

    pub fn screen_size(&self) -> (u16, u16) {
        (self.screen.cols(), self.screen.rows())
    }

    /// Notify of a potential terminal size change.
    /// Uses debounce: only applies after the size has been stable for `stable_ms`.
    /// Returns true if a resize was applied (caller should rebuild UI + render).
    pub fn check_resize(&mut self, cols: u16, rows: u16, stable_ms: u64) -> bool {
        let (cur_cols, cur_rows) = self.screen_size();
        if cols == cur_cols && rows == cur_rows {
            self.pending_resize = None;
            return false;
        }

        let now = Instant::now();
        let pending = (cols, rows);

        if self.pending_resize == Some(pending) {
            // Same pending size — check if stable long enough
            if now.duration_since(self.last_resize_seen).as_millis() >= stable_ms as u128 {
                self.apply_resize(cols, rows);
                self.pending_resize = None;
                return true;
            }
        } else {
            // New size or changed size — reset the clock
            self.pending_resize = Some(pending);
            self.last_resize_seen = now;
        }
        false
    }

    /// Apply a resize immediately (no debounce). Programmatic use only.
    pub fn apply_resize(&mut self, cols: u16, rows: u16) {
        self.screen = VirtualScreen::new(cols, rows);
        self.dirty_tracker.force_render();
    }

    /// Run the full render pipeline on a widget tree.
    /// measure → layout → render → diff → emit.
    pub fn render_widget_tree(
        &mut self,
        root: &WidgetNode,
        theme: &Theme,
        backend: &mut dyn TerminalBackend,
    ) -> anyhow::Result<RenderResult> {
        self.focus_manager.rebuild(root);

        // Check force_render BEFORE draining — resize sets force_render to
        // guarantee the next frame is not skipped by the throttle.
        let force = !self.dirty_tracker.is_empty();
        let dirty_count = self.dirty_tracker.drain().len();

        if !force {
            let elapsed = self.last_frame_time.elapsed();
            if elapsed.as_millis() < MIN_FRAME_INTERVAL_MS as u128 {
                return Ok(RenderResult::Throttled);
            }
        }
        let frame_start = Instant::now();

        let (cols, rows) = self.screen_size();
        let screen_size = Size { w: cols, h: rows };

        let t0 = Instant::now();
        let constraints = measure_tree(root, screen_size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), root, &constraints)
            .context("layout failed")?;
        let layout_us = t0.elapsed().as_micros() as u64;

        let t1 = Instant::now();
        let new_screen = render_tree((cols, rows), root, &layout, theme);
        let render_us = t1.elapsed().as_micros() as u64;

        let t2 = Instant::now();
        let mut regions = diff(&self.screen, &new_screen);
        merge_regions(&mut regions);
        let diff_us = t2.elapsed().as_micros() as u64;

        let region_count = regions.len();

        if regions.is_empty() {
            return Ok(RenderResult::NothingChanged);
        }

        let t3 = Instant::now();
        backend.emit(&regions, &new_screen)
            .context("backend emit failed")?;
        let emit_us = t3.elapsed().as_micros() as u64;

        self.screen = new_screen;
        self.frame_seq += 1;
        self.last_frame_time = Instant::now();

        self.last_frame_stats = FrameStats {
            frame_seq: self.frame_seq,
            layout_us,
            render_us,
            diff_us,
            emit_us,
            emit_queue_us: backend.last_emit_queue_us(),
            emit_flush_us: backend.last_emit_flush_us(),
            total_us: frame_start.elapsed().as_micros() as u64,
            dirty_widgets: dirty_count,
            dirty_regions: region_count,
        };

        Ok(RenderResult::Rendered)
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Move focus to the next focusable widget (Tab).
    pub fn focus_next(&mut self) -> anyhow::Result<()> {
        let old = self.focus_manager.current();
        let new = self.focus_manager.next()
            .context("focus_next failed")?;
        if old != new {
            if let Some(id) = old { self.dirty_tracker.mark_dirty(id); }
            if let Some(id) = new { self.dirty_tracker.mark_dirty(id); }
        }
        Ok(())
    }

    /// Move focus to the previous focusable widget (Shift+Tab).
    pub fn focus_prev(&mut self) -> anyhow::Result<()> {
        let old = self.focus_manager.current();
        let new = self.focus_manager.prev()
            .context("focus_prev failed")?;
        if old != new {
            if let Some(id) = old { self.dirty_tracker.mark_dirty(id); }
            if let Some(id) = new { self.dirty_tracker.mark_dirty(id); }
        }
        Ok(())
    }

    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focus_manager.current()
    }

    /// Dispatch a key event to the currently focused widget, with event bubbling.
    pub fn dispatch_action(
        &mut self,
        root: &mut WidgetNode,
        action: &WidgetAction,
    ) {
        let target = match self.focus_manager.current() {
            Some(id) => id,
            None => return,
        };

        let mut chain = vec![target];
        chain.extend(self.focus_manager.ancestor_chain(target));

        for widget_id in &chain {
            if let Some(widget) = find_widget_mut(root, *widget_id) {
                let result = widget.perform(action);
                if matches!(result, arbor_tui_primitives::input::KeyHandleResult::Handled) {
                    self.dirty_tracker.mark_dirty(*widget_id);
                    return;
                }
            }
        }
    }

    pub fn run(&mut self, _backend: &mut dyn TerminalBackend) {
        self.running = true;
    }
}
