use std::collections::BTreeMap;

use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::{Line, Span}};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::widgets::key::{self, Binding};

const DEFAULT_HORIZONTAL_STEP: usize = 6;

pub type Option = Box<dyn Fn(&mut Model)>;

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub page_down: Binding,
    pub page_up: Binding,
    pub half_page_up: Binding,
    pub half_page_down: Binding,
    pub down: Binding,
    pub up: Binding,
    pub left: Binding,
    pub right: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            page_down: Binding::new([
                key::with_keys(&["pgdown", "space", "f"]),
                key::with_help("f/pgdn", "page down"),
            ]),
            page_up: Binding::new([
                key::with_keys(&["pgup", "b"]),
                key::with_help("b/pgup", "page up"),
            ]),
            half_page_up: Binding::new([
                key::with_keys(&["u", "ctrl+u"]),
                key::with_help("u", "½ page up"),
            ]),
            half_page_down: Binding::new([
                key::with_keys(&["d", "ctrl+d"]),
                key::with_help("d", "½ page down"),
            ]),
            up: Binding::new([
                key::with_keys(&["up", "k"]),
                key::with_help("↑/k", "up"),
            ]),
            down: Binding::new([
                key::with_keys(&["down", "j"]),
                key::with_help("↓/j", "down"),
            ]),
            left: Binding::new([
                key::with_keys(&["left", "h"]),
                key::with_help("←/h", "move left"),
            ]),
            right: Binding::new([
                key::with_keys(&["right", "l"]),
                key::with_help("→/l", "move right"),
            ]),
        }
    }
}

pub type GutterFunc = Box<dyn Fn(GutterContext) -> String>;

#[derive(Clone, Copy, Debug, Default)]
pub struct GutterContext {
    pub index: usize,
    pub total_lines: usize,
    pub soft: bool,
}

pub struct Model {
    width: usize,
    height: usize,
    pub key_map: KeyMap,
    pub soft_wrap: bool,
    pub fill_height: bool,
    pub mouse_wheel_enabled: bool,
    pub mouse_wheel_delta: usize,
    y_offset: usize,
    x_offset: usize,
    horizontal_step: usize,
    pub y_position: usize,
    pub style: Style,
    lines: Vec<String>,
    longest_line_width: usize,
    pub highlight_style: Style,
    pub selected_highlight_style: Style,
    pub style_line_func: std::option::Option<Box<dyn Fn(usize) -> Style>>,
    pub left_gutter_func: std::option::Option<GutterFunc>,
    highlights: Vec<HighlightInfo>,
    hi_idx: isize,
}

impl Default for Model {
    fn default() -> Self {
        Self::new([])
    }
}

impl Model {
    pub fn new(opts: impl IntoIterator<Item = Option>) -> Self {
        let mut model = Self {
            width: 0,
            height: 0,
            key_map: KeyMap::default(),
            soft_wrap: false,
            fill_height: false,
            mouse_wheel_enabled: true,
            mouse_wheel_delta: 3,
            y_offset: 0,
            x_offset: 0,
            horizontal_step: DEFAULT_HORIZONTAL_STEP,
            y_position: 0,
            style: Style::default(),
            lines: Vec::new(),
            longest_line_width: 0,
            highlight_style: Style::default(),
            selected_highlight_style: Style::default(),
            style_line_func: None,
            left_gutter_func: None,
            highlights: Vec::new(),
            hi_idx: -1,
        };
        for opt in opts {
            opt(&mut model);
        }
        model
    }

    pub fn height(&self) -> usize { self.height }
    pub fn set_height(&mut self, height: usize) { self.height = height; }
    pub fn width(&self) -> usize { self.width }
    pub fn set_width(&mut self, width: usize) { self.width = width; }
    pub fn y_offset(&self) -> usize { self.y_offset }
    pub fn x_offset(&self) -> usize { self.x_offset }

    pub fn at_top(&self) -> bool { self.y_offset == 0 }
    pub fn at_bottom(&self) -> bool { self.y_offset >= self.max_y_offset() }
    pub fn past_bottom(&self) -> bool { self.y_offset > self.max_y_offset() }

    pub fn scroll_percent(&self) -> f64 {
        let total = self.total_visual_lines();
        if self.height >= total || total == 0 {
            return 1.0;
        }
        let y = self.y_offset as f64;
        let h = self.height as f64;
        let t = total as f64;
        (y / (t - h)).clamp(0.0, 1.0)
    }

    pub fn horizontal_scroll_percent(&self) -> f64 {
        let max_width = self.max_width();
        if max_width == 0 || self.x_offset >= self.longest_line_width.saturating_sub(max_width) {
            return 1.0;
        }
        let x = self.x_offset as f64;
        let w = max_width as f64;
        let t = self.longest_line_width as f64;
        (x / (t - w)).clamp(0.0, 1.0)
    }

    pub fn set_content(&mut self, content: &str) {
        self.set_content_lines(content.replace("\r\n", "\n").split('\n').map(ToString::to_string).collect());
    }

    pub fn set_content_lines(&mut self, lines: Vec<String>) {
        self.lines = if lines.len() == 1 && lines.first().is_some_and(|line| line.is_empty()) {
            Vec::new()
        } else {
            lines
        };
        self.longest_line_width = self
            .lines
            .iter()
            .map(|line| UnicodeWidthStr::width(line.as_str()))
            .max()
            .unwrap_or(0);
        self.clear_highlights();
        if self.y_offset > self.max_y_offset() {
            self.goto_bottom();
        }
        if self.x_offset > self.max_x_offset() {
            self.x_offset = self.max_x_offset();
        }
    }

    pub fn content(&self) -> String {
        self.lines.join("\n")
    }

    pub fn visible_lines(&self) -> Vec<String> {
        self.visible_render_rows().into_iter().map(|row| row.text).collect()
    }

    pub fn visible_rows(&self) -> Vec<Line<'static>> {
        self.visible_render_rows().into_iter().map(|row| self.render_row(row)).collect()
    }

    fn visible_render_rows(&self) -> Vec<DisplayRow> {
        if self.lines.is_empty() || self.height == 0 || self.max_width() == 0 {
            return Vec::new();
        }
        let raw = if self.soft_wrap {
            self.visible_soft_wrapped_rows()
        } else {
            self.visible_hard_rows()
        };
        if self.fill_height && raw.len() < self.height {
            let mut filled = raw;
            let width = self.max_width();
            while filled.len() < self.height {
                filled.push(DisplayRow { text: String::new(), line_index: self.lines.len(), start_col: 0, end_col: width, soft: false });
            }
            return filled;
        }
        raw
    }

    pub fn set_y_offset(&mut self, offset: usize) {
        self.y_offset = offset.min(self.max_y_offset());
    }

    pub fn ensure_visible(&mut self, line: usize, col_start: usize, col_end: usize) {
        if line < self.y_offset {
            self.y_offset = line;
        } else if line >= self.y_offset + self.height && self.height > 0 {
            self.y_offset = line.saturating_sub(self.height - 1);
        }
        if !self.soft_wrap {
            if col_start < self.x_offset {
                self.x_offset = col_start;
            } else if self.max_width() > 0 && col_end > self.x_offset + self.max_width() {
                self.x_offset = col_start.saturating_sub(self.horizontal_step);
            }
        }
    }

    pub fn page_down(&mut self) { self.scroll_down(self.height); }
    pub fn page_up(&mut self) { self.scroll_up(self.height); }
    pub fn half_page_down(&mut self) { self.scroll_down(self.height / 2); }
    pub fn half_page_up(&mut self) { self.scroll_up(self.height / 2); }
    pub fn scroll_down(&mut self, n: usize) { self.y_offset = (self.y_offset + n).min(self.max_y_offset()); }
    pub fn scroll_up(&mut self, n: usize) { self.y_offset = self.y_offset.saturating_sub(n); }
    pub fn set_horizontal_step(&mut self, n: isize) { self.horizontal_step = n.max(0) as usize; }
    pub fn set_x_offset(&mut self, n: usize) { self.x_offset = n.min(self.max_x_offset()); }
    pub fn scroll_left(&mut self, n: usize) { self.x_offset = self.x_offset.saturating_sub(n); }
    pub fn scroll_right(&mut self, n: usize) { self.x_offset = (self.x_offset + n).min(self.max_x_offset()); }
    pub fn total_line_count(&self) -> usize { self.total_visual_lines() }
    pub fn visible_line_count(&self) -> usize { self.visible_render_rows().len() }
    pub fn goto_top(&mut self) -> Vec<String> { self.y_offset = 0; self.visible_lines() }
    pub fn goto_bottom(&mut self) -> Vec<String> { self.y_offset = self.max_y_offset(); self.hi_idx = self.find_nearest_match(); self.visible_lines() }
    pub fn set_highlights(&mut self, matches: Vec<[usize; 2]>) {
        if matches.is_empty() || self.lines.is_empty() {
            return;
        }
        self.highlights = parse_matches(&self.content(), &matches);
        self.hi_idx = self.find_nearest_match();
        self.show_highlight();
    }
    pub fn clear_highlights(&mut self) { self.highlights.clear(); self.hi_idx = -1; }
    pub fn highlight_next(&mut self) {
        if self.highlights.is_empty() { return; }
        self.hi_idx = (self.hi_idx + 1).rem_euclid(self.highlights.len() as isize);
        self.show_highlight();
    }
    pub fn highlight_previous(&mut self) {
        if self.highlights.is_empty() { return; }
        self.hi_idx = (self.hi_idx - 1).rem_euclid(self.highlights.len() as isize);
        self.show_highlight();
    }

    pub fn handle_key(&mut self, event: &crossterm::event::KeyEvent) {
        if key::matches(event, [&self.key_map.page_down]) {
            self.page_down();
        } else if key::matches(event, [&self.key_map.page_up]) {
            self.page_up();
        } else if key::matches(event, [&self.key_map.half_page_down]) {
            self.half_page_down();
        } else if key::matches(event, [&self.key_map.half_page_up]) {
            self.half_page_up();
        } else if key::matches(event, [&self.key_map.down]) {
            self.scroll_down(1);
        } else if key::matches(event, [&self.key_map.up]) {
            self.scroll_up(1);
        } else if key::matches(event, [&self.key_map.left]) {
            self.scroll_left(self.horizontal_step);
        } else if key::matches(event, [&self.key_map.right]) {
            self.scroll_right(self.horizontal_step);
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (row, line) in self.visible_rows().into_iter().enumerate() {
            if row >= area.height as usize {
                break;
            }
            buf.set_line(area.x, area.y + row as u16, &line, area.width);
        }
    }

    fn max_y_offset(&self) -> usize {
        self.total_visual_lines().saturating_sub(self.height.max(1))
    }

    fn max_x_offset(&self) -> usize {
        self.longest_line_width.saturating_sub(self.max_width().max(1))
    }

    fn max_width(&self) -> usize {
        self.width.saturating_sub(self.gutter_width())
    }

    fn gutter_width(&self) -> usize {
        self.left_gutter_func
            .as_ref()
            .map(|f| UnicodeWidthStr::width(f(GutterContext::default()).as_str()))
            .unwrap_or(0)
    }

    fn total_visual_lines(&self) -> usize {
        if !self.soft_wrap {
            return self.lines.len();
        }
        let max_width = self.max_width().max(1);
        self.lines.iter().map(|line| display_row_count(line, max_width)).sum()
    }

    fn visible_hard_rows(&self) -> Vec<DisplayRow> {
        self.lines
            .iter()
            .enumerate()
            .skip(self.y_offset)
            .take(self.height)
            .map(|(line_index, line)| DisplayRow {
                text: slice_display_width(line, self.x_offset, self.max_width()),
                line_index,
                start_col: self.x_offset,
                end_col: self.x_offset + self.max_width(),
                soft: false,
            })
            .collect()
    }

    fn visible_soft_wrapped_rows(&self) -> Vec<DisplayRow> {
        let mut out = Vec::new();
        let max_width = self.max_width().max(1);
        for (line_index, line) in self.lines.iter().enumerate() {
            let wrapped = wrap_line(line, max_width);
            for (part, start, end) in wrapped {
                out.push(DisplayRow { text: part, line_index, start_col: start, end_col: end, soft: start > 0 });
            }
        }
        out.into_iter().skip(self.y_offset).take(self.height).collect()
    }

    fn render_row(&self, row: DisplayRow) -> Line<'static> {
        let mut spans = Vec::new();
        if let Some(gutter) = &self.left_gutter_func {
            spans.push(Span::raw(gutter(GutterContext { index: row.line_index, total_lines: self.total_visual_lines(), soft: row.soft })));
        }

        let base_style = self.style_line_func.as_ref().map(|f| f(row.line_index)).unwrap_or(self.style);
        let text_spans = highlight_spans(&row.text, row.start_col, row.end_col, row.line_index, &self.highlights, self.hi_idx, base_style, self.highlight_style, self.selected_highlight_style);
        spans.extend(text_spans);
        Line::from(spans)
    }

    fn show_highlight(&mut self) {
        if self.hi_idx < 0 { return; }
        if let Some(hi) = self.highlights.get(self.hi_idx as usize) {
            let (line, col_start, col_end) = hi.coords();
            self.ensure_visible(line, col_start, col_end);
        }
    }

    fn find_nearest_match(&self) -> isize {
        for (i, m) in self.highlights.iter().enumerate() {
            if m.line_start >= self.y_offset {
                return i as isize;
            }
        }
        -1
    }
}

pub fn with_width(width: usize) -> Option { Box::new(move |m| m.width = width) }
pub fn with_height(height: usize) -> Option { Box::new(move |m| m.height = height) }

fn wrap_line(line: &str, width: usize) -> Vec<(String, usize, usize)> {
    if width == 0 || line.is_empty() {
        return vec![(String::new(), 0, 0)];
    }
    let mut out = Vec::new();
    let mut segment = String::new();
    let mut used = 0usize;
    let mut start_col = 0usize;
    let mut col = 0usize;
    for ch in line.chars() {
        let cw = ch.width().unwrap_or(0).max(1);
        if used > 0 && used + cw > width {
            out.push((segment.clone(), start_col, col));
            segment.clear();
            start_col = col;
            used = 0;
        }
        segment.push(ch);
        used += cw;
        col += cw;
    }
    if !segment.is_empty() {
        out.push((segment, start_col, col));
    }
    if out.is_empty() { vec![(String::new(), 0, 0)] } else { out }
}

fn slice_display_width(line: &str, offset: usize, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut col = 0usize;
    let mut used = 0usize;
    for ch in line.chars() {
        let cw = ch.width().unwrap_or(0).max(1);
        let end = col + cw;
        if end <= offset {
            col = end;
            continue;
        }
        if used + cw > width {
            break;
        }
        out.push(ch);
        used += cw;
        col = end;
    }
    out
}

fn display_row_count(line: &str, width: usize) -> usize {
    let width = width.max(1);
    let line_width = UnicodeWidthStr::width(line);
    (line_width.max(1) + width - 1) / width
}

#[derive(Clone, Debug)]
struct DisplayRow {
    text: String,
    line_index: usize,
    start_col: usize,
    end_col: usize,
    soft: bool,
}

#[derive(Clone, Debug)]
struct HighlightInfo {
    line_start: usize,
    line_end: usize,
    lines: BTreeMap<usize, [usize; 2]>,
}

impl HighlightInfo {
    fn coords(&self) -> (usize, usize, usize) {
        for (line, range) in &self.lines {
            return (*line, range[0], range[1]);
        }
        (self.line_start, 0, 0)
    }
}

fn parse_matches(content: &str, matches: &[[usize; 2]]) -> Vec<HighlightInfo> {
    let mut out = Vec::new();
    for [start, end] in matches.iter().copied() {
        let mut byte = 0usize;
        let mut line = 0usize;
        let mut col = 0usize;
        let mut current_line = None::<usize>;
        let mut current_start = 0usize;
        let mut info = HighlightInfo { line_start: 0, line_end: 0, lines: BTreeMap::new() };
        let mut seen = false;
        for ch in content.chars() {
            let len = ch.len_utf8();
            let next = byte + len;
            let cw = ch.width().unwrap_or(0).max(1);
            let in_match = next > start && byte < end;
            if in_match && !seen {
                info.line_start = line;
                info.line_end = line;
                current_line = Some(line);
                current_start = col;
                seen = true;
            }
            if ch == '\n' {
                if let Some(active) = current_line.take() {
                    info.lines.insert(active, [current_start, col + 1]);
                    info.line_end = active;
                }
                line += 1;
                col = 0;
                byte = next;
                continue;
            }
            if seen && !in_match {
                if let Some(active) = current_line.take() {
                    info.lines.insert(active, [current_start, col]);
                    info.line_end = active;
                }
            }
            if seen && in_match && current_line.is_none() {
                current_line = Some(line);
                current_start = col;
            }
            col += cw;
            byte = next;
        }
        if let Some(active) = current_line.take() {
            info.lines.insert(active, [current_start, col]);
            info.line_end = active;
        }
        if !info.lines.is_empty() {
            out.push(info);
        }
    }
    out
}

fn highlight_spans(
    text: &str,
    start_col: usize,
    end_col: usize,
    line_index: usize,
    highlights: &[HighlightInfo],
    selected_idx: isize,
    base: Style,
    highlight: Style,
    selected: Style,
) -> Vec<Span<'static>> {
    let mut marks: Vec<(usize, usize, Style)> = Vec::new();
    for (idx, hi) in highlights.iter().enumerate() {
        if let Some([hs, he]) = hi.lines.get(&line_index).copied() {
            let seg_start = hs.max(start_col).saturating_sub(start_col);
            let seg_end = he.min(end_col).saturating_sub(start_col);
            if seg_end > seg_start {
                let style = if idx as isize == selected_idx { selected } else { highlight };
                marks.push((seg_start, seg_end, style));
            }
        }
    }
    if marks.is_empty() {
        return vec![Span::styled(text.to_string(), base)];
    }

    let mut out = Vec::new();
    let mut plain = String::new();
    let mut styled = String::new();
    let mut active_style = base;
    let mut col = 0usize;
    for ch in text.chars() {
        let cw = ch.width().unwrap_or(0).max(1);
        let next = col + cw;
        let style = marks.iter().find(|(s, e, _)| col >= *s && next <= *e).map(|(_, _, st)| *st).unwrap_or(base);
        if style != active_style {
            if !styled.is_empty() {
                out.push(Span::styled(std::mem::take(&mut styled), active_style));
            }
            if !plain.is_empty() {
                out.push(Span::styled(std::mem::take(&mut plain), active_style));
            }
            active_style = style;
        }
        if style == base {
            plain.push(ch);
        } else {
            styled.push(ch);
        }
        col = next;
    }
    if !plain.is_empty() {
        out.push(Span::styled(plain, base));
    }
    if !styled.is_empty() {
        out.push(Span::styled(styled, active_style));
    }
    out
}
