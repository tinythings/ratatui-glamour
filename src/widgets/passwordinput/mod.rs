use crossterm::event::KeyEvent;
use ratatui::text::Line;

use crate::widgets::textinput::{self, EchoMode};

pub const DEFAULT_OBSCURE_CHAR: char = '\u{25CF}';

pub struct Model(textinput::Model);

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        let mut inner = textinput::Model::new();
        inner.echo_mode = EchoMode::Password;
        inner.echo_character = DEFAULT_OBSCURE_CHAR;
        inner.show_suggestions = false;
        inner.key_map.paste.unbind();
        Self(inner)
    }

    pub fn set_echo_character(&mut self, c: char) {
        self.0.echo_character = c;
    }

    pub fn echo_character(&self) -> char {
        self.0.echo_character
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        self.0.handle_key(event);
    }

    pub fn view(&self) -> Line<'static> {
        self.0.view()
    }

    pub fn focus(&mut self) {
        self.0.focus();
    }

    pub fn blur(&mut self) {
        self.0.blur();
    }

    pub fn focused(&self) -> bool {
        self.0.focused()
    }

    pub fn set_width(&mut self, width: usize) {
        self.0.set_width(width);
    }

    pub fn width(&self) -> usize {
        self.0.width()
    }

    pub fn value(&self) -> String {
        self.0.value()
    }

    pub fn set_value(&mut self, s: &str) {
        self.0.set_value(s);
    }

    pub fn reset(&mut self) {
        self.0.reset();
    }

    pub fn placeholder(&self) -> &str {
        &self.0.placeholder
    }

    pub fn set_placeholder(&mut self, s: impl Into<String>) {
        self.0.placeholder = s.into();
    }

    pub fn prompt(&self) -> &str {
        &self.0.prompt
    }

    pub fn set_prompt(&mut self, s: impl Into<String>) {
        self.0.prompt = s.into();
    }

    pub fn set_char_limit(&mut self, limit: usize) {
        self.0.char_limit = limit;
    }
}
