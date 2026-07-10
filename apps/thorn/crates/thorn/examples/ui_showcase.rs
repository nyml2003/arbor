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
        screen.apply(&dashboard_primitives(size));
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
                break
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

fn dashboard_primitives(size: Size) -> Vec<PaintPrimitive> {
    let mut paint = Vec::new();
    paint.push(fill(
        Rect::new(0, 0, size.width, size.height),
        bg(PaintColor::Rgb(12, 14, 18)),
    ));

    if size.width < 24 || size.height < 8 {
        text_fit(
            &mut paint,
            size,
            1,
            1,
            size.width.saturating_sub(2),
            "Thorn UI",
            fg_bg(
                PaintColor::Rgb(232, 238, 247),
                PaintColor::Rgb(12, 14, 18),
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
            fg_bg(
                PaintColor::Rgb(137, 151, 173),
                PaintColor::Rgb(12, 14, 18),
                PaintAttrs::empty(),
            ),
        );
        return paint;
    }

    let pad = if size.width >= 80 { 2 } else { 1 };
    let content_width = size.width.saturating_sub(pad * 2).max(1);
    let header_height = if size.height >= 12 { 4 } else { 3 };
    let header = Rect::new(pad, 1, content_width, header_height);
    panel(&mut paint, header, PaintColor::Rgb(24, 31, 42));
    text_fit(
        &mut paint,
        size,
        header.x + 2,
        header.y + 1,
        header.width.saturating_sub(4),
        "THORN CONTROL SURFACE",
        fg_bg(
            PaintColor::Rgb(232, 238, 247),
            PaintColor::Rgb(24, 31, 42),
            PaintAttrs::BOLD,
        ),
    );
    if header_height > 3 {
        text_fit(
            &mut paint,
            size,
            header.x + 2,
            header.y + 2,
            header.width.saturating_sub(4),
            "Runtime primitives | terminal patch backend | responsive layout",
            fg_bg(
                PaintColor::Rgb(137, 151, 173),
                PaintColor::Rgb(24, 31, 42),
                PaintAttrs::empty(),
            ),
        );
    }
    if header.width > 40 {
        pill(
            &mut paint,
            size,
            header.x + header.width.saturating_sub(22),
            header.y + 1,
            "READY",
            PaintColor::Rgb(42, 157, 143),
        );
        pill(
            &mut paint,
            size,
            header.x + header.width.saturating_sub(13),
            header.y + 1,
            "MVP+",
            PaintColor::Rgb(82, 113, 255),
        );
    }

    let footer_y = size.height.saturating_sub(1);
    let usable_bottom = footer_y.saturating_sub(2);
    let columns = if content_width >= 88 {
        3
    } else if content_width >= 58 {
        2
    } else {
        1
    };
    let gap = if columns > 1 { 2 } else { 0 };
    let card_width = content_width
        .saturating_sub(gap * (columns - 1))
        .checked_div(columns)
        .unwrap_or(content_width)
        .max(1);
    let card_height = if size.height >= 22 { 8 } else { 6 };
    let cards_y = header.y + header.height + 1;

    for index in 0..3 {
        let row = index / columns;
        let column = index % columns;
        let x = pad + column * (card_width + gap);
        let y = cards_y + row * (card_height + 1);
        if y > usable_bottom {
            continue;
        }
        let height = card_height.min(usable_bottom.saturating_sub(y).saturating_add(1));
        let rect = Rect::new(x, y, card_width, height);
        match index {
            0 => frame_card(&mut paint, size, rect),
            1 => adapter_card(&mut paint, size, rect),
            _ => input_card(&mut paint, size, rect),
        }
    }

    let top_rows = (3 + columns - 1) / columns;
    let bottom_y = cards_y + top_rows * (card_height + 1);
    if bottom_y <= usable_bottom && usable_bottom.saturating_sub(bottom_y) >= 4 {
        let bottom_columns = if content_width >= 76 { 2 } else { 1 };
        let bottom_gap = if bottom_columns > 1 { 3 } else { 0 };
        let bottom_width = content_width
            .saturating_sub(bottom_gap * (bottom_columns - 1))
            .checked_div(bottom_columns)
            .unwrap_or(content_width)
            .max(1);
        let bottom_height = 10.min(usable_bottom.saturating_sub(bottom_y).saturating_add(1));
        transcript_card(
            &mut paint,
            size,
            Rect::new(pad, bottom_y, bottom_width, bottom_height),
        );
        if bottom_columns > 1 {
            roadmap_card(
                &mut paint,
                size,
                Rect::new(
                    pad + bottom_width + bottom_gap,
                    bottom_y,
                    bottom_width,
                    bottom_height,
                ),
            );
        } else if bottom_y + bottom_height < usable_bottom {
            roadmap_card(
                &mut paint,
                size,
                Rect::new(
                    pad,
                    bottom_y + bottom_height + 1,
                    bottom_width,
                    usable_bottom.saturating_sub(bottom_y + bottom_height),
                ),
            );
        }
    }

    text_fit(
        &mut paint,
        size,
        pad,
        footer_y,
        content_width,
        "q exits | resize redraws through CrosstermPresenter",
        fg_bg(
            PaintColor::Rgb(137, 151, 173),
            PaintColor::Rgb(12, 14, 18),
            PaintAttrs::empty(),
        ),
    );

    paint
}

fn frame_card(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(20, 26, 34));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(
        paint,
        size,
        x,
        rect.y + 1,
        width,
        "Frame Pipeline",
        subtle(),
    );
    metric(paint, size, x, rect.y + 3, width, "Host nodes", "128");
    metric(paint, size, x, rect.y + 4, width, "Dirty cells", "14");
    metric(paint, size, x, rect.y + 5, width, "Layout cache", "hit");
    progress(
        paint,
        size,
        x,
        rect.y + rect.height.saturating_sub(2),
        width.min(20),
        15,
        PaintColor::Rgb(42, 157, 143),
    );
}

fn adapter_card(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(20, 26, 34));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(
        paint,
        size,
        x,
        rect.y + 1,
        width,
        "Adapter Health",
        subtle(),
    );
    status_row(
        paint,
        size,
        x,
        rect.y + 3,
        width,
        "Headless",
        "online",
        PaintColor::Rgb(42, 157, 143),
    );
    status_row(
        paint,
        size,
        x,
        rect.y + 4,
        width,
        "Terminal",
        "patching",
        PaintColor::Rgb(233, 196, 106),
    );
    status_row(
        paint,
        size,
        x,
        rect.y + 5,
        width,
        "Win32",
        "dry-run",
        PaintColor::Rgb(141, 153, 174),
    );
    progress(
        paint,
        size,
        x,
        rect.y + rect.height.saturating_sub(2),
        width.min(20),
        12,
        PaintColor::Rgb(233, 196, 106),
    );
}

fn input_card(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(20, 26, 34));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(paint, size, x, rect.y + 1, width, "Input Stack", subtle());
    text_fit(
        paint,
        size,
        x,
        rect.y + 3,
        width,
        "Layered keymap",
        normal(),
    );
    text_fit(
        paint,
        size,
        x,
        rect.y + 4,
        width,
        "Intent resolver",
        normal(),
    );
    text_fit(paint, size, x, rect.y + 5, width, "Bounded queue", normal());
    progress(
        paint,
        size,
        x,
        rect.y + rect.height.saturating_sub(2),
        width.min(21),
        18,
        PaintColor::Rgb(82, 113, 255),
    );
}

fn transcript_card(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(18, 23, 31));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(
        paint,
        size,
        x,
        rect.y + 1,
        width,
        "Agent Transcript",
        subtle_dark(),
    );
    transcript(
        paint,
        size,
        x,
        rect.y + 3,
        width,
        "user",
        "summarize workspace state",
    );
    transcript(
        paint,
        size,
        x,
        rect.y + 5,
        width,
        "tool",
        "cargo test --workspace: passed",
    );
    transcript(
        paint,
        size,
        x,
        rect.y + 7,
        width,
        "assistant",
        "Thorn is ready for richer demos",
    );
}

fn roadmap_card(paint: &mut Vec<PaintPrimitive>, size: Size, rect: Rect) {
    panel(paint, rect, PaintColor::Rgb(18, 23, 31));
    let x = rect.x + 2;
    let width = rect.width.saturating_sub(4);
    text_fit(paint, size, x, rect.y + 1, width, "Roadmap", subtle_dark());
    checklist(paint, size, x, rect.y + 3, width, "[x] headless snapshots");
    checklist(paint, size, x, rect.y + 4, width, "[x] ANSI dirty patches");
    checklist(paint, size, x, rect.y + 5, width, "[x] layout cache stats");
    checklist(paint, size, x, rect.y + 6, width, "[ ] raw terminal mode");
    checklist(
        paint,
        size,
        x,
        rect.y + 7,
        width,
        "[ ] native Win32 presenter",
    );
}

fn panel(paint: &mut Vec<PaintPrimitive>, rect: Rect, color: PaintColor) {
    paint.push(fill(rect, bg(color)));
    paint.push(PaintPrimitive::Border {
        rect,
        style: fg_bg(PaintColor::Rgb(57, 70, 89), color, PaintAttrs::empty()),
    });
}

fn pill(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    label: &str,
    color: PaintColor,
) {
    let width = label.len() as u16 + 4;
    paint.push(fill(Rect::new(x, y, width, 1), bg(color)));
    text_fit(
        paint,
        size,
        x + 2,
        y,
        width.saturating_sub(4),
        label,
        fg_bg(PaintColor::Rgb(255, 255, 255), color, PaintAttrs::BOLD),
    );
}

fn metric(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    value: &str,
) {
    if width >= 22 {
        text_fit(paint, size, x, y, 14, label, normal());
        text_fit(
            paint,
            size,
            x + 17,
            y,
            width.saturating_sub(17),
            value,
            fg_bg(
                PaintColor::Rgb(232, 238, 247),
                PaintColor::Rgb(20, 26, 34),
                PaintAttrs::BOLD,
            ),
        );
    } else {
        text_fit(
            paint,
            size,
            x,
            y,
            width,
            format!("{label} {value}"),
            normal(),
        );
    }
}

fn status_row(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    value: &str,
    color: PaintColor,
) {
    if width >= 20 {
        text_fit(paint, size, x, y, 12, label, normal());
        text_fit(
            paint,
            size,
            x + 14,
            y,
            width.saturating_sub(14),
            value,
            fg_bg(color, PaintColor::Rgb(20, 26, 34), PaintAttrs::BOLD),
        );
    } else {
        text_fit(
            paint,
            size,
            x,
            y,
            width,
            format!("{label} {value}"),
            fg_bg(color, PaintColor::Rgb(20, 26, 34), PaintAttrs::empty()),
        );
    }
}

fn transcript(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    width: u16,
    role: &str,
    body: &str,
) {
    text_fit(
        paint,
        size,
        x,
        y,
        width.min(9),
        role,
        fg_bg(
            PaintColor::Rgb(93, 213, 177),
            PaintColor::Rgb(18, 23, 31),
            PaintAttrs::BOLD,
        ),
    );
    if width > 11 {
        text_fit(
            paint,
            size,
            x + 11,
            y,
            width.saturating_sub(11),
            body,
            normal_dark_panel(),
        );
    }
}

fn checklist(paint: &mut Vec<PaintPrimitive>, size: Size, x: u16, y: u16, width: u16, body: &str) {
    let color = if body.starts_with("[x]") {
        PaintColor::Rgb(93, 213, 177)
    } else {
        PaintColor::Rgb(141, 153, 174)
    };
    text_fit(
        paint,
        size,
        x,
        y,
        width,
        body,
        fg_bg(color, PaintColor::Rgb(18, 23, 31), PaintAttrs::empty()),
    );
}

fn progress(
    paint: &mut Vec<PaintPrimitive>,
    size: Size,
    x: u16,
    y: u16,
    width: u16,
    filled: u16,
    color: PaintColor,
) {
    if width == 0 || x >= size.width || y >= size.height {
        return;
    }
    let width = width.min(size.width.saturating_sub(x));
    paint.push(fill(
        Rect::new(x, y, width, 1),
        bg(PaintColor::Rgb(35, 43, 56)),
    ));
    paint.push(fill(Rect::new(x, y, filled.min(width), 1), bg(color)));
}

fn text(
    paint: &mut Vec<PaintPrimitive>,
    x: u16,
    y: u16,
    content: impl Into<String>,
    style: PaintStyle,
) {
    let content = content.into();
    paint.push(fill(Rect::new(x, y, content.len() as u16, 1), style));
    paint.push(PaintPrimitive::TextRun {
        x,
        y,
        text: content,
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
    let content = truncate_chars(&content.into(), available);
    if content.is_empty() {
        return;
    }
    text(paint, x, y, content, style);
}

fn truncate_chars(content: &str, max_width: u16) -> String {
    content.chars().take(max_width as usize).collect()
}

fn fill(rect: Rect, style: PaintStyle) -> PaintPrimitive {
    PaintPrimitive::FillRect { rect, style }
}

fn normal() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(202, 211, 224),
        PaintColor::Rgb(20, 26, 34),
        PaintAttrs::empty(),
    )
}

fn normal_dark_panel() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(202, 211, 224),
        PaintColor::Rgb(18, 23, 31),
        PaintAttrs::empty(),
    )
}

fn subtle() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(137, 151, 173),
        PaintColor::Rgb(20, 26, 34),
        PaintAttrs::BOLD,
    )
}

fn subtle_dark() -> PaintStyle {
    fg_bg(
        PaintColor::Rgb(137, 151, 173),
        PaintColor::Rgb(18, 23, 31),
        PaintAttrs::BOLD,
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_primitives_stay_inside_common_terminal_sizes() {
        for size in [
            Size::new(120, 36),
            Size::new(100, 24),
            Size::new(80, 24),
            Size::new(60, 20),
            Size::new(40, 14),
            Size::new(24, 8),
        ] {
            for primitive in dashboard_primitives(size) {
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
