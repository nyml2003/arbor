// Layout pattern tests: Col, Row, nested flex layouts.

use arbor_tui_domain::theme::Theme;
use arbor_tui_testing::WidgetHarness;
use arbor_tui_widgets::border::Border;
use arbor_tui_widgets::stack::{Col, Row};
use arbor_tui_widgets::text::Text;
use arbor_tui_widgets::widget_factory::WidgetFactory;

fn wm_and_theme() -> (WidgetFactory, Theme) {
    (WidgetFactory::new(), Theme::dark())
}

// ── Col ───────────────────────────────────────────────────────────

#[test]
fn col_stacks_children_vertically() {
    let (wm, t) = wm_and_theme();
    let root = Col::new()
        .children([
            Text::new("top").build(&wm, &t),
            Text::new("middle").build(&wm, &t),
            Text::new("bottom").build(&wm, &t),
        ])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    let top = h.find_text("top")[0];
    let mid = h.find_text("middle")[0];
    let bot = h.find_text("bottom")[0];
    assert!(mid.1 > top.1, "middle below top");
    assert!(bot.1 > mid.1, "bottom below middle");
}

#[test]
fn col_padding_offsets_children() {
    let (wm, t) = wm_and_theme();
    use arbor_tui_domain::layout::RectOffset;
    let root = Col::new()
        .padding(RectOffset {
            top: 2,
            bottom: 0,
            left: 4,
            right: 0,
        })
        .children([Text::new("padded").build(&wm, &t)])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    let pos = h.find_text("padded")[0];
    assert!(pos.0 >= 4, "should be indented by left padding");
}

// ── Row ───────────────────────────────────────────────────────────

#[test]
fn row_aligns_children_horizontally() {
    let (wm, t) = wm_and_theme();
    let root = Row::new()
        .children([
            Text::new("left").build(&wm, &t),
            Text::new("right").build(&wm, &t),
        ])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 60, 5, &t);
    let l = h.find_text("left")[0];
    let r = h.find_text("right")[0];
    assert!(r.0 > l.0, "right should be to the right of left");
}

#[test]
fn row_with_flex() {
    let (wm, t) = wm_and_theme();
    let root = Row::new()
        .children([
            Border::new()
                .title(" Fix ")
                .child(Text::new("fixed content").build(&wm, &t))
                .build(&wm, &t),
            Border::new()
                .flex(1.0)
                .title(" Flex ")
                .child(Text::new("stretch").build(&wm, &t))
                .build(&wm, &t),
        ])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 80, 5, &t);
    assert!(!h.find_text("Fix").is_empty());
    assert!(!h.find_text("Flex").is_empty());
    assert!(!h.find_text("fixed content").is_empty());
    assert!(!h.find_text("stretch").is_empty());
}

// ── Nested layouts ────────────────────────────────────────────────

#[test]
fn header_body_footer_layout() {
    let (wm, t) = wm_and_theme();
    let header = Border::new()
        .title(" Header ")
        .child(Text::new("header text").build(&wm, &t))
        .build(&wm, &t);

    let body = Row::new()
        .flex(1.0)
        .children([
            Border::new()
                .title(" Left ")
                .child(Text::new("left").build(&wm, &t))
                .build(&wm, &t),
            Border::new()
                .flex(1.0)
                .title(" Center ")
                .child(Text::new("center").build(&wm, &t))
                .build(&wm, &t),
        ])
        .build(&wm, &t);

    let footer = Border::new()
        .title(" Footer ")
        .child(Text::new("status").build(&wm, &t))
        .build(&wm, &t);

    let root = Col::new().children([header, body, footer]).build(&wm, &t);

    let h = WidgetHarness::render(&root, 80, 24, &t);
    assert!(!h.find_text("header text").is_empty());
    assert!(!h.find_text("left").is_empty());
    assert!(!h.find_text("center").is_empty());
    assert!(!h.find_text("status").is_empty());
}

#[test]
fn deeply_nested_light_theme_no_black_bg() {
    let (wm, _) = wm_and_theme();
    let t = Theme::light();

    let leaf = Text::new("deep").fg(t.text()).build(&wm, &t);
    let inner_border = Border::new().title(" Inner ").child(leaf).build(&wm, &t);
    let middle_col = Col::new()
        .children([Text::new("above").fg(t.text()).build(&wm, &t), inner_border])
        .build(&wm, &t);
    let outer = Border::new()
        .title(" Outer ")
        .child(middle_col)
        .build(&wm, &t);

    let h = WidgetHarness::render(&outer, 80, 20, &t);
    h.assert_no_black_bg_on_text().unwrap();
}

#[test]
fn three_column_flex_layout() {
    let (wm, t) = wm_and_theme();
    let cols: Vec<_> = (0..3)
        .map(|i| {
            Border::new()
                .flex(1.0)
                .title(format!(" Col{i} "))
                .child(Text::new(format!("column {i}")).build(&wm, &t))
                .build(&wm, &t)
        })
        .collect();

    let root = Row::new().children(cols).build(&wm, &t);
    let h = WidgetHarness::render(&root, 90, 8, &t);

    for i in 0..3 {
        assert!(
            !h.find_text(&format!("column {i}")).is_empty(),
            "column {i} should be visible"
        );
    }
}

// ── Edge cases ────────────────────────────────────────────────────

#[test]
fn empty_stack() {
    let (wm, t) = wm_and_theme();
    let root = Col::new().build(&wm, &t);
    let h = WidgetHarness::render(&root, 80, 24, &t);
    // Should not panic
    assert!(h.cols() == 80);
}

#[test]
fn single_child_fills_space() {
    let (wm, t) = wm_and_theme();
    let root = Col::new()
        .children([Text::new("solo").build(&wm, &t)])
        .build(&wm, &t);
    let h = WidgetHarness::render(&root, 40, 10, &t);
    assert!(!h.find_text("solo").is_empty());
}
