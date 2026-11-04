//! Radio group widget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Orientation {
    #[default]
    Horizontal,
    Vertical,
}

/// A radio group with a fixed-width symbol slot for every option.
#[derive(Clone, Debug)]
pub struct Model {
    pub options: Vec<String>,
    pub selected: usize,
    pub orientation: Orientation,
    pub checked_symbol: String,
    pub unchecked_symbol: String,
    /// Total cell width reserved for each symbol, including the glyph.
    pub symbol_width: usize,
    /// Additional cells between each symbol slot and its label.
    pub label_gap: usize,
    /// Cells between adjacent options in horizontal orientation.
    pub option_gap: usize,
    pub style: Style,
    pub checked_style: Style,
    pub unchecked_style: Style,
}

impl Default for Model {
    fn default() -> Self {
        Self::new(Vec::<String>::new())
    }
}

impl Model {
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected: 0,
            orientation: Orientation::Horizontal,
            checked_symbol: "⦿".to_string(),
            unchecked_symbol: "◯".to_string(),
            symbol_width: 2,
            label_gap: 2,
            option_gap: 4,
            style: Style::default(),
            checked_style: Style::default(),
            unchecked_style: Style::default(),
        }
    }

    pub fn select(&mut self, index: usize) {
        if self.options.is_empty() {
            self.selected = 0;
        } else {
            self.selected = index.min(self.options.len() - 1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    pub fn select_previous(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + self.options.len() - 1) % self.options.len();
        }
    }

    /// Move with arrow keys. Returns whether the key was consumed.
    pub fn handle_key(&mut self, event: &KeyEvent) -> bool {
        match event.code {
            KeyCode::Left | KeyCode::Up => {
                self.select_previous();
                true
            }
            KeyCode::Right | KeyCode::Down => {
                self.select_next();
                true
            }
            _ => false,
        }
    }

    /// Horizontal representation. Use [`Self::lines`] for both orientations.
    pub fn view(&self) -> Line<'static> {
        let mut spans = Vec::new();
        for (index, option) in self.options.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw(" ".repeat(self.option_gap)));
            }
            self.push_option(&mut spans, index, option);
        }
        Line::from(spans)
    }

    /// One line per option for vertical orientation, or a single horizontal line.
    pub fn lines(&self) -> Vec<Line<'static>> {
        match self.orientation {
            Orientation::Horizontal => vec![self.view()],
            Orientation::Vertical => self
                .options
                .iter()
                .enumerate()
                .map(|(index, option)| {
                    let mut spans = Vec::new();
                    self.push_option(&mut spans, index, option);
                    Line::from(spans)
                })
                .collect(),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        match self.orientation {
            Orientation::Horizontal => {
                buf.set_line(area.x, area.y, &self.view(), area.width);
            }
            Orientation::Vertical => {
                for (offset, line) in self.lines().iter().enumerate() {
                    let y = area.y.saturating_add(offset as u16);
                    if y >= area.bottom() {
                        break;
                    }
                    buf.set_line(area.x, y, line, area.width);
                }
            }
        }
    }

    fn push_option(&self, spans: &mut Vec<Span<'static>>, index: usize, option: &str) {
        let selected = index == self.selected;
        let (symbol, symbol_style) = if selected {
            (self.checked_symbol.as_str(), self.checked_style)
        } else {
            (self.unchecked_symbol.as_str(), self.unchecked_style)
        };
        let symbol_width = UnicodeWidthStr::width(symbol);
        let padding = self.symbol_width.saturating_sub(symbol_width);
        spans.push(Span::styled(symbol.to_string(), symbol_style));
        spans.push(Span::raw(" ".repeat(padding + self.label_gap)));
        spans.push(Span::styled(option.to_string(), self.style));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn horizontal_labels_share_a_column_and_have_gaps() {
        let mut model = Model::new(["read-only", "read-write"]);
        model.selected = 1;
        let view = model.view().to_string();
        assert!(view.starts_with("◯"));
        let first = view.find("read-only").expect("first option");
        assert_eq!(UnicodeWidthStr::width(&view[..first]), 4);
        let second = view.find("read-write").expect("second option");
        assert!(second > first);
        let after_first = &view[first + "read-only".len()..];
        let second_symbol = after_first.find('⦿').expect("second symbol");
        let between = &after_first[..second_symbol];
        assert!(!between.is_empty());
        assert!(between.chars().all(|character| character == ' '));
        assert!(UnicodeWidthStr::width(between) >= 4);
    }

    #[test]
    fn selection_moves_and_wraps() {
        let mut model = Model::new(["a", "b", "c"]);
        assert_eq!(model.selected, 0);
        model.select_next();
        assert_eq!(model.selected, 1);
        model.select_previous();
        assert_eq!(model.selected, 0);
        model.select_previous();
        assert_eq!(model.selected, 2);
        assert!(model.handle_key(&KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)));
        assert_eq!(model.selected, 1);
        assert!(model.handle_key(&KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)));
        assert_eq!(model.selected, 2);
    }

    #[test]
    fn vertical_lines_and_render() {
        let mut model = Model::new(["one", "two"]);
        model.orientation = Orientation::Vertical;
        let lines = model.lines();
        assert_eq!(lines.len(), 2);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 2));
        model.render(buffer.area, &mut buffer);
        let text = (0..2)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        assert!(text[0].contains("one"), "{text:?}");
        assert!(text[1].contains("two"), "{text:?}");
        assert!(text[0].starts_with("⦿"), "{text:?}");
        assert!(text[1].starts_with("◯"), "{text:?}");
    }
}
