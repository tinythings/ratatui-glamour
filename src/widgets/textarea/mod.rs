use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::widgets::{key::{self, Binding}, viewport};

const DEFAULT_HEIGHT: usize = 6;
const DEFAULT_WIDTH: usize = 40;
const DEFAULT_MAX_HEIGHT: usize = 99;
const DEFAULT_MAX_WIDTH: usize = 500;

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub character_backward: Binding,
    pub character_forward: Binding,
    pub delete_after_cursor: Binding,
    pub delete_before_cursor: Binding,
    pub delete_character_backward: Binding,
    pub delete_character_forward: Binding,
    pub delete_word_backward: Binding,
    pub delete_word_forward: Binding,
    pub insert_newline: Binding,
    pub line_end: Binding,
    pub line_next: Binding,
    pub line_previous: Binding,
    pub line_start: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub paste: Binding,
    pub word_backward: Binding,
    pub word_forward: Binding,
    pub input_begin: Binding,
    pub input_end: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            character_forward: Binding::new([key::with_keys(&["right", "ctrl+f"])]),
            character_backward: Binding::new([key::with_keys(&["left", "ctrl+b"])]),
            word_forward: Binding::new([key::with_keys(&["alt+right", "alt+f"])]),
            word_backward: Binding::new([key::with_keys(&["alt+left", "alt+b"])]),
            line_next: Binding::new([key::with_keys(&["down", "ctrl+n"])]),
            line_previous: Binding::new([key::with_keys(&["up", "ctrl+p"])]),
            delete_word_backward: Binding::new([key::with_keys(&["alt+backspace", "ctrl+w"])]),
            delete_word_forward: Binding::new([key::with_keys(&["alt+delete", "alt+d"])]),
            delete_after_cursor: Binding::new([key::with_keys(&["ctrl+k"])]),
            delete_before_cursor: Binding::new([key::with_keys(&["ctrl+u"])]),
            insert_newline: Binding::new([key::with_keys(&["enter", "ctrl+m"])]),
            delete_character_backward: Binding::new([key::with_keys(&["backspace", "ctrl+h"])]),
            delete_character_forward: Binding::new([key::with_keys(&["delete", "ctrl+d"])]),
            line_start: Binding::new([key::with_keys(&["home", "ctrl+a"])]),
            line_end: Binding::new([key::with_keys(&["end", "ctrl+e"])]),
            page_up: Binding::new([key::with_keys(&["pgup"])]),
            page_down: Binding::new([key::with_keys(&["pgdown"])]),
            paste: Binding::new([key::with_keys(&["ctrl+v"])]),
            input_begin: Binding::new([key::with_keys(&["alt+<", "ctrl+home"])]),
            input_end: Binding::new([key::with_keys(&["alt+>", "ctrl+end"])]),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyleState {
    pub base: Style,
    pub text: Style,
    pub line_number: Style,
    pub cursor_line_number: Style,
    pub cursor_line: Style,
    pub end_of_buffer: Style,
    pub placeholder: Style,
    pub prompt: Style,
}

#[derive(Clone, Debug)]
pub struct CursorStyle {
    pub style: Style,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self { style: Style::default().add_modifier(Modifier::REVERSED) }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub focused: StyleState,
    pub blurred: StyleState,
    pub cursor: CursorStyle,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PromptInfo {
    pub line_number: usize,
    pub focused: bool,
}

pub type PromptFunc = Box<dyn Fn(PromptInfo) -> String>;

pub struct Model {
    pub err: Option<String>,
    pub prompt: String,
    pub placeholder: String,
    pub show_line_numbers: bool,
    pub end_of_buffer_character: char,
    pub key_map: KeyMap,
    pub char_limit: usize,
    pub max_height: usize,
    pub max_width: usize,
    pub dynamic_height: bool,
    pub min_height: usize,
    pub max_content_height: usize,
    styles: Styles,
    prompt_func: Option<PromptFunc>,
    prompt_width: usize,
    width: usize,
    height: usize,
    value: Vec<Vec<char>>,
    focus: bool,
    col: usize,
    row: usize,
    last_char_offset: usize,
    pub viewport: viewport::Model,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        let mut viewport = viewport::Model::new([]);
        viewport.soft_wrap = true;
        let mut model = Self {
            err: None,
            prompt: "┃ ".to_string(),
            placeholder: String::new(),
            show_line_numbers: true,
            end_of_buffer_character: ' ',
            key_map: KeyMap::default(),
            char_limit: 0,
            max_height: DEFAULT_MAX_HEIGHT,
            max_width: DEFAULT_MAX_WIDTH,
            dynamic_height: false,
            min_height: 1,
            max_content_height: 0,
            styles: Styles::default(),
            prompt_func: None,
            prompt_width: 0,
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            value: vec![Vec::new()],
            focus: false,
            col: 0,
            row: 0,
            last_char_offset: 0,
            viewport,
        };
        model.set_width(DEFAULT_WIDTH);
        model.set_height(DEFAULT_HEIGHT);
        model.sync_viewport();
        model
    }

    pub fn styles(&self) -> &Styles { &self.styles }
    pub fn set_styles(&mut self, styles: Styles) { self.styles = styles; self.sync_viewport(); }
    pub fn focused(&self) -> bool { self.focus }
    pub fn focus(&mut self) { self.focus = true; }
    pub fn blur(&mut self) { self.focus = false; }
    pub fn line_count(&self) -> usize { self.value.len() }
    pub fn line(&self) -> usize { self.row }
    pub fn column(&self) -> usize { self.col }
    pub fn scroll_y_offset(&self) -> usize { self.viewport.y_offset() }
    pub fn scroll_percent(&self) -> f64 { self.viewport.scroll_percent() }

    pub fn set_value(&mut self, s: &str) {
        self.reset();
        self.insert_string(s);
        self.recalculate_height();
    }

    pub fn insert_string(&mut self, s: &str) {
        self.insert_runes(&sanitize_multiline(s).chars().collect::<Vec<_>>());
        self.recalculate_height();
    }

    pub fn insert_rune(&mut self, r: char) {
        self.insert_runes(&[r]);
        self.recalculate_height();
    }

    pub fn value(&self) -> String {
        self.value.iter().map(|line| line.iter().collect::<String>()).collect::<Vec<_>>().join("\n")
    }

    pub fn length(&self) -> usize {
        self.value.iter().map(Vec::len).sum::<usize>() + self.value.len().saturating_sub(1)
    }

    pub fn cursor_down(&mut self) { self.set_cursor_line_relative(1); }
    pub fn cursor_up(&mut self) { self.set_cursor_line_relative(-1); }
    pub fn set_cursor_column(&mut self, col: usize) { self.col = col.min(self.value[self.row].len()); self.last_char_offset = 0; self.reposition_view(); }
    pub fn cursor_start(&mut self) { self.set_cursor_column(0); }
    pub fn cursor_end(&mut self) { self.set_cursor_column(self.value[self.row].len()); }
    pub fn move_to_begin(&mut self) { self.row = 0; self.cursor_start(); }
    pub fn move_to_end(&mut self) { self.row = self.value.len().saturating_sub(1); self.cursor_end(); }
    pub fn reset(&mut self) { self.value = vec![Vec::new()]; self.col = 0; self.row = 0; self.viewport.goto_top(); self.recalculate_height(); }
    pub fn set_width(&mut self, width: usize) { self.width = width.min(self.max_width); self.viewport.set_width(self.content_width()); self.prompt_width = self.compute_prompt_width(); self.recalculate_height(); }
    pub fn set_height(&mut self, height: usize) { self.height = height.max(1).min(self.max_height); self.viewport.set_height(self.content_height()); self.recalculate_height(); }
    pub fn set_prompt_func(&mut self, prompt_width: usize, func: PromptFunc) { self.prompt_width = prompt_width; self.prompt_func = Some(func); self.sync_viewport(); }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        if !self.focus { return; }
        match event.code {
            _ if key::matches(event, [&self.key_map.delete_word_backward]) => self.delete_word_backward(),
            _ if key::matches(event, [&self.key_map.delete_character_backward]) => self.delete_character_backward(),
            _ if key::matches(event, [&self.key_map.word_backward]) => self.word_backward(),
            _ if key::matches(event, [&self.key_map.character_backward]) => { if self.col > 0 { self.set_cursor_column(self.col - 1); } else if self.row > 0 { self.row -= 1; self.cursor_end(); } },
            _ if key::matches(event, [&self.key_map.word_forward]) => self.word_forward(),
            _ if key::matches(event, [&self.key_map.character_forward]) => { if self.col < self.value[self.row].len() { self.set_cursor_column(self.col + 1); } else if self.row + 1 < self.value.len() { self.row += 1; self.cursor_start(); } },
            _ if key::matches(event, [&self.key_map.line_start]) => self.cursor_start(),
            _ if key::matches(event, [&self.key_map.delete_character_forward]) => self.delete_character_forward(),
            _ if key::matches(event, [&self.key_map.line_end]) => self.cursor_end(),
            _ if key::matches(event, [&self.key_map.delete_after_cursor]) => self.delete_after_cursor(),
            _ if key::matches(event, [&self.key_map.delete_before_cursor]) => self.delete_before_cursor(),
            _ if key::matches(event, [&self.key_map.delete_word_forward]) => self.delete_word_forward(),
            _ if key::matches(event, [&self.key_map.insert_newline]) => self.insert_runes(&['\n']),
            _ if key::matches(event, [&self.key_map.line_next]) => self.cursor_down(),
            _ if key::matches(event, [&self.key_map.line_previous]) => self.cursor_up(),
            _ if key::matches(event, [&self.key_map.page_down]) => { self.viewport.page_down(); self.row = self.viewport.y_offset().min(self.value.len().saturating_sub(1)); self.col = self.col.min(self.value[self.row].len()); },
            _ if key::matches(event, [&self.key_map.page_up]) => { self.viewport.page_up(); self.row = self.viewport.y_offset().min(self.value.len().saturating_sub(1)); self.col = self.col.min(self.value[self.row].len()); },
            _ if key::matches(event, [&self.key_map.input_begin]) => self.move_to_begin(),
            _ if key::matches(event, [&self.key_map.input_end]) => self.move_to_end(),
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) && !event.modifiers.contains(KeyModifiers::ALT) => self.insert_runes(&[c]),
            _ => {}
        }
        self.recalculate_height();
    }

    pub fn line_info(&self) -> LineInfo {
        let line = &self.value[self.row];
        let width = self.content_width().max(1);
        let char_width = line.len();
        let height = (char_width.max(1) + width - 1) / width;
        let start_column = (self.col / width) * width;
        let column_offset = self.col.saturating_sub(start_column);
        let row_offset = self.col / width;
        LineInfo { width: line.len().min(width), char_width, height, start_column, column_offset, row_offset, char_offset: self.col }
    }

    pub fn view(&self) -> Vec<Line<'static>> {
        if self.value.len() == 1 && self.value[0].is_empty() && !self.placeholder.is_empty() {
            return vec![Line::from(vec![
                Span::styled(self.prompt_for_line(0), self.active_style().prompt),
                Span::styled(self.placeholder.clone(), self.active_style().placeholder),
            ])];
        }
        let active = self.active_style();
        let mut lines = Vec::new();
        for (idx, row) in self.value.iter().enumerate() {
            let mut spans = Vec::new();
            let prompt_style = if idx == self.row { active.cursor_line_number } else { active.line_number };
            let text_style = if idx == self.row { active.cursor_line } else { active.text };
            spans.push(Span::styled(self.prompt_for_line(idx), prompt_style));
            if idx == self.row {
                let before: String = row[..self.col].iter().collect();
                spans.push(Span::styled(before, text_style));
                let ch = row.get(self.col).copied().unwrap_or(self.end_of_buffer_character);
                spans.push(Span::styled(ch.to_string(), self.styles.cursor.style));
                let after: String = row.get(self.col + 1..).unwrap_or(&[]).iter().collect();
                spans.push(Span::styled(after, text_style));
            } else {
                spans.push(Span::styled(row.iter().collect::<String>(), text_style));
            }
            lines.push(Line::from(spans));
        }
        lines
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.view();
        for (i, line) in lines.into_iter().skip(self.viewport.y_offset()).take(area.height as usize).enumerate() {
            buf.set_line(area.x, area.y + i as u16, &line, area.width);
        }
    }

    fn insert_runes(&mut self, runes: &[char]) {
        let mut runes = runes.to_vec();
        if self.char_limit > 0 {
            let available = self.char_limit.saturating_sub(self.length());
            runes.truncate(available);
        }
        if runes.is_empty() { return; }
        let mut parts: Vec<Vec<char>> = vec![Vec::new()];
        for ch in runes {
            if ch == '\n' { parts.push(Vec::new()); } else { parts.last_mut().unwrap().push(ch); }
        }
        let tail = self.value[self.row][self.col..].to_vec();
        self.value[self.row].truncate(self.col);
        self.value[self.row].extend_from_slice(&parts[0]);
        self.col += parts[0].len();
        if parts.len() > 1 {
            let mut insert_at = self.row + 1;
            for line in parts.iter().skip(1) {
                self.value.insert(insert_at, line.clone());
                self.row = insert_at;
                self.col = line.len();
                insert_at += 1;
            }
        }
        self.value[self.row].extend_from_slice(&tail);
        self.reposition_view();
    }

    fn delete_character_backward(&mut self) {
        if self.col > 0 {
            self.value[self.row].remove(self.col - 1);
            self.col -= 1;
        } else if self.row > 0 {
            let current = self.value.remove(self.row);
            self.row -= 1;
            self.col = self.value[self.row].len();
            self.value[self.row].extend(current);
        }
        self.reposition_view();
    }

    fn delete_character_forward(&mut self) {
        if self.col < self.value[self.row].len() {
            self.value[self.row].remove(self.col);
        } else if self.row + 1 < self.value.len() {
            let next = self.value.remove(self.row + 1);
            self.value[self.row].extend(next);
        }
        self.reposition_view();
    }

    fn delete_before_cursor(&mut self) {
        self.value[self.row].drain(..self.col);
        self.col = 0;
        self.reposition_view();
    }

    fn delete_after_cursor(&mut self) {
        self.value[self.row].truncate(self.col);
        self.reposition_view();
    }

    fn delete_word_backward(&mut self) {
        let old_col = self.col;
        while self.col > 0 && self.value[self.row][self.col - 1].is_whitespace() { self.col -= 1; }
        while self.col > 0 && !self.value[self.row][self.col - 1].is_whitespace() { self.col -= 1; }
        self.value[self.row].drain(self.col..old_col);
        self.reposition_view();
    }

    fn delete_word_forward(&mut self) {
        let start = self.col;
        while self.col < self.value[self.row].len() && self.value[self.row][self.col].is_whitespace() { self.col += 1; }
        while self.col < self.value[self.row].len() && !self.value[self.row][self.col].is_whitespace() { self.col += 1; }
        self.value[self.row].drain(start..self.col);
        self.col = start;
        self.reposition_view();
    }

    fn word_backward(&mut self) {
        while self.col > 0 && self.value[self.row][self.col - 1].is_whitespace() { self.col -= 1; }
        while self.col > 0 && !self.value[self.row][self.col - 1].is_whitespace() { self.col -= 1; }
        self.reposition_view();
    }

    fn word_forward(&mut self) {
        while self.col < self.value[self.row].len() && self.value[self.row][self.col].is_whitespace() { self.col += 1; }
        while self.col < self.value[self.row].len() && !self.value[self.row][self.col].is_whitespace() { self.col += 1; }
        self.reposition_view();
    }

    fn set_cursor_line_relative(&mut self, delta: isize) {
        if delta == 0 { return; }
        let target = (self.row as isize + delta).clamp(0, self.value.len().saturating_sub(1) as isize) as usize;
        self.row = target;
        self.col = self.col.min(self.value[self.row].len());
        self.reposition_view();
    }

    fn reposition_view(&mut self) {
        self.sync_viewport();
        self.viewport.ensure_visible(self.row, 0, self.col + 1);
    }

    fn sync_viewport(&mut self) {
        self.viewport.set_width(self.content_width());
        self.viewport.set_height(self.content_height());
        self.viewport.set_content(&self.value.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n"));
    }

    fn recalculate_height(&mut self) {
        if self.dynamic_height {
            let visible = self.value.len().clamp(self.min_height.max(1), self.max_height.max(1));
            self.height = visible;
        }
        self.sync_viewport();
        self.reposition_view();
    }

    fn content_width(&self) -> usize {
        self.width.saturating_sub(self.compute_prompt_width()).max(1)
    }

    fn content_height(&self) -> usize { self.height.max(1) }

    fn compute_prompt_width(&self) -> usize {
        (0..self.value.len().max(1))
            .map(|i| self.prompt_for_line(i))
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
    }

    fn prompt_for_line(&self, line: usize) -> String {
        if let Some(f) = &self.prompt_func {
            return f(PromptInfo { line_number: line + 1, focused: self.focus });
        }
        if self.show_line_numbers {
            format!("{:>3} {}", line + 1, self.prompt)
        } else {
            self.prompt.clone()
        }
    }

    fn active_style(&self) -> &StyleState {
        if self.focus { &self.styles.focused } else { &self.styles.blurred }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LineInfo {
    pub width: usize,
    pub char_width: usize,
    pub height: usize,
    pub start_column: usize,
    pub column_offset: usize,
    pub row_offset: usize,
    pub char_offset: usize,
}

fn sanitize_multiline(s: &str) -> String {
    s.replace('\r', "")
}
