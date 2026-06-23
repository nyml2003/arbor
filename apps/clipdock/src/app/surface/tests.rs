use arbor_ui_core::event::PointerEvent;
use arbor_ui_core::geometry::{Point, Size};
use arbor_ui_core::view::components::ComponentNode;

use super::*;
use crate::app::model::AppCommand;

#[test]
fn records_clipboard_text_and_updates_layout() {
    let mut surface = ClipDockSurface::new().unwrap();

    assert!(surface.record_clipboard_text("alpha").unwrap());

    assert_eq!(surface.history.items()[0].text, "alpha");
    assert_eq!(surface.snapshot.items.len(), 1);
}

#[test]
fn item_click_emits_paste_command() {
    let mut surface = ClipDockSurface::new().unwrap();
    surface.record_clipboard_text("alpha").unwrap();
    let rect = surface.snapshot.items[0].rect;
    let point = Point::new(rect.x + 4.0, rect.y + 4.0);

    surface
        .handle_pointer_event(PointerEvent::Down(point))
        .unwrap();
    let update = surface
        .handle_pointer_event(PointerEvent::Up(point))
        .unwrap();

    assert_eq!(
        update.commands,
        vec![AppCommand::PasteText("alpha".to_string())]
    );
    assert!(update.needs_render);
}

#[test]
fn close_click_emits_close_command() {
    let mut surface = ClipDockSurface::new().unwrap();
    let rect = surface.snapshot.close_rect;
    let point = Point::new(rect.x + 4.0, rect.y + 4.0);

    surface
        .handle_pointer_event(PointerEvent::Down(point))
        .unwrap();
    let update = surface
        .handle_pointer_event(PointerEvent::Up(point))
        .unwrap();

    assert_eq!(update.commands, vec![AppCommand::CloseApp]);
}

#[test]
fn title_hit_test_returns_drag_outside_close_button() {
    let surface = ClipDockSurface::new().unwrap();
    let rect = surface.drag_title_rect();

    assert_eq!(
        surface.chrome_hit_test(Point::new(rect.x + 4.0, rect.y + 4.0)),
        ChromeHit::Drag
    );
}

#[test]
fn resize_recomputes_surface() {
    let mut surface = ClipDockSurface::new().unwrap();

    surface.resize(Size::new(640.0, 360.0)).unwrap();

    assert_eq!(surface.snapshot.surface_rect.width, 640.0);
    assert_eq!(surface.snapshot.surface_rect.height, 360.0);
}

#[test]
fn snapshot_root_is_surface() {
    let surface = ClipDockSurface::new().unwrap();
    let snapshot = surface.snapshot();

    assert_eq!(snapshot.primitive_tree.id(), "clipdock-surface");
}
