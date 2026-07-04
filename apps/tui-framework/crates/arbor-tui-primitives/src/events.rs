// Framework event bus — emit/subscribe pattern for debugging, profiling, and testing.
// Compiles only when the "profile" feature is enabled.

use crate::input::KeyEvent;
use crate::widget_id::WidgetId;

/// Events emitted at key points in the framework lifecycle.
#[derive(Clone, Debug)]
pub enum FrameworkEvent {
    // ── Render pipeline ──
    FrameStart { seq: u64 },
    LayoutStart { widget_count: usize },
    LayoutEnd { duration_us: u64 },
    DiffStart { screen_size: (u16, u16) },
    DiffEnd { duration_us: u64, dirty_regions: usize, dirty_cells: usize },
    EmitStart { region_count: usize },
    EmitEnd { duration_us: u64 },
    FrameEnd(FrameStats),

    // ── Input ──
    InputReceived(KeyEvent),
    InputMerged { before: usize, after: usize },
    FocusChanged { from: Option<WidgetId>, to: Option<WidgetId> },

    // ── Signal ──
    SignalSet { widget_id: WidgetId, generation: u64 },

    // ── Lifecycle ──
    WidgetMounted(WidgetId),
    WidgetUnmounted(WidgetId),
    AppStart,
    AppQuit,

    // ── Errors / warnings ──
    Warning { widget_id: Option<WidgetId>, message: String },
    Error { message: String },
}

/// Per-frame performance statistics.
#[derive(Clone, Debug, Default)]
pub struct FrameStats {
    pub seq: u64,
    pub layout_us: u64,
    pub render_us: u64,
    pub diff_us: u64,
    pub emit_us: u64,
    pub total_us: u64,
    pub dirty_widgets: usize,
    pub dirty_cells: usize,
}

/// Subscribe to framework events for profiling, debugging, or testing.
pub trait EventSubscriber: Send + Sync {
    fn on_event(&self, event: &FrameworkEvent);
}

/// Event bus — holds subscribers and dispatches events.
///
/// When the `profile` feature is disabled, this type is not compiled
/// and all emit sites are optimized to no-ops.
pub struct EventBus {
    subscribers: Vec<Box<dyn EventSubscriber>>,
    enabled: bool,
}

impl EventBus {
    pub fn new() -> Self {
        Self {
            subscribers: Vec::new(),
            enabled: true,
        }
    }

    /// Control whether events are dispatched. Set to `false` in production.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a subscriber.
    pub fn subscribe(&mut self, sub: Box<dyn EventSubscriber>) {
        self.subscribers.push(sub);
    }

    /// Emit an event to all subscribers.
    #[inline]
    pub fn emit(&self, event: FrameworkEvent) {
        if !self.enabled {
            return;
        }
        for sub in &self.subscribers {
            sub.on_event(&event);
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
