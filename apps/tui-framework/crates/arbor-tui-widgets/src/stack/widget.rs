use std::collections::HashMap;

use arbor_tui_domain::layout::{
    Direction, LayoutProps, RectOffset, Size, SizeCalc, SizeConstraint,
};
use arbor_tui_domain::widget::{Widget, WidgetId, WidgetNode};

pub struct StackWidget {
    pub id: WidgetId,
    pub props: LayoutProps,
    pub children: Vec<WidgetNode>,
}

impl Widget for StackWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout_props(&self) -> &LayoutProps {
        &self.props
    }

    fn children(&self) -> &[WidgetNode] {
        &self.children
    }

    fn children_mut(&mut self) -> &mut [WidgetNode] {
        &mut self.children
    }

    fn is_transparent(&self) -> bool {
        true
    }

    fn measure_subtree(
        &self,
        available: Size,
        child_constraints: &HashMap<WidgetId, SizeConstraint>,
    ) -> SizeConstraint {
        if self.children.is_empty() {
            return SizeConstraint::fixed(
                self.props.width.unwrap_or(0),
                self.props.height.unwrap_or(0),
            );
        }

        let _ = SizeCalc::content_available(available, self.props.padding, RectOffset::default());
        let mut total_main: u16 = 0;
        let mut max_cross: u16 = 0;

        for child in &self.children {
            let cc = child_constraints
                .get(&child.id())
                .copied()
                .unwrap_or(SizeConstraint::unbounded());
            let child_props = child.layout_props();
            match self.props.direction {
                Direction::Column => {
                    total_main +=
                        cc.min_h + child_props.margin.vertical() + child_props.padding.vertical();
                    max_cross = max_cross.max(
                        cc.min_w
                            + child_props.margin.horizontal()
                            + child_props.padding.horizontal(),
                    );
                }
                Direction::Row => {
                    total_main += cc.min_w
                        + child_props.margin.horizontal()
                        + child_props.padding.horizontal();
                    max_cross = max_cross.max(
                        cc.min_h + child_props.margin.vertical() + child_props.padding.vertical(),
                    );
                }
            }
        }

        let (min_w, min_h) = match self.props.direction {
            Direction::Column => (max_cross, total_main),
            Direction::Row => (total_main, max_cross),
        };

        let outer = SizeCalc::outer_size(
            Size::new(min_w, min_h),
            self.props.padding,
            RectOffset::default(),
        );

        SizeConstraint {
            min_w: self.props.width.unwrap_or(outer.w).max(1),
            min_h: self.props.height.unwrap_or(outer.h).max(1),
            max_w: arbor_tui_domain::layout::AxisConstraint::Fixed(available.w),
            max_h: arbor_tui_domain::layout::AxisConstraint::Fixed(available.h),
        }
    }
}
