// arbor-tui-testing — test drivers and screen assertions.
// Tests use simulated input/output and do not depend on a real terminal.

pub mod e2e;
pub mod widget;

pub use e2e::{find_text_in_screen, TuiTestDriver};
pub use widget::WidgetHarness;
