use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::border::Border;
use crate::color::lerp_color;

pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(area.x + (area.width.saturating_sub(width)) / 2, area.y + (area.height.saturating_sub(height)) / 2, width, height)
}

pub fn render_rounded_panel(buf: &mut Buffer, area: Rect, border_style: Style, fill_style: Style) -> Rect {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(border_style)
        .style(fill_style);
    let inner = block.inner(area);
    block.render(area, buf);
    inner
}

pub fn render_gradient_rounded_panel(buf: &mut Buffer, area: Rect, fill_style: Style, stops: &[ratatui::style::Color]) -> Rect {
    let lines = gradient_rounded_panel_lines(area.width as usize, area.height as usize, fill_style, stops);
    for (row, line) in lines.into_iter().enumerate() {
        buf.set_line(area.x, area.y + row as u16, &line, area.width);
    }

    Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), area.height.saturating_sub(2))
}

pub fn gradient_rounded_panel_lines(
    width: usize, height: usize, fill_style: Style, stops: &[ratatui::style::Color],
) -> Vec<Line<'static>> {
    if width == 0 || height == 0 {
        return Vec::new();
    }

    let mut grid = vec![vec![Span::styled(" ", fill_style); width]; height];
    let area = Rect::new(0, 0, width as u16, height as u16);
    let perimeter = rounded_perimeter(area);
    let colors = gradient_colors(perimeter.len(), stops);
    for (idx, (x, y, symbol)) in perimeter.into_iter().enumerate() {
        grid[y as usize][x as usize] = Span::styled(symbol.to_string(), fill_style.fg(colors[idx]));
    }

    grid.into_iter().map(Line::from).collect()
}

pub fn fill_repeated_text(buf: &mut Buffer, area: Rect, token: &str, style: Style) {
    if area.width == 0 || area.height == 0 || token.is_empty() {
        return;
    }

    let token_width = UnicodeWidthStr::width(token).max(1);
    let repeat = (area.width as usize / token_width) + 3;
    let row = token.repeat(repeat);
    let line = Line::from(vec![Span::styled(row, style)]);
    for y in 0..area.height {
        buf.set_line(area.x, area.y + y, &line, area.width);
    }
}

pub fn place_with_pattern(buf: &mut Buffer, area: Rect, width: u16, height: u16, token: &str, style: Style) -> Rect {
    fill_repeated_text(buf, area, token, style);
    centered_rect(area, width, height)
}

pub fn render_classic_tabs_row(
    buf: &mut Buffer, area: Rect, labels: &[&str], active: usize, border_style: Style, active_label_style: Style, inactive_label_style: Style,
) {
    if area.width < 4 || area.height < 3 || labels.is_empty() {
        return;
    }

    let mut x = area.x;
    let y = area.y;
    let bottom_y = y + 2;

    let widths: Vec<u16> = labels.iter().map(|label| UnicodeWidthStr::width(*label) as u16 + 4).collect();
    let used: u16 = widths.iter().sum();

    for (idx, label) in labels.iter().enumerate() {
        let w = widths[idx];
        render_classic_tab(buf, Rect::new(x, y, w, 3), label, idx == active, border_style, active_label_style, inactive_label_style);
        x += w;
    }

    if used < area.width {
        let gap_width = area.width - used;
        if gap_width > 0 {
            for dx in 0..gap_width {
                if let Some(cell) = buf.cell_mut((x + dx, bottom_y)) {
                    cell.set_symbol("─");
                    cell.set_style(border_style);
                }
            }
        }
    }
}

fn render_classic_tab(
    buf: &mut Buffer, area: Rect, label: &str, active: bool, border_style: Style, active_label_style: Style, inactive_label_style: Style,
) {
    if area.width < 4 || area.height < 3 {
        return;
    }

    buf.set_string(area.x, area.y, "╭", border_style);
    for dx in 1..area.width - 1 {
        buf.set_string(area.x + dx, area.y, "─", border_style);
    }
    buf.set_string(area.right() - 1, area.y, "╮", border_style);
    buf.set_string(area.x, area.y + 1, "│", border_style);
    buf.set_string(area.right() - 1, area.y + 1, "│", border_style);

    if active {
        let label_text = format!(" {label} ");
        let label_w = UnicodeWidthStr::width(label_text.as_str());
        let interior_w = area.width as usize - 2;
        let left = interior_w.saturating_sub(label_w) / 2;
        let right = interior_w.saturating_sub(label_w + left);
        let line = Line::from(vec![
            Span::styled(" ".repeat(left), inactive_label_style),
            Span::styled(label_text, active_label_style),
            Span::styled(" ".repeat(right), inactive_label_style),
        ]);
        buf.set_line(area.x + 1, area.y + 1, &line, area.width - 2);
    } else {
        let centered = center_text(label, area.width as usize - 2);
        buf.set_line(
            area.x + 1,
            area.y + 1,
            &Line::from(vec![Span::styled(centered, inactive_label_style)]),
            area.width - 2,
        );
    }

    let (left, fill, right) = if active { ("┘", " ", "└") } else { ("┴", "─", "┴") };
    buf.set_string(area.x, area.y + 2, left, border_style);
    for dx in 1..area.width - 1 {
        buf.set_string(area.x + dx, area.y + 2, fill, if active { inactive_label_style } else { border_style });
    }
    buf.set_string(area.right() - 1, area.y + 2, right, border_style);
}

fn center_text(text: &str, width: usize) -> String {
    let used = UnicodeWidthStr::width(text);
    if used >= width {
        return text.to_string();
    }
    let left = (width - used) / 2;
    let right = width - used - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn gradient_colors(count: usize, stops: &[ratatui::style::Color]) -> Vec<ratatui::style::Color> {
    if count == 0 || stops.is_empty() {
        return Vec::new();
    }
    if stops.len() == 1 {
        return vec![stops[0]; count];
    }

    let segments = stops.len() - 1;
    (0..count)
        .map(|idx| {
            let pos = if count > 1 { idx as f32 / (count - 1) as f32 } else { 0.0 };
            let scaled = pos * segments as f32;
            let base = scaled.floor() as usize;
            if base >= segments {
                *stops.last().unwrap()
            } else {
                let t = scaled - base as f32;
                lerp_color(stops[base], stops[base + 1], t)
            }
        })
        .collect()
}

fn rounded_perimeter(area: Rect) -> Vec<(u16, u16, &'static str)> {
    let mut out = Vec::new();
    if area.width < 2 || area.height < 2 {
        return out;
    }

    out.push((area.x, area.y, "╭"));
    for dx in 1..area.width - 1 {
        out.push((area.x + dx, area.y, "─"));
    }
    out.push((area.right() - 1, area.y, "╮"));
    for dy in 1..area.height - 1 {
        out.push((area.right() - 1, area.y + dy, "│"));
    }
    out.push((area.right() - 1, area.bottom() - 1, "╯"));
    for dx in (1..area.width - 1).rev() {
        out.push((area.x + dx, area.bottom() - 1, "─"));
    }
    out.push((area.x, area.bottom() - 1, "╰"));
    for dy in (1..area.height - 1).rev() {
        out.push((area.x, area.y + dy, "│"));
    }
    out
}
