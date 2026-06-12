use std::{
    sync::atomic::{AtomicI64, Ordering},
    time::{Duration, Instant},
};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

pub type Opt = Box<dyn Fn(&mut Model)>;

#[derive(Clone, Copy, Debug)]
pub struct TickMsg {
    pub id: i64,
    pub tag: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct StartStopMsg {
    pub id: i64,
    pub running: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ResetMsg {
    pub id: i64,
}

#[derive(Clone, Debug)]
pub struct Model {
    elapsed: Duration,
    id: i64,
    tag: usize,
    running: bool,
    pub interval: Duration,
}

impl Model {
    pub fn new(opts: impl IntoIterator<Item = Opt>) -> Self {
        let mut model = Self {
            elapsed: Duration::ZERO,
            id: next_id(),
            tag: 0,
            running: false,
            interval: Duration::from_secs(1),
        };
        for opt in opts {
            opt(&mut model);
        }
        model
    }

    pub fn id(&self) -> i64 {
        self.id
    }
    pub fn start(&self) -> StartStopMsg {
        StartStopMsg {
            id: self.id,
            running: true,
        }
    }
    pub fn stop(&self) -> StartStopMsg {
        StartStopMsg {
            id: self.id,
            running: false,
        }
    }
    pub fn toggle(&self) -> StartStopMsg {
        StartStopMsg {
            id: self.id,
            running: !self.running,
        }
    }
    pub fn reset(&self) -> ResetMsg {
        ResetMsg { id: self.id }
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }
    pub fn view(&self) -> String {
        format_duration(self.elapsed)
    }
    pub fn tick_msg(&self) -> TickMsg {
        TickMsg {
            id: self.id,
            tag: self.tag,
        }
    }
    pub fn next_tick_at(&self, now: Instant) -> Instant {
        now + self.interval
    }

    pub fn update_start_stop(&mut self, msg: StartStopMsg) {
        if msg.id != self.id {
            return;
        }
        self.running = msg.running;
    }

    pub fn update_reset(&mut self, msg: ResetMsg) {
        if msg.id != self.id {
            return;
        }
        self.elapsed = Duration::ZERO;
    }

    pub fn update_tick(&mut self, msg: TickMsg) -> bool {
        if !self.running || msg.id != self.id {
            return false;
        }
        if msg.tag > 0 && msg.tag != self.tag {
            return false;
        }
        self.elapsed += self.interval;
        self.tag += 1;
        true
    }
}

pub fn with_interval(interval: Duration) -> Opt {
    Box::new(move |m| m.interval = interval)
}

fn format_duration(d: Duration) -> String {
    let total = d.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else if m > 0 {
        format!("{m}m{s:02}s")
    } else {
        format!("{s}s")
    }
}
