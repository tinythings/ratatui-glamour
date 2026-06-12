use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};

use crate::widgets::key::{self, Binding};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Type {
    #[default]
    Arabic,
    Dots,
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub prev_page: Binding,
    pub next_page: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            prev_page: Binding::new([key::with_keys(&["pgup", "left", "h"])]),
            next_page: Binding::new([key::with_keys(&["pgdown", "right", "l"])]),
        }
    }
}

pub type Option = Box<dyn Fn(&mut Model)>;

#[derive(Clone, Debug)]
pub struct Model {
    pub r#type: Type,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub active_dot: String,
    pub inactive_dot: String,
    pub arabic_format: String,
    pub key_map: KeyMap,
    pub style: Style,
}

impl Default for Model {
    fn default() -> Self {
        Self::new([])
    }
}

impl Model {
    pub fn new(opts: impl IntoIterator<Item = Option>) -> Self {
        let mut model = Self {
            r#type: Type::Arabic,
            page: 0,
            per_page: 1,
            total_pages: 1,
            active_dot: "•".to_string(),
            inactive_dot: "○".to_string(),
            arabic_format: "%d/%d".to_string(),
            key_map: KeyMap::default(),
            style: Style::default(),
        };
        for opt in opts {
            opt(&mut model);
        }
        model
    }

    pub fn set_total_pages(&mut self, items: usize) -> usize {
        if items < 1 {
            return self.total_pages;
        }
        let mut n = items / self.per_page;
        if !items.is_multiple_of(self.per_page) {
            n += 1;
        }
        self.total_pages = n;
        n
    }

    pub fn items_on_page(&self, total_items: usize) -> usize {
        if total_items < 1 {
            return 0;
        }
        let (start, end) = self.slice_bounds(total_items);
        end - start
    }

    pub fn slice_bounds(&self, length: usize) -> (usize, usize) {
        let start = self.page * self.per_page;
        let end = (self.page * self.per_page + self.per_page).min(length);
        (start, end)
    }

    pub fn prev_page(&mut self) {
        if self.page > 0 {
            self.page -= 1;
        }
    }

    pub fn next_page(&mut self) {
        if !self.on_last_page() {
            self.page += 1;
        }
    }

    pub fn on_last_page(&self) -> bool {
        self.page == self.total_pages.saturating_sub(1)
    }

    pub fn on_first_page(&self) -> bool {
        self.page == 0
    }

    pub fn handle_key(&mut self, event: &crossterm::event::KeyEvent) {
        if key::matches(event, [&self.key_map.next_page]) {
            self.next_page();
        } else if key::matches(event, [&self.key_map.prev_page]) {
            self.prev_page();
        }
    }

    pub fn view(&self) -> String {
        match self.r#type {
            Type::Dots => self.dots_view(),
            Type::Arabic => self.arabic_view(),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        buf.set_line(
            area.x,
            area.y,
            &Line::styled(self.view(), self.style),
            area.width,
        );
    }

    fn dots_view(&self) -> String {
        let mut out = String::new();
        for i in 0..self.total_pages {
            if i == self.page {
                out.push_str(&self.active_dot);
            } else {
                out.push_str(&self.inactive_dot);
            }
        }
        out
    }

    fn arabic_view(&self) -> String {
        let mut out = String::new();
        let mut values = [(self.page + 1).to_string(), self.total_pages.to_string()].into_iter();
        let mut chars = self.arabic_format.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '%' && chars.peek() == Some(&'d') {
                let _ = chars.next();
                if let Some(value) = values.next() {
                    out.push_str(&value);
                } else {
                    out.push('%');
                    out.push('d');
                }
            } else {
                out.push(ch);
            }
        }
        out
    }
}

pub fn with_total_pages(total_pages: usize) -> Option {
    Box::new(move |model| model.total_pages = total_pages)
}

pub fn with_per_page(per_page: usize) -> Option {
    Box::new(move |model| model.per_page = per_page)
}
