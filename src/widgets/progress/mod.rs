use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
};

use crate::color::blend_1d;

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
    colors: Vec<Color>,
    spring_frequency: f64,
    spring_damping: f64,
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
            colors: Vec::new(),
            spring_frequency: 18.0,
            spring_damping: 1.0,
        };
        for opt in opts {
            opt(&mut model);
        }
        model
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn frame_msg(&self) -> FrameMsg {
        FrameMsg {
            id: self.id,
            tag: self.tag,
        }
    }
    pub fn is_animating(&self) -> bool {
        let dist = (self.percent_shown - self.target_percent).abs();
        !(dist < 0.001 && self.velocity.abs() < 0.01)
    }

    pub fn set_percent(&mut self, percent: f64) {
        self.target_percent = percent.clamp(0.0, 1.0);
        self.tag += 1;
    }

    pub fn incr_percent(&mut self, delta: f64) {
        self.set_percent(self.percent() + delta);
    }

    pub fn decr_percent(&mut self, delta: f64) {
        self.set_percent(self.percent() - delta);
    }

    pub fn set_spring_options(&mut self, frequency: f64, damping: f64) {
        self.spring_frequency = frequency.max(0.0001);
        self.spring_damping = damping.max(0.0);
    }

    pub fn set_colors(&mut self, colors: Vec<Color>) {
        self.colors = colors;
        if self.colors.len() == 1 {
            self.full_style = self.full_style.fg(self.colors[0]);
        }
    }

    pub fn percent(&self) -> f64 {
        self.target_percent
    }

    pub fn view_as(&self, percent: f64) -> String {
        let pct = percent.clamp(0.0, 1.0);
        let bar_width = self.bar_width_for(self.width);
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

    pub fn update(&mut self, msg: FrameMsg) -> bool {
        if msg.id != self.id {
            return false;
        }
        if msg.tag > 0 && msg.tag != self.tag {
            return false;
        }
        if !self.is_animating() {
            self.percent_shown = self.target_percent;
            self.velocity = 0.0;
            return false;
        }
        let stiffness = self.spring_frequency / 60.0;
        let delta = self.target_percent - self.percent_shown;
        self.velocity = (self.velocity + delta * stiffness)
            * (1.0 - (self.spring_damping / 60.0)).clamp(0.0, 1.0);
        self.percent_shown = (self.percent_shown + self.velocity).clamp(0.0, 1.0);
        true
    }

    pub fn view(&self) -> String {
        self.view_as(self.percent_shown)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let pct = self.percent_shown.clamp(0.0, 1.0);
        let total_width = area.width as usize;
        let bar_width = self.bar_width_for(total_width);
        let filled = ((bar_width as f64) * pct).round() as usize;
        let is_half_block = self.full == '▌';
        let blend_multiplier = if is_half_block { 2 } else { 1 };
        let blend_steps =
            if self.scale_blend { filled } else { bar_width }.saturating_mul(blend_multiplier);
        let blend = if self.colors.len() > 1 {
            blend_1d(blend_steps.max(1), &self.colors)
        } else {
            Vec::new()
        };

        for idx in 0..bar_width.min(area.width as usize) {
            let x = area.x + idx as u16;
            let style = if idx < filled {
                self.fill_style(idx, bar_width, filled, &blend, is_half_block)
            } else {
                self.empty_style
            };
            let symbol = if idx < filled { self.full } else { self.empty }.to_string();
            if let Some(cell) = buf.cell_mut((x, area.y)) {
                cell.set_symbol(&symbol);
                cell.set_style(style);
            }
        }

        let suffix = if self.show_percentage {
            format_percent(&self.percent_format, pct * 100.0)
        } else {
            String::new()
        };
        if !suffix.is_empty() {
            let suffix_x = area.x + bar_width.min(area.width as usize) as u16;
            if suffix_x < area.right() {
                buf.set_line(
                    suffix_x,
                    area.y,
                    &Line::styled(suffix, self.percentage_style),
                    area.right().saturating_sub(suffix_x),
                );
            }
        }
    }

    fn bar_width_for(&self, total_width: usize) -> usize {
        if self.show_percentage {
            total_width.saturating_sub(format_percent(&self.percent_format, 100.0).len())
        } else {
            total_width
        }
    }

    fn fill_style(
        &self,
        idx: usize,
        bar_width: usize,
        filled: usize,
        blend: &[Color],
        is_half_block: bool,
    ) -> Style {
        if let Some(color_func) = &self.color_func {
            let current = if bar_width > 0 {
                idx as f64 / bar_width as f64
            } else {
                0.0
            };
            let mut style = self.full_style.fg(color_func(self.percent_shown, current));
            if is_half_block {
                let next = if bar_width > 0 {
                    ((idx as f64) + 0.5) / bar_width as f64
                } else {
                    0.0
                };
                style = style.bg(color_func(self.percent_shown, next.min(1.0)));
            }
            return style;
        }

        if !blend.is_empty() {
            if is_half_block {
                let blend_index = idx * 2;
                return self
                    .full_style
                    .fg(blend[blend_index.min(blend.len() - 1)])
                    .bg(blend[(blend_index + 1).min(blend.len() - 1)]);
            }

            let blend_index = if self.scale_blend {
                if filled <= 1 { 0 } else { idx.min(filled - 1) }
            } else {
                idx.min(blend.len() - 1)
            };
            return self.full_style.fg(blend[blend_index]);
        }

        self.full_style
    }
}

pub fn with_width(width: usize) -> Opt {
    Box::new(move |m| m.width = width)
}
pub fn without_percentage() -> Opt {
    Box::new(|m| m.show_percentage = false)
}
pub fn with_fill_characters(full: char, empty: char) -> Opt {
    Box::new(move |m| {
        m.full = full;
        m.empty = empty;
    })
}
pub fn with_color_func(f: ColorFunc) -> Opt {
    Box::new(move |m| m.color_func = Some(Arc::clone(&f)))
}
pub fn with_scaled(enabled: bool) -> Opt {
    Box::new(move |m| m.scale_blend = enabled)
}
pub fn with_spring_options(frequency: f64, damping: f64) -> Opt {
    Box::new(move |m| m.set_spring_options(frequency, damping))
}
pub fn with_default_blend() -> Opt {
    Box::new(|m| m.colors = vec![Color::Rgb(0x5A, 0x56, 0xE0), Color::Rgb(0xEE, 0x6F, 0xF8)])
}
pub fn with_colors(colors: Vec<Color>) -> Opt {
    Box::new(move |m| {
        m.colors = colors.clone();
        if m.colors.len() == 1 {
            m.full_style = Style::default().fg(m.colors[0]);
        }
    })
}

fn format_percent(fmt: &str, value: f64) -> String {
    if fmt == " %3.0f%%" {
        format!(" {:>3.0}%", value)
    } else {
        format!("{value:.0}%")
    }
}
