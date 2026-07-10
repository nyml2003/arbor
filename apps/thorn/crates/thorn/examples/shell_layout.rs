use std::io;

use thorn::prelude::{
    crossterm_terminal_size, read_crossterm_runtime_input, BackendPresenter, CrosstermPresenter,
    CrosstermSession, Key, KeyModifiers, PaintAttrs, PaintColor, PaintPrimitive, PaintStyle, Rect,
    RuntimeInput, Screen, Size,
};

fn main() -> io::Result<()> {
    let _session = CrosstermSession::enter()?;
    let mut size = crossterm_terminal_size().unwrap_or(Size::new(120, 36));
    let mut presenter = CrosstermPresenter::new(io::stdout());
    let mut previous_screen: Option<Screen> = None;

    loop {
        let mut screen = Screen::new(size);
        screen.apply(&shell_primitives(size));
        let patch = previous_screen
            .as_ref()
            .map_or_else(|| screen.full_patch(), |previous| previous.diff(&screen));
        if patch.full || !patch.cells.is_empty() {
            presenter
                .present(&patch)
                .map_err(|err| io::Error::other(format!("{err:?}")))?;
        }
        previous_screen = Some(screen);

        match read_crossterm_runtime_input()? {
            Some(RuntimeInput::Key(event))
                if event.key == Key::Char('q')
                    || event.key == Key::Char('c')
                        && event.modifiers.contains(KeyModifiers::CTRL) =>
            {
                break;
            }
            Some(RuntimeInput::Resize(next_size)) => {
                size = next_size;
            }
            Some(RuntimeInput::Shutdown) => break,
            Some(RuntimeInput::Key(_))
            | Some(RuntimeInput::Tick)
            | Some(RuntimeInput::BackendWake)
            | None => {}
        }
    }

    Ok(())
}

fn shell_primitives(size: Size) -> Vec<PaintPrimitive> {
    let mut paint = vec![fill(
        Rect::new(0, 0, size.width, size.height),
        bg(PaintColor::Rgb(10, 12, 15)),
    )];

    if size.width < 30 || size.height < 10 {
        text_fit(
            &mut paint,
            size,
            1,
            1,
            size.width.saturating_sub(2),
            "Thorn Shell",
            fg_bg(
                PaintColor::Rgb(238, 242, 248),
                PaintColor::Rgb(10, 12, 15),
                PaintAttrs::BOLD,
            ),
        );
        text_fit(
            &mut paint,
            size,
            1,
            2,
            size.width.saturating_sub(2),
            "q exits",
            muted_on_canvas(),
        );
        return paint;
    }

    let nav_height = 3;
    let foot_height = 2;
    navbar(&mut paint, size, Rect::new(0, 0, size.width, nav_height));
    footbar(
        &mut paint,
        size,
        Rect::new(
            0,
            size.height.saturating_sub(foot_height),
            size.width,
            foot_height,
        ),
    );

    let pad = if size.width >= 90 { 2 } else { 1 };
    let body = Rect::new(
        pad,
        nav_height,
        size.width.saturating_sub(pad * 2),
        size.height.saturating_sub(nav_height + foot_height),
    );
    if body.width >= 72 {
        horizontal_shell(&mut paint, size, body);
    } else {
        stacked_shell(&mut paint, size, body);
    }

    paint
}

fn horizontal_shell(paint: &mut Vec<PaintPrimitive>, size: Size, body: Rect) {
    let gap = if body.width >= 100 { 2 } else { 1 };
    let left_width = if body.width >= 100 { 22 } else { 18 };
    let right_width = if body.width >= 100 { 24 } else { 18 };
    let content_width = body
        .width
        .saturating_sub(left_width)
        .saturating_sub(right_width)
        .saturating_sub(gap * 2)
        .max(1);

    let left = Rect::new(
        body.x,
        body.y + 1,
        left_width,
        body.height.saturating_sub(2),
    );
    let content = Rect::new(
        left.x + left.width + gap,
        body.y + 1,
        content_width,
        body.height.saturating_sub(2),
    );
    let right = Rect::new(
        content.x + content.width + gap,
        body.y + 1,
        right_width,
        body.height.saturating_sub(2),
    );

    left_navbar(paint, size, left);
    content_panel(paint, size, content);
    right_navbar(paint, size, right);
}

fn stacked_shell(paint: &mut Vec<PaintPrimitive>, size: Size, body: Rect) {
    let gap = 1;
    let left_height = 5.min(body.height.saturating_sub(gap * 2));
    let right_height = 5.min(body.height.saturating_sub(left_height + gap * 2));
    let content_height = body
        .height
        .saturating_sub(left_height)
        .saturating_sub(right_height)
        .saturating_sub(gap * 2)
        .max(1);

    let left = Rect::new(body.x, body.y + 1, body.width, left_height);
    let content = Rect::new(
        body.x,
        left.y + left.height + gap,
        body.width,
        content_height,
    );
    let right = Rect::new(
        body.x,
        content.y + content.height + gap,
        body.width,
        right_height,
    );

    left_navbar(paint, size, left);
    content_panel(paint, size, content);
    right_navbar(paint, size, right);
}

fn navbar(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    paint.push(fill(rect, bg(PaintColor::Rgb(17, 24, 35))));
    paint.push(fill(
        Rect::new(
            rect.x,
            rect.y + rect.height.saturating_sub(1),
            rect.width,
            1,
        ),
        bg(PaintColor::Rgb(45, 62, 82)),
    ));
    text_fit(
        paint,
        size,
        2,
        1,
        18,
        "THORN SHELL",
        fg_bg(
            PaintColor::Rgb(240, 244, 250),
            PaintColor::Rgb(17, 24, 35),
            PaintAttrs::BOLD,
        ),
    );
    let tabs_x = if size.width >= 80 {
        size.width.saturating_sub(41)
    } else {
        18
    };
    text_fit(
        paint,
        size,
        tabs_x,
        1,
        size.width.saturating_sub(tabs_x + 2),
        "Overview  Sessions  Settings",
        fg_bg(
            PaintColor::Rgb(165, 178, 196),
            PaintColor::Rgb(17, 24, 35),
            PaintAttrs::empty(),
        ),
    );
}

fn footbar(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    paint.push(fill(rect, bg(PaintColor::Rgb(13, 18, 26))));
    paint.push(fill(
        Rect::new(rect.x, rect.y, rect.width, 1),
        bg(PaintColor::Rgb(45, 62, 82)),
    ));
    text_fit(
        paint,
        size,
        2,
        rect.y + 1,
        size.width.saturating_sub(4),
        "q exits | Ctrl-C exits | resize keeps navbar/content/rightbar inside viewport",
        muted_on_dark(),
    );
}

fn left_navbar(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(18, 25, 34));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(paint, size, x, rect.y + 1, width, "Left Navbar", heading());
    nav_item(paint, size, x, rect.y + 3, width, "● Dashboard", true);
    nav_item(paint, size, x, rect.y + 4, width, "○ Projects", false);
    nav_item(paint, size, x, rect.y + 5, width, "○ Agents", false);
    nav_item(paint, size, x, rect.y + 6, width, "○ Logs", false);
}

fn content_panel(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(15, 20, 27));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(
        paint,
        size,
        x,
        rect.y + 1,
        width,
        "Content",
        fg_bg(
            PaintColor::Rgb(240, 244, 250),
            PaintColor::Rgb(15, 20, 27),
            PaintAttrs::BOLD,
        ),
    );
    text_fit(
        paint,
        size,
        x,
        rect.y + 2,
        width,
        "Main work surface with responsive side navigation.",
        fg_bg(
            PaintColor::Rgb(142, 157, 179),
            PaintColor::Rgb(15, 20, 27),
            PaintAttrs::empty(),
        ),
    );

    let card_y = rect.y + 4;
    let card_height = 5.min(rect.height.saturating_sub(6));
    if card_height >= 3 {
        let gap = if width >= 42 { 2 } else { 1 };
        let card_width = if width >= 42 {
            width.saturating_sub(gap) / 2
        } else {
            width
        };
        stat_card(
            paint,
            size,
            Rect::new(x, card_y, card_width, card_height),
            "Requests",
            "1,284",
            PaintColor::Rgb(77, 163, 255),
        );
        if width >= 42 {
            stat_card(
                paint,
                size,
                Rect::new(x + card_width + gap, card_y, card_width, card_height),
                "Latency",
                "42 ms",
                PaintColor::Rgb(93, 213, 177),
            );
        }
    }

    let list_y = card_y + card_height + 2;
    if list_y < rect.y + rect.height.saturating_sub(1) {
        text_fit(paint, size, x, list_y, width, "Recent activity", heading());
        activity(
            paint,
            size,
            x,
            list_y + 2,
            width,
            "render patch diffed without full clear",
        );
        activity(
            paint,
            size,
            x,
            list_y + 3,
            width,
            "layout shell resized to viewport",
        );
        activity(
            paint,
            size,
            x,
            list_y + 4,
            width,
            "right navbar kept in bounds",
        );
    }
}

fn right_navbar(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(18, 25, 34));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(paint, size, x, rect.y + 1, width, "Right Navbar", heading());
    text_fit(
        paint,
        size,
        x,
        rect.y + 3,
        width,
        "Runtime",
        muted_on_panel(),
    );
    badge(paint, size, x, rect.y + 4, width, "crossterm backend");
    text_fit(
        paint,
        size,
        x,
        rect.y + 6,
        width,
        "Viewport",
        muted_on_panel(),
    );
    text_fit(
        paint,
        size,
        x,
        rect.y + 7,
        width,
        format!("{} x {}", size.width, size.height),
        body_on_panel(),
    );
    text_fit(paint, size, x, rect.y + 9, width, "Mode", muted_on_panel());
    badge(paint, size, x, rect.y + 10, width, "responsive");
}

fn stat_card(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    rect: Rect,
    label: &str,
    value: &str,
    color: PaintColor,
) {
    panel(paint, rect, PaintColor::Rgb(20, 27, 36));
    text_fit(
        paint,
        size,
        rect.x + 2,
        rect.y + 1,
        rect.width.saturating_sub(4),
        label,
        muted_on_card(),
    );
    text_fit(
        paint,
        size,
        rect.x + 2,
        rect.y + 2,
        rect.width.saturating_sub(4),
        value,
        fg_bg(color, PaintColor::Rgb(20, 27, 36), PaintAttrs::BOLD),
    );
}

fn nav_item(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    selected: bool,
) {
    let background = if selected {
        PaintColor::Rgb(31, 45, 62)
    } else {
        PaintColor::Rgb(18, 25, 34)
    };
    text_fit(
        paint,
        size,
        x,
        y,
        width,
        label,
        fg_bg(
            if selected {
                PaintColor::Rgb(93, 213, 177)
            } else {
                PaintColor::Rgb(170, 183, 201)
            },
            background,
            PaintAttrs::empty(),
        ),
    );
}

fn activity(paint: &mut Vec<PaintPrimitive>, size: Size, x: u16, y: u16, width: u16, label: &str) {
    text_fit(
        paint,
        size,
        x,
        y,
        width,
        format!("• {label}"),
        body_on_content(),
    );
}

fn badge(paint: &mut Vec<PaintPrimitive>, size: Size, x: u16, y: u16, width: u16, label: &str) {
    let width = width.min(label.chars().count() as u16 + 2);
    if width == 0 || x >= size.width || y >= size.height {
        return;
    }
    paint.push(fill(
        Rect::new(x, y, width.min(size.width.saturating_sub(x)), 1),
        bg(PaintColor::Rgb(34, 50, 67)),
    ));
    text_fit(
        paint,
        size,
        x + 1,
        y,
        width.saturating_sub(2),
        label,
        fg_bg(
            PaintColor::Rgb(223, 231, 241),
            PaintColor::Rgb(34, 50, 67),
            PaintAttrs::empty(),
        ),
    );
}

fn panel(paint: &mut Vec<PaintPrimitive>, rect: Rect, color: PaintColor) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    paint.push(fill(rect, bg(color)));
    paint.push(PaintPrimitive::Border {
        rect,
        style: fg_bg(PaintColor::Rgb(55, 70, 90), color, PaintAttrs::empty()),
    });
}

fn text_fit(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    max_width: u16,
    content: impl Into<String>,
    style: PaintStyle,
) {
    if max_width == 0 || x >= size.width || y >= size.height {
        return;
    }
    let available = max_width.min(size.width.saturating_sub(x));
    let content = content
        .into()
        .chars()
        .take(available as usize)
        .collect::<String>();
    if content.is_empty() {
        return;
    }
    paint.push(fill(
        Rect::new(x, y, content.chars().count() as u16, 1),
        style,
    ));
    paint.push(PaintPrimitive::TextRun {
        x,
        y,
        text: content,
    });
}

fn fill(rect: Rect, style: PaintStyle) -> PaintPrimitive {
    PaintPrimitive::FillRect { rect, style }
}

fn bg(background: PaintColor) -> PaintStyle {
    PaintStyle {
        background: Some(background),
        ..PaintStyle::default()
    }
}

fn fg_bg(foreground: PaintColor, background: PaintColor, attrs: PaintAttrs) -> PaintStyle {
    PaintStyle {
        foreground: Some(foreground),
        background: Some(background),
        attrs,
    }
}

fn heading() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(230, 237, 246),
        PaintColor::Rgb(18, 25, 34),
        PaintAttrs::BOLD,
    )
}

fn body_on_panel() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(202, 213, 228),
        PaintColor::Rgb(18, 25, 34),
        PaintAttrs::empty(),
    )
}

fn body_on_content() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(202, 213, 228),
        PaintColor::Rgb(15, 20, 27),
        PaintAttrs::empty(),
    )
}

fn muted_on_panel() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(136, 151, 171),
        PaintColor::Rgb(18, 25, 34),
        PaintAttrs::empty(),
    )
}

fn muted_on_card() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(136, 151, 171),
        PaintColor::Rgb(20, 27, 36),
        PaintAttrs::empty(),
    )
}

fn muted_on_dark() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(136, 151, 171),
        PaintColor::Rgb(13, 18, 26),
        PaintAttrs::empty(),
    )
}

fn muted_on_canvas() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(136, 151, 171),
        PaintColor::Rgb(10, 12, 15),
        PaintAttrs::empty(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_primitives_stay_inside_common_terminal_sizes() {
        for size in [
            Size::new(120, 36),
            Size::new(100, 28),
            Size::new(80, 24),
            Size::new(72, 20),
            Size::new(60, 18),
            Size::new(40, 14),
            Size::new(30, 10),
        ] {
            for primitive in shell_primitives(size) {
                assert!(
                    primitive_fits(size, &primitive),
                    "primitive exceeded {size:?}: {primitive:?}"
                );
            }
        }
    }

    fn primitive_fits(size: Size, primitive: &PaintPrimitive) -> bool {
        match primitive {
            PaintPrimitive::FillRect { rect, .. } | PaintPrimitive::Border { rect, .. } => {
                rect.x.saturating_add(rect.width) <= size.width
                    && rect.y.saturating_add(rect.height) <= size.height
            }
            PaintPrimitive::TextRun { x, y, text } => {
                *x < size.width
                    && *y < size.height
                    && x.saturating_add(text.chars().count() as u16) <= size.width
            }
            PaintPrimitive::Cursor { x, y } => *x < size.width && *y < size.height,
            PaintPrimitive::Clip { rect, children } => {
                rect.x.saturating_add(rect.width) <= size.width
                    && rect.y.saturating_add(rect.height) <= size.height
                    && children.iter().all(|child| primitive_fits(size, child))
            }
            PaintPrimitive::Layer { children, .. } => {
                children.iter().all(|child| primitive_fits(size, child))
            }
        }
    }
}
