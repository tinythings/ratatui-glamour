use std::fmt::Write;

use ratatui::{buffer::Buffer, layout::Rect, style::Color};
use unicode_width::UnicodeWidthStr;

use crate::color::color_to_rgb;

pub fn render_buffer(buf: &Buffer, area: Rect) -> String {
    let mut out = String::new();
    let mut last_fg = None;
    let mut last_bg = None;

    for y in 0..area.height {
        let mut x = 0;
        while x < area.width {
            let Some(cell) = buf.cell((area.x + x, area.y + y)) else {
                x += 1;
                continue;
            };

            let fg = cell.fg;
            let bg = cell.bg;
            if last_fg != Some(fg) || last_bg != Some(bg) {
                push_style(&mut out, fg, bg);
                last_fg = Some(fg);
                last_bg = Some(bg);
            }

            let symbol = cell.symbol();
            let cell_width = UnicodeWidthStr::width(symbol).max(1) as u16;
            out.push_str(if symbol.is_empty() { " " } else { symbol });
            x = x.saturating_add(cell_width);
        }
        out.push_str("\x1b[0m\n");
        last_fg = None;
        last_bg = None;
    }

    out.push_str("\x1b[0m");
    out
}

fn push_style(out: &mut String, fg: Color, bg: Color) {
    out.push_str("\x1b[");
    push_color_code(out, fg, true);
    out.push(';');
    push_color_code(out, bg, false);
    out.push('m');
}

fn push_color_code(out: &mut String, color: Color, foreground: bool) {
    let reset = if foreground { 39 } else { 49 };
    let prefix = if foreground { 38 } else { 48 };
    match color {
        Color::Reset => {
            let _ = write!(out, "{reset}");
        }
        Color::Indexed(idx) => {
            let _ = write!(out, "{prefix};5;{idx}");
        }
        other => {
            let (r, g, b) = color_to_rgb(other);
            let _ = write!(out, "{prefix};2;{r};{g};{b}");
        }
    }
}
