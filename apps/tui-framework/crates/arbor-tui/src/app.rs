// App — the TUI application runtime.
// Owns the widget tree, dirty tracker, theme, and coordinates the event/render loops.

use std::time::Instant;

use arbor_tui_core::backend::TerminalBackend;
use arbor_tui_core::diff::{diff, merge_regions};
use arbor_tui_core::dirty::DirtyTracker;
use arbor_tui_core::focus::{find_widget_mut, FocusManager};
use arbor_tui_core::layout::{Rect, Size};
use arbor_tui_core::layout_engine::{layout_tree, measure_tree};
use arbor_tui_core::render::render_tree;
use arbor_tui_core::screen::VirtualScreen;
use arbor_tui_core::theme::Theme;
use arbor_tui_core::widget::{WidgetId, WidgetNode};

/// Frame rate cap — 60fps = ~16.67ms minimum interval.
const MIN_FRAME_INTERVAL_MS: u64 = 16;

/// Per-frame performance measurements.
/// Microsecond-level timing for layout, render, diff, and emit phases.
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub frame_seq: u64,
    pub layout_us: u64,
    pub render_us: u64,
    pub diff_us: u64,
    pub emit_us: u64,
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
        Self {
            theme: Theme::dark(),
        }
    }
}

/// The TUI application runtime.
pub struct App {
    pub config: AppConfig,
    pub dirty_tracker: DirtyTracker,
    pub focus_manager: FocusManager,
    /// Timing stats from the most recent frame.
    pub last_frame_stats: FrameStats,
    screen: VirtualScreen,
    last_frame_time: Instant,
    frame_seq: u64,
    running: bool,
    next_widget_id: u64,
}

impl App {
    /// Create a new App with the given screen dimensions.
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
        }
    }

    /// Allocate the next unique WidgetId.
    pub fn next_widget_id(&mut self) -> WidgetId {
        let id = WidgetId(self.next_widget_id);
        self.next_widget_id += 1;
        id
    }

    /// Get the current screen dimensions.
    pub fn screen_size(&self) -> (u16, u16) {
        (self.screen.cols(), self.screen.rows())
    }

    /// Resize the screen (called on SIGWINCH).
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.screen.resize(cols, rows);
    }

    /// Run the full render pipeline on a widget tree.
    /// measure → layout → render → diff → emit.
    /// Also rebuilds the focus tab order from the tree.
    /// Returns true if anything was rendered.
    pub fn render_widget_tree(
        &mut self,
        root: &WidgetNode,
        theme: &Theme,
        backend: &mut dyn TerminalBackend,
    ) -> bool {
        // Rebuild focus tab order before layout
        self.focus_manager.rebuild(root);

        // Frame rate throttle
        let elapsed = self.last_frame_time.elapsed();
        if elapsed.as_millis() < MIN_FRAME_INTERVAL_MS as u128 {
            return false;
        }

        // Drain dirty markers — we're doing a full-frame render.
        // Callers that don't use Signals (e.g. examples) simply drain an empty set.
        let dirty_count = self.dirty_tracker.drain().len();
        let frame_start = Instant::now();

        let (cols, rows) = self.screen_size();
        let screen_size = Size { w: cols, h: rows };

        let t0 = Instant::now();
        let constraints = measure_tree(root, screen_size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), root, &constraints);
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
            return false;
        }

        let t3 = Instant::now();
        backend.emit(&regions, &new_screen);
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
            total_us: frame_start.elapsed().as_micros() as u64,
            dirty_widgets: dirty_count,
            dirty_regions: region_count,
        };

        true
    }

    /// Flag the app to stop running.
    pub fn quit(&mut self) {
        self.running = false;
    }

    /// Check if the app is still running.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Move focus to the next focusable widget (Tab).
    /// Marks both old and new focus widgets dirty for re-render.
    pub fn focus_next(&mut self) {
        let old = self.focus_manager.current();
        let new = self.focus_manager.next();
        if old != new {
            if let Some(id) = old { self.dirty_tracker.mark_dirty(id); }
            if let Some(id) = new { self.dirty_tracker.mark_dirty(id); }
        }
    }

    /// Move focus to the previous focusable widget (Shift+Tab).
    pub fn focus_prev(&mut self) {
        let old = self.focus_manager.current();
        let new = self.focus_manager.prev();
        if old != new {
            if let Some(id) = old { self.dirty_tracker.mark_dirty(id); }
            if let Some(id) = new { self.dirty_tracker.mark_dirty(id); }
        }
    }

    /// Return the currently focused widget ID, if any.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focus_manager.current()
    }

    /// Dispatch a key event to the currently focused widget, with event bubbling.
    /// 1. Try the focused widget's on_key()
    /// 2. If Bubble, walk up the ancestor chain and try each parent
    /// 3. Stop when Handled or reach root
    /// Marks the handling widget dirty.
    pub fn dispatch_key(
        &mut self,
        root: &mut WidgetNode,
        event: &arbor_tui_core::input::KeyEvent,
    ) {
        let target = match self.focus_manager.current() {
            Some(id) => id,
            None => return,
        };

        // Collect the dispatch chain: [target, parent, grandparent, ..., root]
        let mut chain = vec![target];
        chain.extend(self.focus_manager.ancestor_chain(target));

        for widget_id in &chain {
            if let Some(widget) = find_widget_mut(root, *widget_id) {
                let result = widget.on_key(event);
                if matches!(result, arbor_tui_core::input::KeyHandleResult::Handled) {
                    self.dirty_tracker.mark_dirty(*widget_id);
                    return;
                }
                // Bubble → continue to parent
            }
        }
    }

    /// Start the app.
    pub fn run(&mut self, _backend: &mut dyn TerminalBackend) {
        self.running = true;
        // The main event loop is driven externally — see event_loop.rs
    }
}
