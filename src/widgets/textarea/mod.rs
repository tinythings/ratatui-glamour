use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::widgets::{
    key::{self, Binding},
    viewport,
};

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
    pub uppercase_word_forward: Binding,
    pub lowercase_word_forward: Binding,
    pub capitalize_word_forward: Binding,
    pub transpose_character_backward: Binding,
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
            capitalize_word_forward: Binding::new([key::with_keys(&["alt+c"])]),
            lowercase_word_forward: Binding::new([key::with_keys(&["alt+l"])]),
            uppercase_word_forward: Binding::new([key::with_keys(&["alt+u"])]),
            transpose_character_backward: Binding::new([key::with_keys(&["ctrl+t"])]),
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

    pub fn styles(&self) -> &Styles {
        &self.styles
    }
    pub fn set_styles(&mut self, styles: Styles) {
        self.styles = styles;
        self.sync_viewport();
    }
    pub fn focused(&self) -> bool {
        self.focus
    }
    pub fn focus(&mut self) {
        self.focus = true;
    }
    pub fn blur(&mut self) {
        self.focus = false;
    }
    pub fn line_count(&self) -> usize {
        self.value.len()
    }
    pub fn line(&self) -> usize {
        self.row
    }
    pub fn column(&self) -> usize {
        self.col
    }
    pub fn scroll_y_offset(&self) -> usize {
        self.viewport.y_offset()
    }
    pub fn scroll_percent(&self) -> f64 {
        self.viewport.scroll_percent()
    }

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
        self.value
            .iter()
            .map(|line| line.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn length(&self) -> usize {
        self.value.iter().map(Vec::len).sum::<usize>() + self.value.len().saturating_sub(1)
    }

    pub fn cursor_down(&mut self) {
        self.set_cursor_line_relative(1);
    }
    pub fn cursor_up(&mut self) {
        self.set_cursor_line_relative(-1);
    }
    pub fn set_cursor_column(&mut self, col: usize) {
        self.col = col.min(self.value[self.row].len());
        self.last_char_offset = 0;
        self.reposition_view();
    }
    pub fn cursor_start(&mut self) {
        self.set_cursor_column(0);
    }
    pub fn cursor_end(&mut self) {
        self.set_cursor_column(self.value[self.row].len());
    }
    pub fn move_to_begin(&mut self) {
        self.row = 0;
        self.cursor_start();
    }
    pub fn move_to_end(&mut self) {
        self.row = self.value.len().saturating_sub(1);
        self.cursor_end();
    }
    pub fn reset(&mut self) {
        self.value = vec![Vec::new()];
        self.col = 0;
        self.row = 0;
        self.viewport.goto_top();
        self.recalculate_height();
    }
    pub fn set_width(&mut self, width: usize) {
        self.width = width.min(self.max_width);
        self.viewport.set_width(self.content_width());
        self.prompt_width = self.compute_prompt_width();
        self.recalculate_height();
    }
    pub fn set_height(&mut self, height: usize) {
        self.height = height.max(1).min(self.max_height);
        self.viewport.set_height(self.content_height());
        self.recalculate_height();
    }
    pub fn set_prompt_func(&mut self, prompt_width: usize, func: PromptFunc) {
        self.prompt_width = prompt_width;
        self.prompt_func = Some(func);
        self.sync_viewport();
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        if !self.focus {
            return;
        }
        match event.code {
            _ if key::matches(event, [&self.key_map.delete_word_backward]) => {
                self.delete_word_backward()
            }
            _ if key::matches(event, [&self.key_map.delete_character_backward]) => {
                self.delete_character_backward()
            }
            _ if key::matches(event, [&self.key_map.word_backward]) => self.word_backward(),
            _ if key::matches(event, [&self.key_map.character_backward]) => {
                if self.col > 0 {
                    self.set_cursor_column(self.col - 1);
                } else if self.row > 0 {
                    self.row -= 1;
                    self.cursor_end();
                }
            }
            _ if key::matches(event, [&self.key_map.word_forward]) => self.word_forward(),
            _ if key::matches(event, [&self.key_map.character_forward]) => {
                if self.col < self.value[self.row].len() {
                    self.set_cursor_column(self.col + 1);
                } else if self.row + 1 < self.value.len() {
                    self.row += 1;
                    self.cursor_start();
                }
            }
            _ if key::matches(event, [&self.key_map.line_start]) => self.cursor_start(),
            _ if key::matches(event, [&self.key_map.delete_character_forward]) => {
                self.delete_character_forward()
            }
            _ if key::matches(event, [&self.key_map.line_end]) => self.cursor_end(),
            _ if key::matches(event, [&self.key_map.delete_after_cursor]) => {
                self.delete_after_cursor()
            }
            _ if key::matches(event, [&self.key_map.delete_before_cursor]) => {
                self.delete_before_cursor()
            }
            _ if key::matches(event, [&self.key_map.delete_word_forward]) => {
                self.delete_word_forward()
            }
            _ if key::matches(event, [&self.key_map.insert_newline]) => self.insert_runes(&['\n']),
            _ if key::matches(event, [&self.key_map.line_next]) => self.cursor_down(),
            _ if key::matches(event, [&self.key_map.line_previous]) => self.cursor_up(),
            _ if key::matches(event, [&self.key_map.page_down]) => self.page_down(),
            _ if key::matches(event, [&self.key_map.page_up]) => self.page_up(),
            _ if key::matches(event, [&self.key_map.input_begin]) => self.move_to_begin(),
            _ if key::matches(event, [&self.key_map.input_end]) => self.move_to_end(),
            _ if key::matches(event, [&self.key_map.lowercase_word_forward]) => {
                self.lowercase_right()
            }
            _ if key::matches(event, [&self.key_map.uppercase_word_forward]) => {
                self.uppercase_right()
            }
            _ if key::matches(event, [&self.key_map.capitalize_word_forward]) => {
                self.capitalize_right()
            }
            _ if key::matches(event, [&self.key_map.transpose_character_backward]) => {
                self.transpose_left()
            }
            KeyCode::Char(c)
                if !event.modifiers.contains(KeyModifiers::CONTROL)
                    && !event.modifiers.contains(KeyModifiers::ALT) =>
            {
                self.insert_runes(&[c])
            }
            _ => {}
        }
        self.recalculate_height();
    }

    pub fn line_info(&self) -> LineInfo {
        let grid = wrap(&self.value[self.row], self.content_width().max(1));
        let mut counter = 0usize;
        for (i, line) in grid.iter().enumerate() {
            if counter + line.len() == self.col && i + 1 < grid.len() {
                return LineInfo {
                    char_offset: 0,
                    column_offset: 0,
                    height: grid.len(),
                    row_offset: i + 1,
                    start_column: self.col,
                    width: grid[i + 1].len(),
                    char_width: display_width_chars(line),
                };
            }
            if counter + line.len() >= self.col {
                let prefix: Vec<char> =
                    line[..self.col.saturating_sub(counter).min(line.len())].to_vec();
                return LineInfo {
                    char_offset: display_width_chars(&prefix),
                    column_offset: self.col.saturating_sub(counter),
                    height: grid.len(),
                    row_offset: i,
                    start_column: counter,
                    width: line.len(),
                    char_width: display_width_chars(line),
                };
            }
            counter += line.len();
        }
        LineInfo::default()
    }

    pub fn view(&self) -> Vec<Line<'static>> {
        if self.value.len() == 1
            && self.value[0].is_empty()
            && self.row == 0
            && self.col == 0
            && !self.placeholder.is_empty()
        {
            return self.placeholder_lines();
        }

        let active = self.active_style();
        let mut lines = Vec::new();
        let mut display_line = 0usize;
        let line_info = self.line_info();

        for (logical_idx, row) in self.value.iter().enumerate() {
            let wrapped_lines = wrap(row, self.content_width().max(1));
            let line_number_style = if self.row == logical_idx {
                active.cursor_line_number
            } else {
                active.line_number
            };
            let text_style = if self.row == logical_idx {
                active.cursor_line
            } else {
                active.text
            };

            for (wrapped_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
                let mut spans = Vec::new();
                spans.push(Span::styled(
                    self.prompt_for_display_line(display_line),
                    active.prompt,
                ));
                display_line += 1;

                if self.show_line_numbers {
                    spans.push(Span::styled(
                        self.line_number_view(if wrapped_idx == 0 {
                            Some(logical_idx + 1)
                        } else {
                            None
                        }),
                        line_number_style,
                    ));
                }

                let mut text: Vec<char> = wrapped_line.clone();
                if display_width_chars(&text) > self.content_width().max(1) {
                    while text.last() == Some(&' ')
                        && display_width_chars(&text) > self.content_width().max(1)
                    {
                        text.pop();
                    }
                }

                if self.row == logical_idx && line_info.row_offset == wrapped_idx {
                    let before: String = text[..line_info.column_offset.min(text.len())]
                        .iter()
                        .collect();
                    spans.push(Span::styled(before, text_style));
                    let cursor_ch = if self.col >= row.len()
                        && line_info.char_offset >= self.content_width().max(1)
                    {
                        ' '
                    } else {
                        text.get(line_info.column_offset)
                            .copied()
                            .unwrap_or(self.end_of_buffer_character)
                    };
                    spans.push(Span::styled(
                        cursor_ch.to_string(),
                        self.styles.cursor.style,
                    ));
                    let after: String = text
                        .get(line_info.column_offset + 1..)
                        .unwrap_or(&[])
                        .iter()
                        .collect();
                    spans.push(Span::styled(after, text_style));
                } else {
                    spans.push(Span::styled(text.iter().collect::<String>(), text_style));
                }

                let padding = self
                    .content_width()
                    .saturating_sub(display_width_chars(&text));
                if padding > 0 {
                    spans.push(Span::styled(" ".repeat(padding), text_style));
                }
                lines.push(Line::from(spans));
            }
        }

        while lines.len() < self.height {
            let mut spans = Vec::new();
            spans.push(Span::styled(
                self.prompt_for_display_line(display_line),
                active.prompt,
            ));
            display_line += 1;
            let mut eob = self.end_of_buffer_character.to_string();
            let pad = self.content_width().saturating_sub(1)
                + if self.show_line_numbers {
                    self.line_number_width()
                } else {
                    0
                };
            eob.push_str(&" ".repeat(pad));
            spans.push(Span::styled(eob, active.end_of_buffer));
            lines.push(Line::from(spans));
        }

        lines
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.view();
        for (i, line) in lines
            .into_iter()
            .skip(self.viewport.y_offset())
            .take(area.height as usize)
            .enumerate()
        {
            buf.set_line(area.x, area.y + i as u16, &line, area.width);
        }
    }

    fn insert_runes(&mut self, runes: &[char]) {
        let mut runes = runes.to_vec();
        if self.char_limit > 0 {
            let available = self.char_limit.saturating_sub(self.length());
            runes.truncate(available);
        }
        if runes.is_empty() {
            return;
        }
        let mut parts: Vec<Vec<char>> = vec![Vec::new()];
        for ch in runes {
            if ch == '\n' {
                parts.push(Vec::new());
            } else {
                parts.last_mut().unwrap().push(ch);
            }
        }
        let tail = self.value[self.row][self.col..].to_vec();
        self.value[self.row].truncate(self.col);
        self.value[self.row].extend_from_slice(&parts[0]);
        self.col += parts[0].len();
        if parts.len() > 1 {
            for (insert_at, line) in (self.row + 1..).zip(parts.iter().skip(1)) {
                self.value.insert(insert_at, line.clone());
                self.row = insert_at;
                self.col = line.len();
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
            self.merge_line_below(self.row);
            self.reposition_view();
            return;
        }
        self.reposition_view();
    }

    fn delete_before_cursor(&mut self) {
        if self.col == 0 {
            self.merge_line_above(self.row);
            self.reposition_view();
            return;
        }
        self.value[self.row].drain(..self.col);
        self.col = 0;
        self.reposition_view();
    }

    fn delete_after_cursor(&mut self) {
        if self.col >= self.value[self.row].len() {
            self.merge_line_below(self.row);
            self.reposition_view();
            return;
        }
        self.value[self.row].truncate(self.col);
        self.reposition_view();
    }

    fn delete_word_backward(&mut self) {
        if self.col == 0 {
            self.merge_line_above(self.row);
            self.reposition_view();
            return;
        }
        let old_col = self.col;
        self.col -= 1;
        while self.col > 0 && self.value[self.row][self.col].is_whitespace() {
            self.col -= 1;
        }
        while self.col > 0 && !self.value[self.row][self.col].is_whitespace() {
            self.col -= 1;
        }
        if self.col > 0 && self.value[self.row][self.col].is_whitespace() {
            self.col += 1;
        }
        self.value[self.row].drain(self.col..old_col);
        self.reposition_view();
    }

    fn delete_word_forward(&mut self) {
        if self.col >= self.value[self.row].len() {
            self.merge_line_below(self.row);
            self.reposition_view();
            return;
        }
        let start = self.col;
        while self.col < self.value[self.row].len()
            && self.value[self.row][self.col].is_whitespace()
        {
            self.col += 1;
        }
        while self.col < self.value[self.row].len()
            && !self.value[self.row][self.col].is_whitespace()
        {
            self.col += 1;
        }
        self.value[self.row].drain(start..self.col);
        self.col = start;
        self.reposition_view();
    }

    fn word_backward(&mut self) {
        while self.col > 0 && self.value[self.row][self.col - 1].is_whitespace() {
            self.col -= 1;
        }
        while self.col > 0 && !self.value[self.row][self.col - 1].is_whitespace() {
            self.col -= 1;
        }
        self.reposition_view();
    }

    fn word_forward(&mut self) {
        while self.col < self.value[self.row].len()
            && self.value[self.row][self.col].is_whitespace()
        {
            self.col += 1;
        }
        while self.col < self.value[self.row].len()
            && !self.value[self.row][self.col].is_whitespace()
        {
            self.col += 1;
        }
        self.reposition_view();
    }

    fn do_word_right(&mut self, mut f: impl FnMut(usize, usize, &mut Vec<char>)) {
        while self.col >= self.value[self.row].len()
            || self.value[self.row][self.col].is_whitespace()
        {
            if self.row == self.value.len().saturating_sub(1)
                && self.col == self.value[self.row].len()
            {
                break;
            }
            if self.col < self.value[self.row].len() {
                self.set_cursor_column(self.col + 1);
            } else if self.row + 1 < self.value.len() {
                self.row += 1;
                self.cursor_start();
            } else {
                break;
            }
        }

        let mut char_idx = 0;
        while self.col < self.value[self.row].len() {
            if self.value[self.row][self.col].is_whitespace() {
                break;
            }
            let pos = self.col;
            f(char_idx, pos, &mut self.value[self.row]);
            self.set_cursor_column(self.col + 1);
            char_idx += 1;
        }
    }

    fn uppercase_right(&mut self) {
        self.do_word_right(|_, i, row| row[i] = row[i].to_ascii_uppercase());
    }

    fn lowercase_right(&mut self) {
        self.do_word_right(|_, i, row| row[i] = row[i].to_ascii_lowercase());
    }

    fn capitalize_right(&mut self) {
        self.do_word_right(|char_idx, i, row| {
            if char_idx == 0 {
                row[i] = row[i].to_ascii_uppercase();
            }
        });
    }

    fn transpose_left(&mut self) {
        if self.col == 0 || self.value[self.row].len() < 2 {
            return;
        }
        if self.col >= self.value[self.row].len() {
            self.set_cursor_column(self.col - 1);
        }
        self.value[self.row].swap(self.col - 1, self.col);
        if self.col < self.value[self.row].len() {
            self.set_cursor_column(self.col + 1);
        }
    }

    fn set_cursor_line_relative(&mut self, delta: isize) {
        if delta == 0 {
            return;
        }
        let mut li = self.line_info();
        let char_offset = self.last_char_offset.max(li.char_offset);
        self.last_char_offset = char_offset;
        const TRAILING_SPACE: usize = 2;

        if delta > 0 {
            for _ in 0..delta as usize {
                if li.row_offset + 1 >= li.height && self.row < self.value.len().saturating_sub(1) {
                    self.row += 1;
                    self.col = 0;
                } else {
                    self.col = (li.start_column + li.width + TRAILING_SPACE)
                        .min(self.value[self.row].len().saturating_sub(1));
                }
                li = self.line_info();
            }
        } else {
            for _ in 0..(-delta) as usize {
                if li.row_offset == 0 && self.row > 0 {
                    self.row -= 1;
                    self.col = self.value[self.row].len();
                } else {
                    self.col = li.start_column.saturating_sub(TRAILING_SPACE);
                }
                li = self.line_info();
            }
        }

        let nli = self.line_info();
        self.col = nli.start_column;
        if nli.width > 0 {
            let mut offset = 0usize;
            while offset < char_offset {
                if self.row >= self.value.len()
                    || self.col >= self.value[self.row].len()
                    || offset >= nli.char_width.saturating_sub(1)
                {
                    break;
                }
                offset += char_width(self.value[self.row][self.col]);
                self.col += 1;
            }
        }
        self.reposition_view();
    }

    fn reposition_view(&mut self) {
        self.sync_viewport();
        let minimum = self.viewport.y_offset();
        let maximum = minimum + self.viewport.height().saturating_sub(1);
        let row = self.cursor_line_number();
        if row < minimum {
            self.viewport.scroll_up(minimum - row);
        } else if row > maximum {
            self.viewport.scroll_down(row - maximum);
        }
    }

    fn merge_line_above(&mut self, row: usize) {
        if row == 0 {
            return;
        }
        let current = self.value.remove(row);
        self.row = row - 1;
        self.col = self.value[self.row].len();
        self.value[self.row].extend(current);
    }

    fn merge_line_below(&mut self, row: usize) {
        if row + 1 >= self.value.len() {
            return;
        }
        let next = self.value.remove(row + 1);
        self.value[row].extend(next);
        self.row = row;
        self.col = self.col.min(self.value[row].len());
    }

    fn sync_viewport(&mut self) {
        self.viewport.set_width(self.content_width());
        self.viewport.set_height(self.content_height());
        self.viewport.set_content(&self.rendered_content_string());
    }

    fn recalculate_height(&mut self) {
        if self.dynamic_height {
            let min_h = self.min_height.max(1);
            let total = self.total_visual_lines();
            let mut h = total.max(min_h);
            if self.max_height > 0 {
                h = h.min(self.max_height);
            }
            let max_offset = total.saturating_sub(h);
            if self.viewport.y_offset() > max_offset {
                self.viewport.set_y_offset(max_offset);
            }
            self.height = h.max(1);
            self.viewport.set_height(self.height);
        }
        self.sync_viewport();
        self.reposition_view();
    }

    fn content_width(&self) -> usize {
        self.width.max(1)
    }

    fn content_height(&self) -> usize {
        self.height.max(1)
    }

    fn compute_prompt_width(&self) -> usize {
        self.prompt_width
            .max(UnicodeWidthStr::width(self.prompt.as_str()))
    }

    fn active_style(&self) -> &StyleState {
        if self.focus {
            &self.styles.focused
        } else {
            &self.styles.blurred
        }
    }

    fn prompt_for_display_line(&self, line: usize) -> String {
        if let Some(f) = &self.prompt_func {
            let prompt = f(PromptInfo {
                line_number: line,
                focused: self.focus,
            });
            let width = UnicodeWidthStr::width(prompt.as_str());
            if width < self.prompt_width {
                return format!("{}{}", " ".repeat(self.prompt_width - width), prompt);
            }
            return prompt;
        }
        self.prompt.clone()
    }

    fn line_number_width(&self) -> usize {
        if !self.show_line_numbers {
            return 0;
        }
        digits(self.max_height) + 3
    }

    fn line_number_view(&self, number: Option<usize>) -> String {
        if !self.show_line_numbers {
            return String::new();
        }
        let raw = number
            .map(|n| n.to_string())
            .unwrap_or_else(|| " ".to_string());
        format!(" {:>width$} ", raw, width = digits(self.max_height))
    }

    fn cursor_line_number(&self) -> usize {
        let mut line = 0usize;
        for i in 0..self.row {
            line += wrap(&self.value[i], self.content_width().max(1)).len();
        }
        line + self.line_info().row_offset
    }

    fn total_visual_lines(&self) -> usize {
        self.value
            .iter()
            .map(|line| wrap(line, self.content_width().max(1)).len())
            .sum()
    }

    fn page_up(&mut self) {
        let offset = self.viewport.y_offset() as isize - self.cursor_line_number() as isize;
        if offset < 0 {
            self.set_cursor_line_relative(offset);
            return;
        }
        self.set_cursor_line_relative(-(self.height as isize));
    }

    fn page_down(&mut self) {
        let offset = self
            .cursor_line_number()
            .saturating_sub(self.viewport.y_offset());
        if offset < self.height.saturating_sub(1) {
            self.set_cursor_line_relative(
                (self.height.saturating_sub(1).saturating_sub(offset)) as isize,
            );
            return;
        }
        self.set_cursor_line_relative(self.height as isize);
    }

    fn rendered_content_string(&self) -> String {
        self.view()
            .into_iter()
            .map(|line| line.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn placeholder_lines(&self) -> Vec<Line<'static>> {
        let active = self.active_style();
        let placeholder = hard_wrap_lines(&self.placeholder, self.content_width().max(1));
        let mut out = Vec::new();
        for i in 0..self.height {
            let mut spans = Vec::new();
            spans.push(Span::styled(self.prompt_for_display_line(i), active.prompt));
            if self.show_line_numbers {
                let line_num = if i == 0 { Some(1) } else { None };
                spans.push(Span::styled(
                    self.line_number_view(line_num),
                    if i < placeholder.len() {
                        active.cursor_line_number
                    } else {
                        active.line_number
                    },
                ));
            }
            if i == 0 && !placeholder.is_empty() {
                let first = placeholder[0]
                    .chars()
                    .next()
                    .unwrap_or(self.end_of_buffer_character);
                spans.push(Span::styled(first.to_string(), self.styles.cursor.style));
                let rest = placeholder[0].chars().skip(1).collect::<String>();
                spans.push(Span::styled(rest, active.placeholder));
            } else if let Some(line) = placeholder.get(i) {
                spans.push(Span::styled(line.clone(), active.placeholder));
            } else {
                spans.push(Span::styled(
                    self.end_of_buffer_character.to_string(),
                    active.end_of_buffer,
                ));
            }
            out.push(Line::from(spans));
        }
        out
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

fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0).max(1)
}

fn display_width_chars(chars: &[char]) -> usize {
    chars.iter().map(|c| char_width(*c)).sum()
}

fn wrap(runes: &[char], width: usize) -> Vec<Vec<char>> {
    let width = width.max(1);
    let mut lines: Vec<Vec<char>> = vec![Vec::new()];
    let mut word: Vec<char> = Vec::new();
    let mut row = 0usize;
    let mut spaces = 0usize;

    for &r in runes {
        if r.is_whitespace() {
            spaces += 1;
        } else {
            word.push(r);
        }

        if spaces > 0 {
            if display_width_chars(&lines[row]) + display_width_chars(&word) + spaces > width {
                row += 1;
                lines.push(Vec::new());
                lines[row].extend(word.iter().copied());
                lines[row].extend(std::iter::repeat_n(' ', spaces));
                spaces = 0;
                word.clear();
            } else {
                lines[row].extend(word.iter().copied());
                lines[row].extend(std::iter::repeat_n(' ', spaces));
                spaces = 0;
                word.clear();
            }
        } else if !word.is_empty() {
            let last_char_len = char_width(*word.last().unwrap());
            if display_width_chars(&word) + last_char_len > width {
                if !lines[row].is_empty() {
                    row += 1;
                    lines.push(Vec::new());
                }
                lines[row].extend(word.iter().copied());
                word.clear();
            }
        }
    }

    if display_width_chars(&lines[row]) + display_width_chars(&word) + spaces >= width {
        lines.push(Vec::new());
        let next = row + 1;
        lines[next].extend(word.iter().copied());
        spaces += 1;
        lines[next].extend(std::iter::repeat_n(' ', spaces));
    } else {
        lines[row].extend(word.iter().copied());
        spaces += 1;
        lines[row].extend(std::iter::repeat_n(' ', spaces));
    }

    lines
}

fn digits(n: usize) -> usize {
    if n == 0 { 1 } else { n.to_string().len() }
}

fn hard_wrap_lines(s: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for logical in s.split('\n') {
        let wrapped = wrap(&logical.chars().collect::<Vec<_>>(), width.max(1));
        for line in wrapped {
            let mut text: String = line.into_iter().collect();
            text = text.trim_end().to_string();
            out.push(text);
        }
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}
