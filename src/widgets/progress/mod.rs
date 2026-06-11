use std::{sync::{Arc, atomic::{AtomicI64, Ordering}}};

use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Style}, text::Line};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

pub type ColorFunc = Arc<dyn Fn(f64, f64) -> Color + Send + Sync>;
pub type Opt = Box<dyn Fn(&mut Model)>;

#[derive(Clone, Copy, Debug)]
pub struct FrameMsg {
    pub id: i64,
    pub tag: usize,
}

pub struct Model {
    id: i64,
    tag: usize,
    width: usize,
    pub full: char,
    pub full_style: Style,
    pub empty: char,
    pub empty_style: Style,
    pub show_percentage: bool,
    pub percent_format: String,
    pub percentage_style: Style,
    pub percent_shown: f64,
    pub target_percent: f64,
    pub velocity: f64,
    pub scale_blend: bool,
    pub color_func: std::option::Option<ColorFunc>,
}

impl Model {
    pub fn new(opts: impl IntoIterator<Item = Opt>) -> Self {
        let mut model = Self {
            id: next_id(),
            tag: 0,
            width: 40,
            full: '▌',
            full_style: Style::default().fg(Color::Rgb(117, 113, 249)),
            empty: '░',
            empty_style: Style::default().fg(Color::Rgb(96, 96, 96)),
            show_percentage: true,
            percent_format: " %3.0f%%".into(),
            percentage_style: Style::default(),
            percent_shown: 0.0,
            target_percent: 0.0,
            velocity: 0.0,
            scale_blend: false,
            color_func: None,
        };
        for opt in opts {
            opt(&mut model);
        }
        model
    }

    pub fn set_width(&mut self, width: usize) { self.width = width; }
    pub fn width(&self) -> usize { self.width }
    pub fn frame_msg(&self) -> FrameMsg { FrameMsg { id: self.id, tag: self.tag } }

    pub fn set_percent(&mut self, percent: f64) {
        self.target_percent = percent.clamp(0.0, 1.0);
        self.percent_shown = self.target_percent;
        self.tag += 1;
    }

    pub fn percent(&self) -> f64 { self.target_percent }

    pub fn update(&mut self, msg: FrameMsg) -> bool {
        if msg.id != self.id { return false; }
        if msg.tag > 0 && msg.tag != self.tag { return false; }
        self.percent_shown = self.target_percent;
        true
    }

    pub fn view(&self) -> String {
        let pct = self.percent_shown.clamp(0.0, 1.0);
        let bar_width = self.bar_width();
        let filled = ((bar_width as f64) * pct).round() as usize;
        let mut bar = String::new();
        for idx in 0..bar_width {
            if idx < filled {
                bar.push(self.full);
            } else {
                bar.push(self.empty);
            }
        }
        if self.show_percentage {
            format!("{bar}{}", format_percent(&self.percent_format, pct * 100.0))
        } else {
            bar
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        buf.set_line(area.x, area.y, &Line::styled(self.view(), Style::default()), area.width);
    }

    fn bar_width(&self) -> usize {
        if self.show_percentage {
            self.width.saturating_sub(format_percent(&self.percent_format, 100.0).len())
        } else {
            self.width
        }
    }
}

pub fn with_width(width: usize) -> Opt { Box::new(move |m| m.width = width) }
pub fn without_percentage() -> Opt { Box::new(|m| m.show_percentage = false) }
pub fn with_fill_characters(full: char, empty: char) -> Opt { Box::new(move |m| { m.full = full; m.empty = empty; }) }
pub fn with_color_func(f: ColorFunc) -> Opt { Box::new(move |m| m.color_func = Some(Arc::clone(&f))) }
pub fn with_scaled(enabled: bool) -> Opt { Box::new(move |m| m.scale_blend = enabled) }
pub fn with_spring_options(_frequency: f64, _damping: f64) -> Opt { Box::new(|_| {}) }
pub fn with_default_blend() -> Opt { Box::new(|_| {}) }
pub fn with_colors(_colors: Vec<Color>) -> Opt { Box::new(|_| {}) }

fn format_percent(fmt: &str, value: f64) -> String {
    if fmt == " %3.0f%%" {
        format!(" {:>3.0}%", value)
    } else {
        format!("{value:.0}%")
    }
}
