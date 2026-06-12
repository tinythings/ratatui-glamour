use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, Instant},
};

use ratatui::{style::Style, text::Span};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

pub const DEFAULT_BLINK_SPEED: Duration = Duration::from_millis(530);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Blink,
    Static,
    Hide,
}

#[derive(Clone, Copy, Debug)]
pub struct BlinkMsg {
    pub id: i64,
    pub tag: usize,
}

#[derive(Clone, Debug)]
pub struct Model {
    pub style: Style,
    pub text_style: Style,
    pub blink_speed: Duration,
    pub is_blinked: bool,
    char_under: String,
    id: i64,
    focus: bool,
    blink_tag: usize,
    mode: Mode,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            text_style: Style::default(),
            blink_speed: DEFAULT_BLINK_SPEED,
            is_blinked: true,
            char_under: " ".into(),
            id: next_id(),
            focus: false,
            blink_tag: 0,
            mode: Mode::Blink,
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.is_blinked = self.mode == Mode::Hide || !self.focus;
    }
    pub fn blink_msg(&mut self) -> Option<BlinkMsg> {
        if self.mode != Mode::Blink {
            return None;
        }
        self.blink_tag += 1;
        Some(BlinkMsg {
            id: self.id,
            tag: self.blink_tag,
        })
    }
    pub fn update_blink(&mut self, msg: BlinkMsg) -> bool {
        if self.mode != Mode::Blink || !self.focus {
            return false;
        }
        if msg.id != self.id || msg.tag != self.blink_tag {
            return false;
        }
        self.is_blinked = !self.is_blinked;
        true
    }
    pub fn next_blink_at(&self, now: Instant) -> Instant {
        now + self.blink_speed
    }
    pub fn focus(&mut self) {
        self.focus = true;
        self.is_blinked = self.mode == Mode::Hide;
    }
    pub fn blur(&mut self) {
        self.focus = false;
        self.is_blinked = true;
    }
    pub fn set_char(&mut self, ch: impl Into<String>) {
        self.char_under = ch.into();
    }
    pub fn view(&self) -> Span<'static> {
        if self.is_blinked {
            Span::styled(self.char_under.clone(), self.text_style)
        } else {
            Span::styled(self.char_under.clone(), self.style.reversed())
        }
    }
}
