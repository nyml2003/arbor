// testing — lightweight test harness for widget rendering.
//
// Renders a widget tree via measure → layout → render and returns the
// resulting VirtualScreen for assertion. No real terminal needed.

use arbor_tui_primitives::cell::Cell;
use arbor_tui_primitives::layout::{Rect, Size};
use arbor_tui_primitives::widget_id::WidgetId;
use arbor_tui_render::screen::VirtualScreen;
use arbor_tui_render::theme::Theme;
use arbor_tui_widget::layout_engine::{layout_tree, measure_tree};
use arbor_tui_widget::render::render_tree;
use arbor_tui_widget::widget::WidgetNode;

/// A test harness that renders a widget tree into a [`VirtualScreen`].
///
/// # Example
///
/// ```ignore
/// let root = Border::new()
///     .title(" Test ")
///     .child(Text::new("hello").build(&wm, &theme))
///     .build(&wm, &theme);
///
/// let harness = WidgetHarness::render(&root, 80, 24, &Theme::light());
/// assert!(harness.find_text("hello").len() > 0);
/// ```
pub struct WidgetHarness {
    screen: VirtualScreen,
}

impl WidgetHarness {
    /// Render a widget tree at the given terminal size with the given theme.
    /// No widget is marked as focused.
    pub fn render(root: &WidgetNode, cols: u16, rows: u16, theme: &Theme) -> Self {
        Self::render_with_focus(root, cols, rows, theme, None)
    }

    /// Render a widget tree with an optional focused widget.
    pub fn render_with_focus(
        root: &WidgetNode,
        cols: u16,
        rows: u16,
        theme: &Theme,
        focused: Option<WidgetId>,
    ) -> Self {
        let size = Size { w: cols, h: rows };
        let constraints = measure_tree(root, size);
        let layout = layout_tree(Rect::new(0, 0, cols, rows), root, &constraints)
            .expect("layout must succeed");
        let screen = render_tree((cols, rows), root, &layout, theme, focused);
        Self { screen }
    }

    /// Read a single cell. Returns [`Cell::default()`] for out-of-bounds.
    pub fn cell_at(&self, col: u16, row: u16) -> Cell {
        self.screen.cell_at(col, row)
    }

    /// Reference to the underlying screen.
    pub fn screen(&self) -> &VirtualScreen {
        &self.screen
    }

    /// Screen dimensions.
    pub fn cols(&self) -> u16 { self.screen.cols() }
    pub fn rows(&self) -> u16 { self.screen.rows() }

    // ── Inspection helpers ──────────────────────────────────────────

    /// Search for text across the entire screen. Returns all (col, row)
    /// positions where `needle` starts.
    ///
    /// This does a row-by-row scan, comparing character sequences. It is
    /// NOT a substring search that crosses cell boundaries — it looks for
    /// contiguous runs of chars that form the needle.
    pub fn find_text(&self, needle: &str) -> Vec<(u16, u16)> {
        let mut positions = Vec::new();
        let needle_chars: Vec<char> = needle.chars().collect();
        if needle_chars.is_empty() {
            return positions;
        }
        for row in 0..self.rows() {
            let mut col = 0u16;
            while col < self.cols() {
                // Try to match needle starting at (col, row)
                let mut matched = true;
                for (i, &ch) in needle_chars.iter().enumerate() {
                    let c = col + i as u16;
                    if c >= self.cols() || self.cell_at(c, row).ch != ch {
                        matched = false;
                        break;
                    }
                }
                if matched {
                    positions.push((col, row));
                    col += needle_chars.len() as u16;
                } else {
                    col += 1;
                }
            }
        }
        positions
    }

    /// Count cells whose background matches `bg`.
    pub fn count_bg(&self, bg: impl Into<arbor_tui_primitives::cell::AnsiColor>) -> usize {
        let target = bg.into();
        let mut count = 0;
        for (_, _, cell) in self.screen.iter_cells() {
            if cell.bg == target {
                count += 1;
            }
        }
        count
    }

    /// Assert that no cell with a visible character has the default black
    /// background (palette 0). This catches the common bug where widgets
    /// forget to fill their screen before rendering, leaking black cells
    /// in light theme.
    ///
    /// Returns `Ok(())` if the assertion passes, or `Err` with a list of
    /// offending positions.
    pub fn assert_no_black_bg_on_text(&self) -> Result<(), Vec<(u16, u16, char)>> {
        let black = arbor_tui_primitives::cell::PaletteColor(0);
        let mut offenders = Vec::new();
        for (col, row, cell) in self.screen.iter_cells() {
            // Only flag cells that have visible content (not spaces)
            if cell.ch != ' ' && cell.bg.palette == black {
                offenders.push((col, row, cell.ch));
            }
        }
        if offenders.is_empty() {
            Ok(())
        } else {
            Err(offenders)
        }
    }
}
