use arbor_tui_domain::theme::Theme;
use arbor_tui_domain::widget::WidgetNode;
use arbor_tui_testing::TuiTestDriver;

pub fn mounted(root: WidgetNode, cols: u16, rows: u16, theme: Theme) -> TuiTestDriver {
    let mut driver = TuiTestDriver::new(root, cols, rows, theme);
    driver.render_initial().unwrap();
    driver
}

#[allow(dead_code)]
pub fn assert_has_text(driver: &TuiTestDriver, text: &str) {
    assert!(
        !driver.find_text(text).is_empty(),
        "expected screen to contain {text:?}\n{}",
        driver.visible_text()
    );
}

#[allow(dead_code)]
pub fn assert_not_text(driver: &TuiTestDriver, text: &str) {
    assert!(
        driver.find_text(text).is_empty(),
        "expected screen not to contain {text:?}\n{}",
        driver.visible_text()
    );
}

#[allow(dead_code)]
pub fn numbered(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|i| format!("{prefix} {i:02}")).collect()
}
