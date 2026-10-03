//! Large block-letter text filled with a gradient.
//!
//! Two fonts are available (see [`Font`]): the 8×8 Serif bitmap and a 3-row
//! half-block Sans font in the Crush wordmark style. Any string renders.

mod font;

pub use font::Font;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};

use crate::color::blend_2d;

/// The default gradient when no stops are supplied.
const DEFAULT_STOP: Color = Color::Rgb(0xff, 0xff, 0xff);

/// A rendered banner.
pub struct Model {
    lines: Vec<Line<'static>>,
    width: u16,
    height: u16,
}

impl Model {
    /// Renders `text` in the default Sans font.
    pub fn new(text: &str, stops: &[Color]) -> Self {
        Self::with_font(text, Font::Sans, stops)
    }

    /// Renders `text` in `font`, filling the set cells with a diagonal gradient
    /// through `stops`.
    pub fn with_font(text: &str, font: Font, stops: &[Color]) -> Self {
        let rows = font.rows(text);
        let height = rows.len();
        let width = rows
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        let columns = width.max(1);

        let empty = [DEFAULT_STOP];
        let stops = if stops.is_empty() { &empty } else { stops };
        let colors = blend_2d(columns, height, 45.0, stops);

        let mut lines = Vec::with_capacity(height);
        for (y, row) in rows.iter().enumerate() {
            let mut spans = Vec::new();
            for (x, cell) in row.chars().enumerate() {
                if cell == ' ' {
                    spans.push(Span::raw(" "));
                } else {
                    let color = colors.get(y * columns + x).copied().unwrap_or(DEFAULT_STOP);
                    spans.push(Span::styled(cell.to_string(), Style::default().fg(color)));
                }
            }
            lines.push(Line::from(spans));
        }

        Self {
            lines,
            width: u16::try_from(width).unwrap_or(u16::MAX),
            height: u16::try_from(height).unwrap_or(u16::MAX),
        }
    }

    /// The rendered rows.
    pub fn view(&self) -> &[Line<'static>] {
        &self.lines
    }

    /// The width in cells.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// The height in rows.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Draws the banner into `area`, clipping to its width and height.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (row, line) in self.lines.iter().enumerate() {
            if row >= area.height as usize {
                break;
            }
            buf.set_line(area.x, area.y + row as u16, line, area.width);
        }
    }
}
