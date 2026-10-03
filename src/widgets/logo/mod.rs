//! A compact wordmark in the Crush style.
//!
//! Renders a title in the [`Font::Sans`] font, bracketed by `╱` field rows and
//! preceded by a charm/version meta line, matching Crush's sidebar logo
//! (`internal/ui/logo/logo.go`).
//!
//! [`Font::Sans`]: crate::widgets::bigtext::Font::Sans

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::color::blend_1d;
use crate::widgets::bigtext::Font;

/// The colors of one wordmark.
pub struct Palette {
    /// The `╱` field rows.
    pub field: Color,
    /// The charm label.
    pub charm: Color,
    /// The version label.
    pub version: Color,
    /// The title gradient start.
    pub title_from: Color,
    /// The title gradient end.
    pub title_to: Color,
}

/// The rendered width of a wordmark in cells.
pub fn width(title: &str, charm: &str, version: &str) -> usize {
    let title_width = Font::Sans
        .rows(title)
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    title_width
        .max(charm.chars().count() + 1 + version.chars().count())
        .max(1)
}

/// Renders `title` as a compact wordmark with `charm` and `version`.
pub fn render(title: &str, charm: &str, version: &str, palette: &Palette) -> Vec<Line<'static>> {
    let rows = Font::Sans.rows(title);
    let width = width(title, charm, version);
    let field = "╱".repeat(width);

    let mut lines = vec![
        field_line(&field, palette.field),
        field_line(&field, palette.field),
        meta_line(charm, version, width, palette),
    ];
    let colors = blend_1d(width, &[palette.title_from, palette.title_to]);
    for row in &rows {
        let mut spans = Vec::new();
        for (column, cell) in row.chars().enumerate() {
            if cell == ' ' {
                spans.push(Span::raw(" "));
            } else {
                let color = colors.get(column).copied().unwrap_or(palette.title_to);
                spans.push(Span::styled(
                    cell.to_string(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ));
            }
        }
        lines.push(Line::from(spans));
    }
    lines.push(field_line(&field, palette.field));
    lines.push(Line::from(""));
    lines
}

fn field_line(field: &str, color: Color) -> Line<'static> {
    Line::from(Span::styled(field.to_string(), Style::default().fg(color)))
}

fn meta_line(charm: &str, version: &str, width: usize, palette: &Palette) -> Line<'static> {
    let gap = width.saturating_sub(charm.chars().count() + version.chars().count());
    Line::from(vec![
        Span::styled(charm.to_string(), Style::default().fg(palette.charm)),
        Span::raw(" ".repeat(gap)),
        Span::styled(version.to_string(), Style::default().fg(palette.version)),
    ])
}
