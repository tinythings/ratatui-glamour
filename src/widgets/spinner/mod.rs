use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, Instant},
};

use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

#[derive(Clone, Debug)]
pub struct Spinner {
    pub frames: Vec<String>,
    pub fps: Duration,
}

#[derive(Clone, Debug)]
pub struct Model {
    pub spinner: Spinner,
    pub style: Style,
    frame: usize,
    id: i64,
    tag: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct Tick {
    pub at: Instant,
    pub tag: usize,
    pub id: i64,
}

impl Spinner {
    pub fn line() -> Self {
        Self {
            frames: vec!["|".into(), "/".into(), "-".into(), "\\".into()],
            fps: Duration::from_millis(100),
        }
    }
    pub fn dot() -> Self {
        Self {
            frames: vec![
                "⣾ ".into(),
                "⣽ ".into(),
                "⣻ ".into(),
                "⢿ ".into(),
                "⡿ ".into(),
                "⣟ ".into(),
                "⣯ ".into(),
                "⣷ ".into(),
            ],
            fps: Duration::from_millis(100),
        }
    }
    pub fn mini_dot() -> Self {
        Self {
            frames: vec![
                "⠋".into(),
                "⠙".into(),
                "⠹".into(),
                "⠸".into(),
                "⠼".into(),
                "⠴".into(),
                "⠦".into(),
                "⠧".into(),
                "⠇".into(),
                "⠏".into(),
            ],
            fps: Duration::from_millis(83),
        }
    }
    pub fn jump() -> Self {
        Self {
            frames: vec![
                "⢄".into(),
                "⢂".into(),
                "⢁".into(),
                "⡁".into(),
                "⡈".into(),
                "⡐".into(),
                "⡠".into(),
            ],
            fps: Duration::from_millis(100),
        }
    }
    pub fn pulse() -> Self {
        Self {
            frames: vec!["█".into(), "▓".into(), "▒".into(), "░".into()],
            fps: Duration::from_millis(125),
        }
    }
    pub fn points() -> Self {
        Self {
            frames: vec!["∙∙∙".into(), "●∙∙".into(), "∙●∙".into(), "∙∙●".into()],
            fps: Duration::from_millis(142),
        }
    }
    pub fn globe() -> Self {
        Self {
            frames: vec!["🌍".into(), "🌎".into(), "🌏".into()],
            fps: Duration::from_millis(250),
        }
    }
    pub fn moon() -> Self {
        Self {
            frames: vec![
                "🌑".into(),
                "🌒".into(),
                "🌓".into(),
                "🌔".into(),
                "🌕".into(),
                "🌖".into(),
                "🌗".into(),
                "🌘".into(),
            ],
            fps: Duration::from_millis(125),
        }
    }
    pub fn monkey() -> Self {
        Self {
            frames: vec!["🙈".into(), "🙉".into(), "🙊".into()],
            fps: Duration::from_millis(333),
        }
    }
    pub fn meter() -> Self {
        Self {
            frames: vec![
                "▱▱▱".into(),
                "▰▱▱".into(),
                "▰▰▱".into(),
                "▰▰▰".into(),
                "▰▰▱".into(),
                "▰▱▱".into(),
                "▱▱▱".into(),
            ],
            fps: Duration::from_millis(142),
        }
    }
    pub fn hamburger() -> Self {
        Self {
            frames: vec!["☱".into(), "☲".into(), "☴".into(), "☲".into()],
            fps: Duration::from_millis(333),
        }
    }
    pub fn ellipsis() -> Self {
        Self {
            frames: vec!["".into(), ".".into(), "..".into(), "...".into()],
            fps: Duration::from_millis(333),
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            spinner: Spinner::line(),
            style: Style::default(),
            frame: 0,
            id: next_id(),
            tag: 0,
        }
    }

    pub fn id(&self) -> i64 {
        self.id
    }

    pub fn tick(&self) -> Tick {
        Tick {
            at: Instant::now(),
            tag: self.tag,
            id: self.id,
        }
    }

    pub fn update(&mut self, msg: Tick) {
        if msg.id > 0 && msg.id != self.id {
            return;
        }
        if msg.tag > 0 && msg.tag != self.tag {
            return;
        }
        self.frame += 1;
        if self.frame >= self.spinner.frames.len() {
            self.frame = 0;
        }
        self.tag += 1;
    }

    pub fn view(&self) -> Line<'static> {
        if self.frame >= self.spinner.frames.len() {
            return Line::styled("(error)", self.style);
        }
        Line::styled(self.spinner.frames[self.frame].clone(), self.style)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        buf.set_line(area.x, area.y, &self.view(), area.width);
    }
}
