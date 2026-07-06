// App — the TUI application runtime.
// Owns runtime state and coordinates focus, dirty tracking, resize, and rendering.
// All fallible operations propagate errors via anyhow::Result.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Instant;

use anyhow::Context;

use arbor_tui_domain::backend::TerminalBackend;
use arbor_tui_domain::cache::{
    FocusState, LayoutCacheEntry, LayoutCacheShadow, MeasureCacheEntry, RenderCacheEntry,
    RenderCacheShadow,
};
use arbor_tui_domain::diff::{diff, merge_regions};
use arbor_tui_domain::dirty::DirtyTracker;
use arbor_tui_domain::focus::{find_widget_mut, FocusManager};
use arbor_tui_domain::frame::FrameSnapshot;
use arbor_tui_domain::identity::DirtyKind;
use arbor_tui_domain::layout::{
    Align, AxisConstraint, Direction, Justify, LayoutProps, Rect, Size, SizeConstraint,
};
use arbor_tui_domain::layout_engine::{layout_tree, measure_tree};
use arbor_tui_domain::render::{render_tree, render_tree_with_fragments};
use arbor_tui_domain::screen::VirtualScreen;
use arbor_tui_domain::signal::{Signal, SignalChange, SignalId};
use arbor_tui_domain::theme::{Theme, ThemeVariant};
use arbor_tui_domain::widget::{WidgetAction, WidgetId, WidgetNode};

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
    pub dirty_render_widgets: usize,
    pub dirty_layout_widgets: usize,
    pub dirty_structure_widgets: usize,
    pub dirty_theme_widgets: usize,
    pub dirty_full_widgets: usize,
    pub dirty_regions: usize,
    pub focus_rebuilt: bool,
    pub layout_cache_hits: usize,
    pub layout_cache_misses: usize,
    pub layout_cache_mismatches: usize,
    pub render_cache_hits: usize,
    pub render_cache_misses: usize,
    pub render_cache_mismatches: usize,
}

/// The TUI application runtime.
pub struct App {
    dirty_tracker: DirtyTracker,
    pending_signal_dirty: HashMap<WidgetId, DirtyKind>,
    pending_signal_generations: HashMap<SignalId, u64>,
    focus_manager: FocusManager,
    last_frame_stats: FrameStats,
    last_frame_snapshot: FrameSnapshot,
    screen: VirtualScreen,
    last_frame_time: Instant,
    frame_seq: u64,
    running: bool,
    focus_dirty: bool,
    cache_shadow_enabled: bool,
    layout_cache_shadow: LayoutCacheShadow,
    render_cache_shadow: RenderCacheShadow,
    // Resize debounce state
    pending_resize: Option<(u16, u16)>,
    last_resize_seen: Instant,
}

impl App {
    pub fn new(cols: u16, rows: u16) -> Self {
        Self {
            screen: VirtualScreen::new(cols, rows),
            dirty_tracker: DirtyTracker::new(),
            pending_signal_dirty: HashMap::new(),
            pending_signal_generations: HashMap::new(),
            focus_manager: FocusManager::new(),
            last_frame_stats: FrameStats::default(),
            last_frame_snapshot: FrameSnapshot::default(),
            last_frame_time: Instant::now(),
            frame_seq: 0,
            running: false,
            focus_dirty: true,
            cache_shadow_enabled: false,
            layout_cache_shadow: LayoutCacheShadow::new(),
            render_cache_shadow: RenderCacheShadow::new(),
            pending_resize: None,
            last_resize_seen: Instant::now(),
        }
    }

    pub fn screen_size(&self) -> (u16, u16) {
        (self.screen.cols(), self.screen.rows())
    }

    pub fn screen(&self) -> &VirtualScreen {
        &self.screen
    }

    pub fn last_frame_stats(&self) -> &FrameStats {
        &self.last_frame_stats
    }

    pub fn last_frame_snapshot(&self) -> &FrameSnapshot {
        &self.last_frame_snapshot
    }

    pub fn enable_cache_shadow(&mut self, enabled: bool) {
        self.cache_shadow_enabled = enabled;
        if !enabled {
            self.layout_cache_shadow = LayoutCacheShadow::new();
            self.render_cache_shadow = RenderCacheShadow::new();
        }
    }

    pub(crate) fn has_pending_render(&self) -> bool {
        !self.dirty_tracker.is_empty() || !self.pending_signal_dirty.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn take_dirty_widgets(&mut self) -> Vec<WidgetId> {
        self.take_dirty_widget_kinds().into_keys().collect()
    }

    fn take_dirty_widget_kinds(&mut self) -> HashMap<WidgetId, DirtyKind> {
        self.drain_pending_signals_into_dirty_tracker();
        self.dirty_tracker.drain()
    }

    pub fn request_render(&mut self) {
        self.dirty_tracker.force_render();
    }

    pub fn request_focus_rebuild(&mut self) {
        self.focus_dirty = true;
    }

    pub(crate) fn rebuild_focus(&mut self, root: &WidgetNode) {
        self.focus_manager.rebuild(root);
        self.focus_dirty = false;
    }

    pub fn update_signal<T: Clone + PartialEq>(&mut self, signal: &Signal<T>, value: T) {
        if let Some(change) = signal.set_collect(value) {
            self.enqueue_signal_change(change);
        }
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
        self.request_focus_rebuild();
    }

    /// Run the full render pipeline on a widget tree.
    /// measure → layout → render → diff → emit.
    pub fn render_widget_tree(
        &mut self,
        root: &WidgetNode,
        theme: &Theme,
        backend: &mut dyn TerminalBackend,
    ) -> anyhow::Result<RenderResult> {
        let focus_rebuilt = if self.focus_dirty {
            self.rebuild_focus(root);
            true
        } else {
            false
        };

        // Check force_render BEFORE draining — resize sets force_render to
        // guarantee the next frame is not skipped by the throttle.
        let force = self.has_pending_render();
        let dirty_widgets = self.take_dirty_widget_kinds();
        let dirty_count = dirty_widgets.len();
        let dirty_counts = count_dirty_kinds(dirty_widgets.values().copied());

        if self.frame_seq > 0 && !force {
            let elapsed = self.last_frame_time.elapsed();
            if elapsed.as_millis() < MIN_FRAME_INTERVAL_MS as u128 {
                return Ok(RenderResult::Throttled);
            }
        }
        let frame_start = Instant::now();

        let (cols, rows) = self.screen_size();
        let screen_size = Size { w: cols, h: rows };
        self.last_frame_snapshot =
            FrameSnapshot::new(self.frame_seq.saturating_add(1), collect_signal_deps(root));

        let t0 = Instant::now();
        let constraints = measure_tree(root, screen_size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), root, &constraints)
            .context("layout failed")?;
        if self.cache_shadow_enabled {
            observe_layout_shadow(
                &mut self.layout_cache_shadow,
                root,
                screen_size,
                Rect::new(0, 0, cols, rows),
                &constraints,
                &layout,
            );
        }
        let layout_us = t0.elapsed().as_micros() as u64;

        let t1 = Instant::now();
        let focused = self.focus_manager.current();
        let new_screen = if self.cache_shadow_enabled {
            let theme_revision = theme_rev(theme);
            let render_cache_shadow = &mut self.render_cache_shadow;
            render_tree_with_fragments(
                (cols, rows),
                root,
                &layout,
                theme,
                focused,
                |node, rect, screen| {
                    render_cache_shadow.observe(
                        node.id(),
                        RenderCacheEntry {
                            rect,
                            theme_rev: theme_revision,
                            focus_state: FocusState::from_focus(node.id(), focused),
                            render_rev: hash_widget_identity(node),
                            screen: screen.clone(),
                        },
                    );
                },
            )
        } else {
            render_tree((cols, rows), root, &layout, theme, focused)
        };
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
        backend
            .emit(&regions, &new_screen)
            .context("backend emit failed")?;
        let emit_us = t3.elapsed().as_micros() as u64;

        self.screen = new_screen;
        self.frame_seq += 1;
        self.last_frame_time = Instant::now();

        let (
            layout_cache_hits,
            layout_cache_misses,
            layout_cache_mismatches,
            render_cache_hits,
            render_cache_misses,
            render_cache_mismatches,
        ) = if self.cache_shadow_enabled {
            (
                self.layout_cache_shadow.measure_stats.hits
                    + self.layout_cache_shadow.layout_stats.hits,
                self.layout_cache_shadow.measure_stats.misses
                    + self.layout_cache_shadow.layout_stats.misses,
                self.layout_cache_shadow.measure_stats.mismatches
                    + self.layout_cache_shadow.layout_stats.mismatches,
                self.render_cache_shadow.stats.hits,
                self.render_cache_shadow.stats.misses,
                self.render_cache_shadow.stats.mismatches,
            )
        } else {
            (0, 0, 0, 0, 0, 0)
        };

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
            dirty_render_widgets: dirty_counts.render,
            dirty_layout_widgets: dirty_counts.layout,
            dirty_structure_widgets: dirty_counts.structure,
            dirty_theme_widgets: dirty_counts.theme,
            dirty_full_widgets: dirty_counts.full,
            dirty_regions: region_count,
            focus_rebuilt,
            layout_cache_hits,
            layout_cache_misses,
            layout_cache_mismatches,
            render_cache_hits,
            render_cache_misses,
            render_cache_mismatches,
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
        let new = self
            .focus_manager
            .focus_next()
            .context("focus_next failed")?;
        if old != new {
            if let Some(id) = old {
                self.dirty_tracker.mark_dirty(id);
            }
            if let Some(id) = new {
                self.dirty_tracker.mark_dirty(id);
            }
        }
        Ok(())
    }

    /// Move focus to the previous focusable widget (Shift+Tab).
    pub fn focus_prev(&mut self) -> anyhow::Result<()> {
        let old = self.focus_manager.current();
        let new = self
            .focus_manager
            .focus_prev()
            .context("focus_prev failed")?;
        if old != new {
            if let Some(id) = old {
                self.dirty_tracker.mark_dirty(id);
            }
            if let Some(id) = new {
                self.dirty_tracker.mark_dirty(id);
            }
        }
        Ok(())
    }

    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.focus_manager.current()
    }

    /// Dispatch a key event to the currently focused widget, with event bubbling.
    pub fn dispatch_action(&mut self, root: &mut WidgetNode, action: &WidgetAction) {
        let target = match self.focus_manager.current() {
            Some(id) => id,
            None => return,
        };

        let mut chain = vec![target];
        chain.extend(self.focus_manager.ancestor_chain(target));

        for widget_id in &chain {
            if let Some(widget) = find_widget_mut(root, *widget_id) {
                let result = widget.perform(action);
                if matches!(result, arbor_tui_domain::input::KeyHandleResult::Handled) {
                    let dirty_kind = widget.dirty_on_action(action);
                    self.dirty_tracker.mark_dirty_kind(*widget_id, dirty_kind);
                    if matches!(dirty_kind, DirtyKind::Structure | DirtyKind::Full) {
                        self.request_focus_rebuild();
                    }
                    return;
                }
            }
        }
    }

    pub fn run(&mut self) {
        self.running = true;
    }
}

impl App {
    pub(crate) fn enqueue_signal_change(&mut self, change: SignalChange) {
        if change.dirty.is_empty() {
            return;
        }
        self.pending_signal_generations
            .insert(change.signal_id, change.generation);
        for (widget_id, dirty_kind) in change.dirty {
            self.pending_signal_dirty
                .entry(widget_id)
                .and_modify(|existing| *existing = existing.merge(dirty_kind))
                .or_insert(dirty_kind);
            if matches!(dirty_kind, DirtyKind::Structure | DirtyKind::Full) {
                self.request_focus_rebuild();
            }
        }
    }

    fn drain_pending_signals_into_dirty_tracker(&mut self) {
        for (widget_id, dirty_kind) in std::mem::take(&mut self.pending_signal_dirty) {
            self.dirty_tracker.mark_dirty_kind(widget_id, dirty_kind);
        }
        self.pending_signal_generations.clear();
    }
}

fn collect_signal_deps(root: &WidgetNode) -> Vec<arbor_tui_domain::SignalDep> {
    let mut deps = Vec::new();
    collect_signal_deps_inner(root, &mut deps);
    deps
}

fn collect_signal_deps_inner(node: &WidgetNode, deps: &mut Vec<arbor_tui_domain::SignalDep>) {
    deps.extend(node.signal_deps());
    for child in node.children() {
        collect_signal_deps_inner(child, deps);
    }
}

fn observe_layout_shadow(
    shadow: &mut LayoutCacheShadow,
    root: &WidgetNode,
    available: Size,
    root_rect: Rect,
    constraints: &HashMap<WidgetId, SizeConstraint>,
    layout: &HashMap<WidgetId, arbor_tui_domain::WidgetLayoutInfo>,
) {
    observe_layout_node(shadow, root, available, root_rect, constraints, layout);
}

#[derive(Copy, Clone, Debug, Default)]
struct DirtyKindCounts {
    render: usize,
    layout: usize,
    structure: usize,
    theme: usize,
    full: usize,
}

fn count_dirty_kinds(kinds: impl IntoIterator<Item = DirtyKind>) -> DirtyKindCounts {
    let mut counts = DirtyKindCounts::default();
    for kind in kinds {
        match kind {
            DirtyKind::Render => counts.render += 1,
            DirtyKind::Layout => counts.layout += 1,
            DirtyKind::Structure => counts.structure += 1,
            DirtyKind::Theme => counts.theme += 1,
            DirtyKind::Full => counts.full += 1,
        }
    }
    counts
}

fn observe_layout_node(
    shadow: &mut LayoutCacheShadow,
    node: &WidgetNode,
    available: Size,
    parent_rect: Rect,
    constraints: &HashMap<WidgetId, SizeConstraint>,
    layout: &HashMap<WidgetId, arbor_tui_domain::WidgetLayoutInfo>,
) {
    let props_hash = hash_layout_props(node.layout_props());
    let children_measure_hash = hash_child_constraints(node, constraints);

    if let Some(output) = constraints.get(&node.id()).copied() {
        shadow.observe_measure(
            node.id(),
            MeasureCacheEntry {
                available,
                props_hash,
                children_measure_hash,
                output,
            },
        );
    }

    if let Some(output) = layout.get(&node.id()).cloned() {
        shadow.observe_layout(
            node.id(),
            LayoutCacheEntry {
                parent_rect,
                props_hash,
                children_measure_hash,
                output: output.clone(),
            },
        );
        let child_available = Size::new(output.content_rect.w, output.content_rect.h);
        for child in node.children() {
            observe_layout_node(
                shadow,
                child,
                child_available,
                output.content_rect,
                constraints,
                layout,
            );
        }
    }
}

fn hash_widget_identity(node: &WidgetNode) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    node.id().hash(&mut hasher);
    node.identity().hash(&mut hasher);
    hasher.finish()
}

fn hash_child_constraints(
    node: &WidgetNode,
    constraints: &HashMap<WidgetId, SizeConstraint>,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for child in node.children() {
        child.id().hash(&mut hasher);
        if let Some(constraint) = constraints.get(&child.id()) {
            hash_size_constraint(*constraint, &mut hasher);
        }
    }
    hasher.finish()
}

fn hash_layout_props(props: &LayoutProps) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hash_direction(props.direction, &mut hasher);
    hash_justify(props.justify, &mut hasher);
    hash_align(props.align, &mut hasher);
    props.flex.to_bits().hash(&mut hasher);
    props.width.hash(&mut hasher);
    props.height.hash(&mut hasher);
    props.padding.top.hash(&mut hasher);
    props.padding.right.hash(&mut hasher);
    props.padding.bottom.hash(&mut hasher);
    props.padding.left.hash(&mut hasher);
    props.margin.top.hash(&mut hasher);
    props.margin.right.hash(&mut hasher);
    props.margin.bottom.hash(&mut hasher);
    props.margin.left.hash(&mut hasher);
    hasher.finish()
}

fn hash_size_constraint(constraint: SizeConstraint, hasher: &mut impl Hasher) {
    constraint.min_w.hash(hasher);
    constraint.min_h.hash(hasher);
    hash_axis_constraint(constraint.max_w, hasher);
    hash_axis_constraint(constraint.max_h, hasher);
}

fn hash_axis_constraint(value: AxisConstraint, hasher: &mut impl Hasher) {
    match value {
        AxisConstraint::Fixed(value) => {
            0u8.hash(hasher);
            value.hash(hasher);
        }
        AxisConstraint::Unbounded => 1u8.hash(hasher),
    }
}

fn hash_direction(value: Direction, hasher: &mut impl Hasher) {
    match value {
        Direction::Row => 0u8.hash(hasher),
        Direction::Column => 1u8.hash(hasher),
    }
}

fn hash_justify(value: Justify, hasher: &mut impl Hasher) {
    match value {
        Justify::Start => 0u8.hash(hasher),
        Justify::Center => 1u8.hash(hasher),
        Justify::End => 2u8.hash(hasher),
        Justify::SpaceBetween => 3u8.hash(hasher),
    }
}

fn hash_align(value: Align, hasher: &mut impl Hasher) {
    match value {
        Align::Start => 0u8.hash(hasher),
        Align::Center => 1u8.hash(hasher),
        Align::End => 2u8.hash(hasher),
        Align::Stretch => 3u8.hash(hasher),
    }
}

fn theme_rev(theme: &Theme) -> u64 {
    match theme.variant {
        ThemeVariant::Dark => 1,
        ThemeVariant::Light => 2,
        ThemeVariant::HighContrast => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbor_tui_adapters::simulated_backend::SimulatedBackend;
    use arbor_tui_domain::signal::Signal;
    use arbor_tui_widgets::input::Input;
    use arbor_tui_widgets::tabs::{TabDef, Tabs};
    use arbor_tui_widgets::text::Text;
    use arbor_tui_widgets::widget_factory::WidgetFactory;

    #[test]
    fn update_signal_marks_subscribers_dirty_without_exposing_dirty_tracker() {
        let signal = Signal::new("before".to_string());
        signal.subscribe(WidgetId(1));
        let mut app = App::new(20, 1);

        app.update_signal(&signal, "after".to_string());

        assert_eq!(signal.get(), "after");
        assert!(app.has_pending_render());
    }

    #[test]
    fn multiple_signal_writes_before_render_merge_dirty_once() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let signal = Signal::new("before".to_string());
        let mut root = Text::new("").content_from(&signal).build(&factory, &theme);
        arbor_tui_domain::focus::mount_tree(&mut root);
        let mut backend = SimulatedBackend::new(20, 1);
        let mut app = App::new(20, 1);

        app.update_signal(&signal, "middle".to_string());
        app.update_signal(&signal, "after".to_string());

        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("render should drain pending signal changes");

        assert_eq!(signal.get(), "after");
        assert_eq!(app.last_frame_stats().dirty_widgets, 1);
        assert_eq!(app.last_frame_stats().dirty_layout_widgets, 1);
        assert_eq!(
            app.last_frame_snapshot().generation(signal.id()),
            Some(signal.generation())
        );
    }

    #[test]
    fn signal_write_after_snapshot_waits_for_next_frame_snapshot() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let signal = Signal::new("before".to_string());
        let mut root = Text::new("").content_from(&signal).build(&factory, &theme);
        arbor_tui_domain::focus::mount_tree(&mut root);
        let mut backend = SimulatedBackend::new(20, 1);
        let mut app = App::new(20, 1);

        app.request_render();
        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("initial render should create snapshot");
        let first_snapshot_generation = app
            .last_frame_snapshot()
            .generation(signal.id())
            .expect("snapshot should include text signal dep");

        app.update_signal(&signal, "after".to_string());

        assert_eq!(
            app.last_frame_snapshot().generation(signal.id()),
            Some(first_snapshot_generation),
            "pending writes must not mutate an existing frame snapshot"
        );

        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("next render should create a new snapshot");
        assert_eq!(
            app.last_frame_snapshot().generation(signal.id()),
            Some(signal.generation())
        );
    }

    #[test]
    fn focus_index_rebuilds_only_when_marked_dirty() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let root = Text::new("hello").build(&factory, &theme);
        let mut backend = SimulatedBackend::new(20, 3);
        let mut app = App::new(20, 3);

        app.request_render();
        assert!(app.focus_dirty);
        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("first render should succeed");
        assert!(!app.focus_dirty);
        assert!(app.last_frame_stats().focus_rebuilt);

        app.request_render();
        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("unchanged render should succeed");
        assert!(!app.focus_dirty);

        app.request_focus_rebuild();
        assert!(app.focus_dirty);
        app.request_render();
        app.render_widget_tree(&root, &theme, &mut backend)
            .expect("explicit focus rebuild render should succeed");
        assert!(!app.focus_dirty);
    }

    #[test]
    fn render_only_widget_action_does_not_rebuild_focus() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let mut root = Input::new().build(&factory, &theme);
        let mut app = App::new(20, 1);

        app.run();
        app.rebuild_focus(&root);
        app.focus_next().unwrap();
        app.take_dirty_widgets();
        assert!(!app.focus_dirty);

        app.dispatch_action(&mut root, &WidgetAction::TypeChar('x'));

        assert!(app.has_pending_render());
        assert!(!app.focus_dirty);
    }

    #[test]
    fn structure_widget_action_requests_focus_rebuild() {
        let theme = Theme::dark();
        let factory = WidgetFactory::new();
        let first = Text::new("first").build(&factory, &theme);
        let second = Text::new("second").build(&factory, &theme);
        let mut root = Tabs::new(0)
            .tabs(vec![
                TabDef {
                    label: "First".to_string(),
                    content: first,
                },
                TabDef {
                    label: "Second".to_string(),
                    content: second,
                },
            ])
            .build(&factory, &theme);
        let mut app = App::new(20, 3);

        app.run();
        app.rebuild_focus(&root);
        app.focus_next().unwrap();
        app.take_dirty_widgets();
        assert!(!app.focus_dirty);

        app.dispatch_action(&mut root, &WidgetAction::NavigateRight);

        assert!(app.has_pending_render());
        assert!(app.focus_dirty);
    }
}
