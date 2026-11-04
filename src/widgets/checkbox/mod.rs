//! Checkbox widget.

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// A single checkbox with a fixed-width symbol slot.
#[derive(Clone, Debug)]
pub struct Model {
    pub checked: bool,
    pub label: String,
    pub description: String,
    pub checked_symbol: String,
    pub unchecked_symbol: String,
    /// Total cell width reserved for the symbol, including the glyph.
    pub symbol_width: usize,
    /// Additional cells between the symbol slot and the label.
    pub label_gap: usize,
    /// Additional cells between the label and the description.
    pub description_gap: usize,
    pub style: Style,
    pub description_style: Style,
    pub checked_style: Style,
    pub unchecked_style: Style,
}

impl Default for Model {
    fn default() -> Self {
        Self::new("")
    }
}

impl Model {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            checked: false,
            label: label.into(),
            description: String::new(),
            checked_symbol: "▣".to_string(),
            unchecked_symbol: "☐".to_string(),
            symbol_width: 2,
            label_gap: 2,
            description_gap: 2,
            style: Style::default(),
            description_style: Style::default(),
            checked_style: Style::default(),
            unchecked_style: Style::default(),
        }
    }

    /// Adds secondary text after the primary label.
    ///
    /// When `label` is empty the description is rendered as the label with
    /// `style`; when `label` is present it uses `description_style`.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    /// Toggle with Space or Enter. Returns whether the key was consumed.
    pub fn handle_key(&mut self, event: &KeyEvent) -> bool {
        if matches!(event.code, KeyCode::Char(' ') | KeyCode::Enter) {
            self.toggle();
            true
        } else {
            false
        }
    }

    pub fn view(&self) -> Line<'static> {
        let (symbol, symbol_style) = if self.checked {
            (self.checked_symbol.as_str(), self.checked_style)
        } else {
            (self.unchecked_symbol.as_str(), self.unchecked_style)
        };
        let symbol_width = UnicodeWidthStr::width(symbol);
        let padding = self.symbol_width.saturating_sub(symbol_width);
        let mut spans = vec![Span::styled(symbol.to_string(), symbol_style)];
        spans.push(Span::raw(" ".repeat(padding + self.label_gap)));
        if self.label.is_empty() {
            if !self.description.is_empty() {
                spans.push(Span::styled(self.description.clone(), self.style));
            }
        } else {
            spans.push(Span::styled(self.label.clone(), self.style));
            if !self.description.is_empty() {
                spans.push(Span::raw(" ".repeat(self.description_gap)));
                spans.push(Span::styled(
                    self.description.clone(),
                    self.description_style,
                ));
            }
        }
        Line::from(spans)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        buf.set_line(area.x, area.y, &self.view(), area.width);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn checked_and_unchecked_labels_share_a_column() {
        let mut model = Model::new("optional");
        let unchecked = model.view().to_string();
        assert!(unchecked.starts_with("☐"));
        let start = unchecked.find("optional").expect("label");
        assert_eq!(UnicodeWidthStr::width(&unchecked[..start]), 4);

        model.checked = true;
        let checked = model.view().to_string();
        assert!(checked.starts_with("▣"));
        let start = checked.find("optional").expect("label");
        assert_eq!(UnicodeWidthStr::width(&checked[..start]), 4);
    }

    #[test]
    fn space_and_enter_toggle() {
        let mut model = Model::new("optional");
        assert!(model.handle_key(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)));
        assert!(model.checked);
        assert!(model.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
        assert!(!model.checked);
        assert!(!model.handle_key(&KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)));
    }

    #[test]
    fn render_writes_the_checkbox() {
        let mut model = Model::new("dynamic");
        model.checked = true;
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 1));
        model.render(buffer.area, &mut buffer);
        let text = (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<String>();
        assert!(text.contains("▣"), "{text}");
        assert!(text.contains("dynamic"), "{text}");
    }

    #[test]
    fn custom_symbols_and_gaps_are_respected() {
        let mut model = Model::new("label");
        model.checked_symbol = "X".to_string();
        model.unchecked_symbol = ".".to_string();
        model.symbol_width = 3;
        model.label_gap = 1;
        let start = model.view().to_string().find("label").expect("label");
        assert_eq!(UnicodeWidthStr::width(&model.view().to_string()[..start]), 4);
        model.checked = true;
        let start = model.view().to_string().find("label").expect("label");
        assert_eq!(UnicodeWidthStr::width(&model.view().to_string()[..start]), 4);
    }

    #[test]
    fn description_becomes_the_label_when_label_is_empty() {
        let mut model = Model::new("");
        model.style = Style::default().fg(ratatui::style::Color::Red);
        model.description_style = Style::default().fg(ratatui::style::Color::Blue);
        model = model.with_description("Description");
        let view = model.view();
        let text = view.to_string();
        assert!(text.contains("Description"), "{text}");
        assert_eq!(
            view.spans.last().unwrap().style.fg,
            Some(ratatui::style::Color::Red)
        );
        let start = text.find("Description").expect("description");
        assert_eq!(UnicodeWidthStr::width(&text[..start]), 4);
    }

    #[test]
    fn description_uses_secondary_style_when_label_exists() {
        let mut model = Model::new("label");
        model.style = Style::default().fg(ratatui::style::Color::Red);
        model.description_style = Style::default().fg(ratatui::style::Color::Blue);
        model = model.with_description("Description");
        let view = model.view();
        assert_eq!(view.spans.last().unwrap().style.fg, Some(ratatui::style::Color::Blue));
        let text = view.to_string();
        assert!(text.contains("label"), "{text}");
        assert!(text.contains("Description"), "{text}");
    }
}
