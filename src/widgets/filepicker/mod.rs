use std::{fs, path::{Path, PathBuf}, sync::atomic::{AtomicI64, Ordering}};

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect, style::Style, text::Line};

use crate::widgets::key::{self, Binding};

static LAST_ID: AtomicI64 = AtomicI64::new(0);

fn next_id() -> i64 {
    LAST_ID.fetch_add(1, Ordering::Relaxed) + 1
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub go_to_top: Binding,
    pub go_to_last: Binding,
    pub down: Binding,
    pub up: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub back: Binding,
    pub open: Binding,
    pub select: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            go_to_top: Binding::new([key::with_keys(&["g"]), key::with_help("g", "first")]),
            go_to_last: Binding::new([key::with_keys(&["g"]), key::with_help("G", "last")]),
            down: Binding::new([key::with_keys(&["j", "down", "ctrl+n"]), key::with_help("j", "down")]),
            up: Binding::new([key::with_keys(&["k", "up", "ctrl+p"]), key::with_help("k", "up")]),
            page_up: Binding::new([key::with_keys(&["K", "pgup"]), key::with_help("pgup", "page up")]),
            page_down: Binding::new([key::with_keys(&["J", "pgdown"]), key::with_help("pgdown", "page down")]),
            back: Binding::new([key::with_keys(&["h", "backspace", "left", "esc"]), key::with_help("h", "back")]),
            open: Binding::new([key::with_keys(&["l", "right", "enter"]), key::with_help("l", "open")]),
            select: Binding::new([key::with_keys(&["enter"]), key::with_help("enter", "select")]),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub disabled_cursor: Style,
    pub cursor: Style,
    pub symlink: Style,
    pub directory: Style,
    pub file: Style,
    pub disabled_file: Style,
    pub permission: Style,
    pub selected: Style,
    pub disabled_selected: Style,
    pub file_size: Style,
    pub empty_directory: Style,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub struct Model {
    id: i64,
    pub path: PathBuf,
    pub current_directory: PathBuf,
    pub allowed_types: Vec<String>,
    pub key_map: KeyMap,
    files: Vec<FileEntry>,
    pub show_permissions: bool,
    pub show_size: bool,
    pub show_hidden: bool,
    pub dir_allowed: bool,
    pub file_allowed: bool,
    pub file_selected: String,
    selected: usize,
    min_idx: usize,
    max_idx: usize,
    height: usize,
    pub auto_height: bool,
    pub cursor: String,
    pub styles: Styles,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    pub fn new() -> Self {
        Self {
            id: next_id(),
            path: PathBuf::new(),
            current_directory: PathBuf::from("."),
            allowed_types: Vec::new(),
            key_map: KeyMap::default(),
            files: Vec::new(),
            show_permissions: true,
            show_size: true,
            show_hidden: false,
            dir_allowed: false,
            file_allowed: true,
            file_selected: String::new(),
            selected: 0,
            min_idx: 0,
            max_idx: 0,
            height: 0,
            auto_height: true,
            cursor: ">".into(),
            styles: Styles::default(),
        }
    }

    pub fn id(&self) -> i64 { self.id }
    pub fn set_height(&mut self, h: usize) { self.height = h; if self.max_idx > self.height.saturating_sub(1) { self.max_idx = self.min_idx + self.height.saturating_sub(1); } }
    pub fn height(&self) -> usize { self.height }
    pub fn files(&self) -> &[FileEntry] { &self.files }
    pub fn selected_index(&self) -> usize { self.selected }
    pub fn selected(&self) -> Option<&FileEntry> { self.files.get(self.selected) }

    pub fn read_dir(&mut self) -> std::io::Result<()> {
        let mut entries: Vec<FileEntry> = fs::read_dir(&self.current_directory)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                if !self.show_hidden && is_hidden(&name) {
                    return None;
                }
                let path = entry.path();
                let meta = entry.metadata().ok()?;
                Some(FileEntry { name, path, is_dir: meta.is_dir(), size: meta.len() })
            })
            .collect();
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) { (true, false) => std::cmp::Ordering::Less, (false, true) => std::cmp::Ordering::Greater, _ => a.name.cmp(&b.name) });
        self.files = entries;
        self.selected = self.selected.min(self.files.len().saturating_sub(1));
        self.max_idx = self.max_idx.max(self.height.saturating_sub(1));
        Ok(())
    }

    pub fn handle_key(&mut self, event: &KeyEvent) -> std::io::Result<()> {
        if key::matches(event, [&self.key_map.go_to_top]) {
            self.selected = 0;
            self.min_idx = 0;
            self.max_idx = self.height.saturating_sub(1);
        } else if key::matches(event, [&self.key_map.go_to_last]) {
            self.selected = self.files.len().saturating_sub(1);
            self.min_idx = self.files.len().saturating_sub(self.height.max(1));
            self.max_idx = self.files.len().saturating_sub(1);
        } else if key::matches(event, [&self.key_map.down]) {
            self.selected = (self.selected + 1).min(self.files.len().saturating_sub(1));
            if self.selected > self.max_idx { self.min_idx += 1; self.max_idx += 1; }
        } else if key::matches(event, [&self.key_map.up]) {
            self.selected = self.selected.saturating_sub(1);
            if self.selected < self.min_idx { self.min_idx = self.min_idx.saturating_sub(1); self.max_idx = self.max_idx.saturating_sub(1); }
        } else if key::matches(event, [&self.key_map.page_down]) {
            let step = self.height.max(1);
            self.selected = (self.selected + step).min(self.files.len().saturating_sub(1));
            self.min_idx = (self.min_idx + step).min(self.files.len().saturating_sub(step));
            self.max_idx = (self.min_idx + step).min(self.files.len()).saturating_sub(1);
        } else if key::matches(event, [&self.key_map.page_up]) {
            let step = self.height.max(1);
            self.selected = self.selected.saturating_sub(step);
            self.min_idx = self.min_idx.saturating_sub(step);
            self.max_idx = self.min_idx.saturating_add(step).saturating_sub(1);
        } else if key::matches(event, [&self.key_map.back]) {
            self.current_directory = self.current_directory.parent().unwrap_or(Path::new("/")).to_path_buf();
            self.selected = 0;
            self.min_idx = 0;
            self.max_idx = self.height.saturating_sub(1);
            self.read_dir()?;
        } else if key::matches(event, [&self.key_map.open]) {
            if let Some(entry) = self.selected() {
                if entry.is_dir {
                    self.current_directory = entry.path.clone();
                    self.selected = 0;
                    self.min_idx = 0;
                    self.max_idx = self.height.saturating_sub(1);
                    self.read_dir()?;
                }
            }
        } else if key::matches(event, [&self.key_map.select]) {
            if let Some(entry) = self.selected() {
                let path = entry.path.clone();
                let name = entry.name.clone();
                self.path = path;
                self.file_selected = name;
            }
        }
        Ok(())
    }

    pub fn view(&self) -> Vec<Line<'static>> {
        if self.files.is_empty() {
            return vec![Line::styled("  Bummer. No Files Found.", self.styles.empty_directory)];
        }
        let end = if self.height == 0 { self.files.len() } else { self.max_idx.min(self.files.len().saturating_sub(1)) + 1 };
        self.files[self.min_idx..end].iter().enumerate().map(|(idx, entry)| {
            let real_idx = self.min_idx + idx;
            let prefix = if real_idx == self.selected { &self.cursor } else { " " };
            let suffix = if self.show_size && !entry.is_dir { format!(" {:>7}", human_size(entry.size)) } else { String::new() };
            let style = if real_idx == self.selected { self.styles.selected } else if entry.is_dir { self.styles.directory } else { self.styles.file };
            Line::styled(format!("{prefix} {}{suffix}", entry.name), style)
        }).collect()
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (i, line) in self.view().into_iter().take(area.height as usize).enumerate() {
            buf.set_line(area.x, area.y + i as u16, &line, area.width);
        }
    }
}

fn is_hidden(file: &str) -> bool {
    file.starts_with('.')
}

fn human_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut val = size as f64;
    let mut idx = 0;
    while val >= 1024.0 && idx + 1 < UNITS.len() {
        val /= 1024.0;
        idx += 1;
    }
    if idx == 0 { format!("{}{}", size, UNITS[idx]) } else { format!("{val:.1}{}", UNITS[idx]) }
}
