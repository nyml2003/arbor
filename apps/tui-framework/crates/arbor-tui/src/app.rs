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

/// Application configuration.
pub struct AppConfig {
    pub theme: Theme,
    pub fps_cap: u64, // 30, 60, or 120
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: Theme::dark(),
            fps_cap: 60,
        }
    }
}

/// The TUI application runtime.
pub struct App {
    pub config: AppConfig,
    pub dirty_tracker: DirtyTracker,
    pub focus_manager: FocusManager,
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

    /// Mark all widgets dirty — used after SIGTSTP resume or SIGWINCH.
    pub fn mark_all_dirty(&mut self, widget_ids: &[WidgetId]) {
        self.dirty_tracker.mark_all(widget_ids);
    }

    /// Mark a single widget dirty.
    pub fn mark_dirty(&mut self, id: WidgetId) {
        self.dirty_tracker.mark_dirty(id);
    }

    /// Get the current screen dimensions.
    pub fn screen_size(&self) -> (u16, u16) {
        (self.screen.cols(), self.screen.rows())
    }

    /// Resize the screen (called on SIGWINCH).
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.screen.resize(cols, rows);
    }

    /// Render dirty widgets to the terminal. Returns true if anything was rendered.
    pub fn render(&mut self, backend: &mut dyn TerminalBackend) -> bool {
        // Frame rate throttle
        let elapsed = self.last_frame_time.elapsed();
        if elapsed.as_millis() < MIN_FRAME_INTERVAL_MS as u128 {
            return false;
        }

        let dirty = self.dirty_tracker.drain();
        if dirty.is_empty() {
            return false;
        }

        self.frame_seq += 1;

        // TODO: For each dirty widget, re-layout its subtree and re-render.
        // Currently a placeholder: marks full screen as dirty via resize.
        let old_screen = self.screen.clone();
        // (Widget rendering goes here — fills self.screen with updated cells)

        let regions = diff(&old_screen, &self.screen);
        backend.emit(&regions, &self.screen);

        self.last_frame_time = Instant::now();
        true
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

        let (cols, rows) = self.screen_size();
        let screen_size = Size { w: cols, h: rows };

        let constraints = measure_tree(root, screen_size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), root, &constraints);
        let new_screen = render_tree((cols, rows), root, &layout, theme);

        let mut regions = diff(&self.screen, &new_screen);
        merge_regions(&mut regions);

        if regions.is_empty() && self.frame_seq > 0 {
            return false;
        }

        backend.emit(&regions, &new_screen);
        self.screen = new_screen;
        self.frame_seq += 1;
        self.last_frame_time = Instant::now();
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

/// Run an external subprocess from within the TUI.
///
/// Four-step protocol per TEP-0005:
/// 1. Exit alternate screen, show cursor, restore terminal
/// 2. Spawn subprocess, block until it exits
/// 3. Re-enter raw mode + alternate screen + hide cursor
/// 4. Mark all widgets dirty for full repaint
pub fn run_subprocess(
    cmd: &str,
    args: &[&str],
    backend: &mut dyn TerminalBackend,
    app: &mut App,
) -> std::io::Result<()> {
    // Step 1: restore terminal
    backend.exit_alternate_screen();
    backend.show_cursor();
    backend.flush();

    // Drop raw mode guard by letting the caller manage it.
    // The caller should release their TerminalHandle before calling this.

    // Step 2: spawn and wait
    let status = std::process::Command::new(cmd).args(args).status()?;

    if !status.success() {
        // Subprocess failed — still restore TUI state
        eprintln!("[arbor-tui] subprocess exited with: {}", status);
    }

    // Step 3: re-init terminal
    backend.enter_alternate_screen();
    backend.hide_cursor();
    backend.clear();
    backend.flush();

    // Step 4: full repaint
    let (cols, rows) = backend.size();
    app.resize(cols, rows);
    // Mark everything dirty — caller's next render_widget_tree will repaint

    Ok(())
}
