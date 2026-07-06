// arbor-tui-domain — pure TUI domain model.
// Contains terminal cells, layout, rendering model, widget protocol, input,
// reactive state, and backend ports. No concrete terminal adapter lives here.

pub mod backend;
pub mod cache;
pub mod cell;
pub mod component;
pub mod computed;
pub mod diff;
pub mod dirty;
pub mod focus;
pub mod frame;
pub mod identity;
pub mod input;
pub mod layout;
pub mod layout_engine;
pub mod layout_error;
pub mod memo;
pub mod reconcile;
pub mod render;
pub mod screen;
pub mod signal;
pub mod text;
pub mod theme;
pub mod widget;
pub mod widget_id;

#[cfg(feature = "profile")]
pub mod events;

pub use component::{ComponentProps, PropsRevision, PropsRevisionBuilder};
pub use computed::{ComputedRead, ComputedSignal};
pub use frame::FrameSnapshot;
pub use identity::{DirtyKind, IdentityError, NodeIdentity, ReconcileReport, WidgetKey};
pub use layout_error::LayoutError;
pub use memo::{MemoRetention, MemoSlot, MemoStats, MemoStatus, MemoStore};
pub use signal::{SignalChange, SignalDep, SignalId, SignalSource};
pub use widget_id::{WidgetAction, WidgetId, WidgetLayoutInfo};
