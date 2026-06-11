use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::widgets::{cursor, key::{self, Binding}};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EchoMode {
    #[default]
    Normal,
    Password,
    None,
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub character_forward: Binding,
    pub character_backward: Binding,
    pub word_forward: Binding,
    pub word_backward: Binding,
    pub delete_word_backward: Binding,
    pub delete_word_forward: Binding,
    pub delete_after_cursor: Binding,
    pub delete_before_cursor: Binding,
    pub delete_character_backward: Binding,
    pub delete_character_forward: Binding,
    pub line_start: Binding,
    pub line_end: Binding,
    pub paste: Binding,
    pub accept_suggestion: Binding,
    pub next_suggestion: Binding,
    pub prev_suggestion: Binding,
}

impl KeyMap {
    pub fn default_keymap() -> Self {
        Self {
            character_forward: Binding::new([key::with_keys(&["right", "ctrl+f"])]),
            character_backward: Binding::new([key::with_keys(&["left", "ctrl+b"])]),
            word_forward: Binding::new([key::with_keys(&["alt+right", "ctrl+right", "alt+f"])]),
            word_backward: Binding::new([key::with_keys(&["alt+left", "ctrl+left", "alt+b"])]),
            delete_word_backward: Binding::new([key::with_keys(&["alt+backspace", "ctrl+w"])]),
            delete_word_forward: Binding::new([key::with_keys(&["alt+delete", "alt+d"])]),
            delete_after_cursor: Binding::new([key::with_keys(&["ctrl+k"])]),
            delete_before_cursor: Binding::new([key::with_keys(&["ctrl+u"])]),
            delete_character_backward: Binding::new([key::with_keys(&["backspace", "ctrl+h"])]),
            delete_character_forward: Binding::new([key::with_keys(&["delete", "ctrl+d"])]),
            line_start: Binding::new([key::with_keys(&["home", "ctrl+a"])]),
            line_end: Binding::new([key::with_keys(&["end", "ctrl+e"])]),
            paste: Binding::new([key::with_keys(&["ctrl+v"])]),
            accept_suggestion: Binding::new([key::with_keys(&["tab"])]),
            next_suggestion: Binding::new([key::with_keys(&["down", "ctrl+n"])]),
            prev_suggestion: Binding::new([key::with_keys(&["up", "ctrl+p"])]),
        }
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::default_keymap()
    }
}

#[derive(Clone, Debug, Default)]
pub struct StyleState {
    pub text: Style,
    pub placeholder: Style,
    pub suggestion: Style,
    pub prompt: Style,
}

#[derive(Clone, Debug)]
pub struct CursorStyle {
    pub style: Style,
}

impl Default for CursorStyle {
    fn default() -> Self {
        Self {
            style: Style::default().add_modifier(Modifier::REVERSED),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub focused: StyleState,
    pub blurred: StyleState,
    pub cursor: CursorStyle,
}

pub type ValidateFunc = Box<dyn Fn(&str) -> Result<(), String>>;

pub struct Model {
    pub err: Option<String>,
    pub prompt: String,
    pub placeholder: String,
    pub echo_mode: EchoMode,
    pub echo_character: char,
    pub char_limit: usize,
    styles: Styles,
    width: usize,
    pub key_map: KeyMap,
    value: Vec<char>,
    focus: bool,
    pos: usize,
    offset: usize,
    offset_right: usize,
    pub validate: Option<ValidateFunc>,
    pub show_suggestions: bool,
    suggestions: Vec<Vec<char>>,
    matched_suggestions: Vec<Vec<char>>,
    current_suggestion_index: usize,
    pub virtual_cursor: cursor::Model,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            err: None,
            prompt: "> ".to_string(),
            placeholder: String::new(),
            echo_mode: EchoMode::Normal,
            echo_character: '*',
            char_limit: 0,
            styles: Styles::default(),
            width: 0,
            key_map: KeyMap::default(),
            value: Vec::new(),
            focus: false,
            pos: 0,
            offset: 0,
            offset_right: 0,
            validate: None,
            show_suggestions: false,
            suggestions: Vec::new(),
            matched_suggestions: Vec::new(),
            current_suggestion_index: 0,
            virtual_cursor: cursor::Model::new(),
        }
    }

    pub fn styles(&self) -> &Styles { &self.styles }
    pub fn set_styles(&mut self, styles: Styles) { self.styles = styles; }
    pub fn width(&self) -> usize { self.width }
    pub fn set_width(&mut self, width: usize) { self.width = width; self.handle_overflow(); }
    pub fn value(&self) -> String { self.value.iter().collect() }
    pub fn position(&self) -> usize { self.pos }
    pub fn focused(&self) -> bool { self.focus }
    pub fn focus(&mut self) { self.focus = true; self.virtual_cursor.focus(); }
    pub fn blur(&mut self) { self.focus = false; self.virtual_cursor.blur(); }
    pub fn reset(&mut self) { self.value.clear(); self.set_cursor(0); }

    pub fn set_value(&mut self, s: &str) {
        let chars: Vec<char> = sanitize_single_line(s).chars().collect();
        let err = self.validate_chars(&chars);
        self.set_value_internal(chars, err);
    }

    pub fn set_cursor(&mut self, pos: usize) {
        self.pos = pos.min(self.value.len());
        self.handle_overflow();
    }

    pub fn cursor_start(&mut self) { self.set_cursor(0); }
    pub fn cursor_end(&mut self) { self.set_cursor(self.value.len()); }

    pub fn set_suggestions(&mut self, suggestions: &[String]) {
        self.suggestions = suggestions.iter().map(|s| s.chars().collect()).collect();
        self.update_suggestions();
    }

    pub fn available_suggestions(&self) -> Vec<String> {
        self.suggestions.iter().map(|s| s.iter().collect()).collect()
    }

    pub fn matched_suggestions(&self) -> Vec<String> {
        self.matched_suggestions.iter().map(|s| s.iter().collect()).collect()
    }

    pub fn current_suggestion(&self) -> String {
        self.matched_suggestions
            .get(self.current_suggestion_index)
            .map(|s| s.iter().collect())
            .unwrap_or_default()
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        if !self.focus {
            return;
        }

        if key::matches(event, [&self.key_map.accept_suggestion]) && self.can_accept_suggestion() {
            if let Some(suggestion) = self.matched_suggestions.get(self.current_suggestion_index).cloned() {
                self.value = suggestion;
                self.cursor_end();
            }
        }

        match event.code {
            _ if key::matches(event, [&self.key_map.delete_word_backward]) => self.delete_word_backward(),
            _ if key::matches(event, [&self.key_map.delete_character_backward]) => self.delete_character_backward(),
            _ if key::matches(event, [&self.key_map.word_backward]) => self.word_backward(),
            _ if key::matches(event, [&self.key_map.character_backward]) => {
                if self.pos > 0 { self.set_cursor(self.pos - 1); }
            }
            _ if key::matches(event, [&self.key_map.word_forward]) => self.word_forward(),
            _ if key::matches(event, [&self.key_map.character_forward]) => {
                if self.pos < self.value.len() { self.set_cursor(self.pos + 1); }
            }
            _ if key::matches(event, [&self.key_map.line_start]) => self.cursor_start(),
            _ if key::matches(event, [&self.key_map.delete_character_forward]) => self.delete_character_forward(),
            _ if key::matches(event, [&self.key_map.line_end]) => self.cursor_end(),
            _ if key::matches(event, [&self.key_map.delete_after_cursor]) => self.delete_after_cursor(),
            _ if key::matches(event, [&self.key_map.delete_before_cursor]) => self.delete_before_cursor(),
            _ if key::matches(event, [&self.key_map.delete_word_forward]) => self.delete_word_forward(),
            _ if key::matches(event, [&self.key_map.next_suggestion]) => self.next_suggestion(),
            _ if key::matches(event, [&self.key_map.prev_suggestion]) => self.previous_suggestion(),
            KeyCode::Char(c) if !event.modifiers.contains(KeyModifiers::CONTROL) && !event.modifiers.contains(KeyModifiers::ALT) => {
                self.insert_chars(&[c]);
            }
            KeyCode::Enter => self.insert_chars(&[' ']),
            _ => {}
        }

        self.update_suggestions();
        self.handle_overflow();
    }

    pub fn view(&self) -> Line<'static> {
        let styles = self.active_style();
        let prompt = Span::styled(self.prompt.clone(), styles.prompt);

        if self.value.is_empty() && !self.placeholder.is_empty() {
            return self.placeholder_view(prompt, styles);
        }

        let visible = self.visible_value_chars();
        let pos = self.pos.saturating_sub(self.offset).min(visible.len());
        let mut spans = vec![prompt];
        spans.extend(render_chars(&visible[..pos], styles.text, self.echo_mode, self.echo_character));

        if pos < visible.len() {
            let mut vc = self.virtual_cursor.clone();
            vc.text_style = styles.text;
            vc.style = self.styles.cursor.style;
            vc.set_char(echo_char(visible[pos], self.echo_mode, self.echo_character).to_string());
            spans.push(vc.view());
            spans.extend(render_chars(&visible[pos + 1..], styles.text, self.echo_mode, self.echo_character));
            spans.push(Span::styled(self.completion_view(0), styles.suggestion));
        } else {
            let cursor_char = if self.focus && self.can_accept_suggestion() {
                self.current_suggestion()
                    .chars()
                    .nth(pos)
                    .map(|c| echo_char(c, self.echo_mode, self.echo_character))
                    .unwrap_or(' ')
            } else {
                ' '
            };
            let mut vc = self.virtual_cursor.clone();
            vc.text_style = styles.suggestion;
            vc.style = self.styles.cursor.style;
            vc.set_char(cursor_char.to_string());
            spans.push(vc.view());
            spans.push(Span::styled(self.completion_view(1), styles.suggestion));
        }

        let visible_width = UnicodeWidthStr::width(self.visible_value_string().as_str());
        if self.width > 0 && visible_width <= self.width {
            let mut padding = self.width.saturating_sub(visible_width);
            if pos < visible.len() {
                padding += 1;
            }
            if padding > 0 {
                spans.push(Span::styled(" ".repeat(padding), styles.text));
            }
        }

        Line::from(spans)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        buf.set_line(area.x, area.y, &self.view(), area.width);
    }

    fn active_style(&self) -> &StyleState {
        if self.focus { &self.styles.focused } else { &self.styles.blurred }
    }

    fn placeholder_view(&self, prompt: Span<'static>, styles: &StyleState) -> Line<'static> {
        let mut text = self.placeholder.clone();
        if self.width > 0 {
            text = text.chars().take(self.width).collect();
            let shown = UnicodeWidthStr::width(text.as_str());
            if shown < self.width {
                text.push_str(&" ".repeat(self.width - shown));
            }
        }
        let mut spans = vec![prompt];
        if self.focus && !text.is_empty() {
            let mut chars = text.chars();
            if let Some(first) = chars.next() {
                spans.push(Span::styled(first.to_string(), self.styles.cursor.style));
            }
            let rest: String = chars.collect();
            spans.push(Span::styled(rest, styles.placeholder));
        } else {
            spans.push(Span::styled(text, styles.placeholder));
        }
        Line::from(spans)
    }

    fn visible_value_chars(&self) -> Vec<char> {
        self.value[self.offset..self.offset_right.min(self.value.len())].to_vec()
    }

    fn visible_value_string(&self) -> String {
        self.visible_value_chars().iter().collect()
    }

    fn completion_view(&self, offset: usize) -> String {
        if !self.can_accept_suggestion() {
            return String::new();
        }
        let suggestion = &self.matched_suggestions[self.current_suggestion_index];
        if self.value.len() < suggestion.len() {
            suggestion[self.value.len() + offset..].iter().collect()
        } else {
            String::new()
        }
    }

    fn can_accept_suggestion(&self) -> bool {
        self.show_suggestions && !self.matched_suggestions.is_empty()
    }

    fn update_suggestions(&mut self) {
        if !self.show_suggestions {
            return;
        }
        if self.value.is_empty() || self.suggestions.is_empty() {
            self.matched_suggestions.clear();
            self.current_suggestion_index = 0;
            return;
        }
        let needle = self.value().to_lowercase();
        let matches: Vec<Vec<char>> = self
            .suggestions
            .iter()
            .filter(|s| s.iter().collect::<String>().to_lowercase().starts_with(&needle))
            .cloned()
            .collect();
        if matches != self.matched_suggestions {
            self.current_suggestion_index = 0;
        }
        self.matched_suggestions = matches;
    }

    fn next_suggestion(&mut self) {
        if self.matched_suggestions.is_empty() { return; }
        self.current_suggestion_index = (self.current_suggestion_index + 1) % self.matched_suggestions.len();
    }

    fn previous_suggestion(&mut self) {
        if self.matched_suggestions.is_empty() { return; }
        self.current_suggestion_index = (self.current_suggestion_index + self.matched_suggestions.len() - 1) % self.matched_suggestions.len();
    }

    fn insert_chars(&mut self, chars: &[char]) {
        let mut sanitized: Vec<char> = sanitize_single_line(&chars.iter().collect::<String>()).chars().collect();
        if self.char_limit > 0 {
            let available = self.char_limit.saturating_sub(self.value.len());
            sanitized.truncate(available);
        }
        if sanitized.is_empty() { return; }
        let mut new_value = self.value[..self.pos].to_vec();
        new_value.extend_from_slice(&sanitized);
        new_value.extend_from_slice(&self.value[self.pos..]);
        self.pos += sanitized.len();
        self.err = self.validate_chars(&new_value);
        self.value = new_value;
    }

    fn handle_overflow(&mut self) {
        if self.width == 0 || display_width_chars(&self.value) <= self.width {
            self.offset = 0;
            self.offset_right = self.value.len();
            return;
        }
        self.offset_right = self.offset_right.min(self.value.len());
        if self.pos < self.offset {
            self.offset = self.pos;
            self.offset_right = fit_right(&self.value, self.offset, self.width);
        } else if self.pos >= self.offset_right {
            self.offset_right = self.pos;
            self.offset = fit_left(&self.value, self.offset_right, self.width);
        }
        if self.offset_right <= self.offset {
            self.offset_right = fit_right(&self.value, self.offset, self.width);
        }
    }

    fn delete_before_cursor(&mut self) {
        self.value = self.value[self.pos..].to_vec();
        self.err = self.validate_chars(&self.value);
        self.offset = 0;
        self.set_cursor(0);
    }

    fn delete_after_cursor(&mut self) {
        self.value.truncate(self.pos);
        self.err = self.validate_chars(&self.value);
        self.set_cursor(self.value.len());
    }

    fn delete_character_backward(&mut self) {
        self.err = None;
        if !self.value.is_empty() && self.pos > 0 {
            self.value.remove(self.pos - 1);
            self.pos -= 1;
            self.err = self.validate_chars(&self.value);
        }
    }

    fn delete_character_forward(&mut self) {
        if !self.value.is_empty() && self.pos < self.value.len() {
            self.value.remove(self.pos);
            self.err = self.validate_chars(&self.value);
        }
    }

    fn delete_word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() { return; }
        if self.echo_mode != EchoMode::Normal { self.delete_before_cursor(); return; }
        let old = self.pos;
        while self.pos > 0 && self.value[self.pos - 1].is_whitespace() { self.pos -= 1; }
        while self.pos > 0 && !self.value[self.pos - 1].is_whitespace() { self.pos -= 1; }
        self.value.drain(self.pos..old);
        self.err = self.validate_chars(&self.value);
    }

    fn delete_word_forward(&mut self) {
        if self.pos >= self.value.len() || self.value.is_empty() { return; }
        if self.echo_mode != EchoMode::Normal { self.delete_after_cursor(); return; }
        let start = self.pos;
        while self.pos < self.value.len() && self.value[self.pos].is_whitespace() { self.pos += 1; }
        while self.pos < self.value.len() && !self.value[self.pos].is_whitespace() { self.pos += 1; }
        self.value.drain(start..self.pos);
        self.pos = start;
        self.err = self.validate_chars(&self.value);
    }

    fn word_backward(&mut self) {
        if self.pos == 0 || self.value.is_empty() { return; }
        if self.echo_mode != EchoMode::Normal { self.cursor_start(); return; }
        while self.pos > 0 && self.value[self.pos - 1].is_whitespace() { self.pos -= 1; }
        while self.pos > 0 && !self.value[self.pos - 1].is_whitespace() { self.pos -= 1; }
        self.handle_overflow();
    }

    fn word_forward(&mut self) {
        if self.pos >= self.value.len() || self.value.is_empty() { return; }
        if self.echo_mode != EchoMode::Normal { self.cursor_end(); return; }
        while self.pos < self.value.len() && self.value[self.pos].is_whitespace() { self.pos += 1; }
        while self.pos < self.value.len() && !self.value[self.pos].is_whitespace() { self.pos += 1; }
        self.handle_overflow();
    }

    fn validate_chars(&self, chars: &[char]) -> Option<String> {
        self.validate.as_ref().and_then(|f| f(&chars.iter().collect::<String>()).err())
    }

    fn set_value_internal(&mut self, mut chars: Vec<char>, err: Option<String>) {
        self.err = err;
        if self.char_limit > 0 && chars.len() > self.char_limit {
            chars.truncate(self.char_limit);
        }
        self.value = chars;
        if self.pos > self.value.len() || self.pos == 0 {
            self.set_cursor(self.value.len());
        } else {
            self.handle_overflow();
        }
    }
}

fn sanitize_single_line(s: &str) -> String {
    s.replace(['\n', '\r', '\t'], " ")
}

fn display_width_chars(chars: &[char]) -> usize {
    chars.iter().map(|c| c.width().unwrap_or(0).max(1)).sum()
}

fn fit_right(chars: &[char], start: usize, width: usize) -> usize {
    let mut used = 0usize;
    let mut idx = start;
    while idx < chars.len() {
        let cw = chars[idx].width().unwrap_or(0).max(1);
        if used + cw > width {
            break;
        }
        used += cw;
        idx += 1;
    }
    idx
}

fn fit_left(chars: &[char], end: usize, width: usize) -> usize {
    let mut used = 0usize;
    let mut idx = end;
    while idx > 0 {
        let cw = chars[idx - 1].width().unwrap_or(0).max(1);
        if used + cw > width {
            break;
        }
        used += cw;
        idx -= 1;
    }
    idx
}

fn echo_char(c: char, mode: EchoMode, echo: char) -> char {
    match mode {
        EchoMode::Normal => c,
        EchoMode::Password => echo,
        EchoMode::None => ' ',
    }
}

fn render_chars(chars: &[char], style: Style, mode: EchoMode, echo: char) -> Vec<Span<'static>> {
    if mode == EchoMode::None {
        return Vec::new();
    }
    vec![Span::styled(chars.iter().map(|c| echo_char(*c, mode, echo)).collect::<String>(), style)]
}
