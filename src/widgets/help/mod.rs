use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::widgets::key::{Binding, Help as KeyHelp};

pub trait KeyMap {
    fn short_help(&self) -> Vec<Binding>;
    fn full_help(&self) -> Vec<Vec<Binding>>;
}

#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub ellipsis: Style,
    pub short_key: Style,
    pub short_desc: Style,
    pub short_separator: Style,
    pub full_key: Style,
    pub full_desc: Style,
    pub full_separator: Style,
}

#[derive(Clone, Debug)]
pub struct Model {
    pub show_all: bool,
    pub short_separator: String,
    pub full_separator: String,
    pub ellipsis: String,
    pub styles: Styles,
    width: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            show_all: false,
            short_separator: " • ".to_string(),
            full_separator: "    ".to_string(),
            ellipsis: "…".to_string(),
            styles: Styles::default(),
            width: 0,
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn view<K: KeyMap>(&self, keymap: &K) -> Vec<Line<'static>> {
        if self.show_all {
            self.full_help_view(&keymap.full_help())
        } else {
            vec![self.short_help_view(&keymap.short_help())]
        }
    }

    pub fn render<K: KeyMap>(&self, area: Rect, buf: &mut Buffer, keymap: &K) {
        for (row, line) in self.view(keymap).into_iter().enumerate() {
            if row >= area.height as usize {
                break;
            }
            buf.set_line(area.x, area.y + row as u16, &line, area.width);
        }
    }

    pub fn short_help_view(&self, bindings: &[Binding]) -> Line<'static> {
        let mut spans = Vec::new();
        let mut total_width = 0usize;
        let separator_width = UnicodeWidthStr::width(self.short_separator.as_str());

        for binding in bindings.iter().filter(|binding| binding.enabled()) {
            let KeyHelp { key, desc } = binding.help();
            let item_width = UnicodeWidthStr::width(key.as_str()) + 1 + UnicodeWidthStr::width(desc.as_str());
            let sep_width = if spans.is_empty() { 0 } else { separator_width };
            if let Some(tail) = self.truncation_tail(total_width, sep_width + item_width) {
                if !tail.is_empty() {
                    spans.push(Span::styled(tail, self.styles.ellipsis));
                }
                break;
            }
            if !spans.is_empty() {
                spans.push(Span::styled(self.short_separator.clone(), self.styles.short_separator));
            }
            spans.push(Span::styled(key.clone(), self.styles.short_key));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(desc.clone(), self.styles.short_desc));
            total_width += sep_width + item_width;
        }

        Line::from(spans)
    }

    pub fn full_help_view(&self, groups: &[Vec<Binding>]) -> Vec<Line<'static>> {
        let rendered: Vec<Vec<(String, String)>> = groups
            .iter()
            .map(|group| {
                group
                    .iter()
                    .filter(|binding| binding.enabled())
                    .map(|binding| (binding.help().key.clone(), binding.help().desc.clone()))
                    .collect::<Vec<_>>()
            })
            .filter(|group| !group.is_empty())
            .collect();

        if rendered.is_empty() {
            return Vec::new();
        }

        let widths: Vec<usize> = rendered
            .iter()
            .map(|group| {
                group
                    .iter()
                    .map(|(key, desc)| UnicodeWidthStr::width(key.as_str()) + 1 + UnicodeWidthStr::width(desc.as_str()))
                    .max()
                    .unwrap_or(0)
            })
            .collect();

        let mut kept = 0usize;
        let mut total_width = 0usize;
        let full_sep_width = UnicodeWidthStr::width(self.full_separator.as_str());
        for width in &widths {
            let sep = if kept == 0 { 0 } else { full_sep_width };
            if self.truncation_tail(total_width, sep + *width).is_some() {
                break;
            }
            total_width += sep + *width;
            kept += 1;
        }

        let kept = kept.max(1).min(rendered.len());
        let max_rows = rendered.iter().take(kept).map(Vec::len).max().unwrap_or(0);
        let mut lines = Vec::with_capacity(max_rows);

        for row in 0..max_rows {
            let mut spans = Vec::new();
            for col in 0..kept {
                if col > 0 {
                    spans.push(Span::styled(self.full_separator.clone(), self.styles.full_separator));
                }
                let width = widths[col];
                if let Some((key, desc)) = rendered[col].get(row) {
                    let key_width = UnicodeWidthStr::width(key.as_str());
                    spans.push(Span::styled(key.clone(), self.styles.full_key));
                    spans.push(Span::raw(" "));
                    spans.push(Span::styled(desc.clone(), self.styles.full_desc));
                    let used = key_width + 1 + UnicodeWidthStr::width(desc.as_str());
                    if width > used {
                        spans.push(Span::raw(" ".repeat(width - used)));
                    }
                } else {
                    spans.push(Span::raw(" ".repeat(width)));
                }
            }

            if kept < rendered.len() && row == 0 {
                spans.push(Span::styled(format!(" {}", self.ellipsis), self.styles.ellipsis));
            }
            lines.push(Line::from(spans));
        }

        lines
    }

    fn truncation_tail(&self, total_width: usize, next_width: usize) -> Option<String> {
        if self.width > 0 && total_width + next_width > self.width {
            let tail = format!(" {}", self.ellipsis);
            let tail_width = UnicodeWidthStr::width(tail.as_str());
            if total_width + tail_width < self.width {
                return Some(tail);
            }
            return Some(String::new());
        }
        None
    }
}
