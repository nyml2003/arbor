// Layout types — pure data structures for the flexbox layout engine.

/// A rectangle on the character grid. Origin (0,0) is top-left.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Self { x, y, w, h }
    }
}

/// Width and height in columns and rows.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size {
    pub w: u16,
    pub h: u16,
}

impl Size {
    pub const fn new(w: u16, h: u16) -> Self {
        Self { w, h }
    }
}

/// Offset for padding and margin on all four sides.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct RectOffset {
    pub top: u16,
    pub right: u16,
    pub bottom: u16,
    pub left: u16,
}

impl RectOffset {
    pub const fn all(v: u16) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    pub const fn horizontal(&self) -> u16 {
        self.left + self.right
    }

    pub const fn vertical(&self) -> u16 {
        self.top + self.bottom
    }
}

/// A single-axis constraint.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AxisConstraint {
    Fixed(u16),
    Unbounded,
}

/// Size constraints reported by `Widget::measure()`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct SizeConstraint {
    pub min_w: u16,
    pub min_h: u16,
    pub max_w: AxisConstraint,
    pub max_h: AxisConstraint,
}

impl SizeConstraint {
    /// Fully bounded — parent has assigned a concrete available size.
    pub fn bounded(available: Size) -> Self {
        Self {
            min_w: 0,
            min_h: 0,
            max_w: AxisConstraint::Fixed(available.w),
            max_h: AxisConstraint::Fixed(available.h),
        }
    }

    /// Unbounded — leaf widget measuring its intrinsic size.
    pub fn unbounded() -> Self {
        Self {
            min_w: 0,
            min_h: 0,
            max_w: AxisConstraint::Unbounded,
            max_h: AxisConstraint::Unbounded,
        }
    }

    /// Fixed exact size.
    pub fn fixed(w: u16, h: u16) -> Self {
        Self {
            min_w: w,
            min_h: h,
            max_w: AxisConstraint::Fixed(w),
            max_h: AxisConstraint::Fixed(h),
        }
    }

    /// At least 1 column and 1 row — widgets must never have zero size.
    pub fn at_least_one() -> Self {
        Self {
            min_w: 1,
            min_h: 1,
            max_w: AxisConstraint::Unbounded,
            max_h: AxisConstraint::Unbounded,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Layout properties for a widget.
#[derive(Clone, Debug)]
pub struct LayoutProps {
    pub direction: Direction,
    pub justify: Justify,
    pub align: Align,
    pub flex: f32,
    pub width: Option<u16>,
    pub height: Option<u16>,
    pub padding: RectOffset,
    pub margin: RectOffset,
}

impl Default for LayoutProps {
    fn default() -> Self {
        Self {
            direction: Direction::Column,
            justify: Justify::Start,
            align: Align::Stretch,
            flex: 0.0,
            width: None,
            height: None,
            padding: RectOffset::default(),
            margin: RectOffset::default(),
        }
    }
}

/// Unified size calculation utility. All layout arithmetic must go through
/// SizeCalc — direct `w - padding.left - padding.right` is forbidden.
pub struct SizeCalc;

impl SizeCalc {
    /// Content-available size after subtracting padding and margin from container.
    pub fn content_available(container: Size, padding: RectOffset, margin: RectOffset) -> Size {
        Size {
            w: sat_sub(container.w, padding.horizontal() + margin.horizontal()),
            h: sat_sub(container.h, padding.vertical() + margin.vertical()),
        }
    }

    /// Outer size of a widget: content + padding + margin.
    pub fn outer_size(content: Size, padding: RectOffset, margin: RectOffset) -> Size {
        Size {
            w: content.w + padding.horizontal() + margin.horizontal(),
            h: content.h + padding.vertical() + margin.vertical(),
        }
    }

    /// Inner content rect: outer_rect minus padding.
    pub fn content_rect(outer: Rect, padding: RectOffset) -> Rect {
        Rect {
            x: outer.x + padding.left,
            y: outer.y + padding.top,
            w: sat_sub(outer.w, padding.horizontal()),
            h: sat_sub(outer.h, padding.vertical()),
        }
    }
}

/// Saturated subtraction — returns 0 instead of underflowing.
#[inline]
pub fn sat_sub(a: u16, b: u16) -> u16 {
    a.saturating_sub(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sat_sub_normal() {
        assert_eq!(sat_sub(10, 3), 7);
    }

    #[test]
    fn sat_sub_underflow() {
        assert_eq!(sat_sub(3, 10), 0);
    }

    #[test]
    fn content_available_reduces_correctly() {
        let container = Size::new(80, 24);
        let padding = RectOffset::all(1);
        let margin = RectOffset::all(0);
        let avail = SizeCalc::content_available(container, padding, margin);
        assert_eq!(avail.w, 78);
        assert_eq!(avail.h, 22);
    }

    #[test]
    fn outer_size_adds_correctly() {
        let content = Size::new(10, 5);
        let padding = RectOffset::all(2);
        let margin = RectOffset::all(1);
        let outer = SizeCalc::outer_size(content, padding, margin);
        assert_eq!(outer.w, 16); // 10 + 4 + 2
        assert_eq!(outer.h, 11);
    }
}
