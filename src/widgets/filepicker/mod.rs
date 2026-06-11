use std::{fs, path::{Path, PathBuf}, sync::atomic::{AtomicI64, Ordering}};

use crossterm::event::KeyEvent;
use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Modifier, Style}, text::{Line, Span}};

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
            go_to_last: Binding::new([key::with_keys(&["G"]), key::with_help("G", "last")]),
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

#[derive(Clone, Debug)]
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

impl Default for Styles {
    fn default() -> Self {
        Self {
            disabled_cursor: Style::default().fg(Color::Indexed(247)),
            cursor: Style::default().fg(Color::Indexed(212)),
            symlink: Style::default().fg(Color::Indexed(36)),
            directory: Style::default().fg(Color::Indexed(99)),
            file: Style::default(),
            disabled_file: Style::default().fg(Color::Indexed(243)),
            permission: Style::default().fg(Color::Indexed(244)),
            selected: Style::default().fg(Color::Indexed(212)).add_modifier(Modifier::BOLD),
            disabled_selected: Style::default().fg(Color::Indexed(247)),
            file_size: Style::default().fg(Color::Indexed(240)),
            empty_directory: Style::default().fg(Color::Indexed(240)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub symlink_path: Option<PathBuf>,
    pub permissions: String,
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
    selected_stack: Vec<usize>,
    min_idx: usize,
    max_idx: usize,
    min_stack: Vec<usize>,
    max_stack: Vec<usize>,
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
            selected_stack: Vec::new(),
            min_idx: 0,
            max_idx: 0,
            min_stack: Vec::new(),
            max_stack: Vec::new(),
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
                let symlink_meta = fs::symlink_metadata(&path).ok()?;
                let is_symlink = symlink_meta.file_type().is_symlink();
                let symlink_path = if is_symlink { fs::canonicalize(&path).ok() } else { None };
                let meta = entry.metadata().ok()?;
                Some(FileEntry {
                    name,
                    path,
                    is_dir: meta.is_dir(),
                    is_symlink,
                    symlink_path,
                    permissions: permissions_string(&symlink_meta),
                    size: meta.len(),
                })
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
            if let (Some(selected), Some(min_idx), Some(max_idx)) = (self.selected_stack.pop(), self.min_stack.pop(), self.max_stack.pop()) {
                self.selected = selected;
                self.min_idx = min_idx;
                self.max_idx = max_idx;
            } else {
                self.selected = 0;
                self.min_idx = 0;
                self.max_idx = self.height.saturating_sub(1);
            }
            self.read_dir()?;
        } else if key::matches(event, [&self.key_map.open]) {
            if let Some(entry) = self.selected() {
                if entry.is_dir {
                    self.current_directory = entry.path.clone();
                    self.selected_stack.push(self.selected);
                    self.min_stack.push(self.min_idx);
                    self.max_stack.push(self.max_idx);
                    self.selected = 0;
                    self.min_idx = 0;
                    self.max_idx = self.height.saturating_sub(1);
                    self.read_dir()?;
                } else if self.file_allowed && self.entry_allowed(entry) {
                    let path = entry.path.clone();
                    let name = entry.name.clone();
                    self.path = path;
                    self.file_selected = name;
                }
            }
        } else if key::matches(event, [&self.key_map.select]) {
            if let Some(entry) = self.selected() {
                if (entry.is_dir && self.dir_allowed) || (!entry.is_dir && self.file_allowed && self.entry_allowed(entry)) {
                    let path = entry.path.clone();
                    let name = entry.name.clone();
                    self.path = path;
                    self.file_selected = name;
                }
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
            let disabled = !entry.is_dir && !self.entry_allowed(entry);
            let selected = real_idx == self.selected;
            let cursor_style = if disabled { self.styles.disabled_cursor } else { self.styles.cursor };
            let row_style = if selected {
                if disabled { self.styles.disabled_selected } else { self.styles.selected }
            } else if entry.is_dir {
                self.styles.directory
            } else if entry.is_symlink {
                self.styles.symlink
            } else if disabled {
                self.styles.disabled_file
            } else {
                self.styles.file
            };

            let mut spans = vec![
                Span::styled(if selected { self.cursor.clone() } else { " ".to_string() }, cursor_style),
            ];
            if self.show_permissions {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(entry.permissions.clone(), if selected { row_style } else { self.styles.permission }));
            }
            if self.show_size {
                spans.push(Span::styled(format!("{:>8}", human_size(entry.size)), if selected { row_style } else { self.styles.file_size }));
            }
            spans.push(Span::raw(" "));
            spans.push(Span::styled(entry.name.clone(), row_style));
            if let Some(path) = &entry.symlink_path {
                spans.push(Span::styled(format!(" → {}", path.display()), self.styles.symlink));
            }
            Line::from(spans)
        }).collect()
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (i, line) in self.view().into_iter().take(area.height as usize).enumerate() {
            buf.set_line(area.x, area.y + i as u16, &line, area.width);
        }
    }

    pub fn did_select_file(&self, event: &KeyEvent) -> std::option::Option<PathBuf> {
        if self.files.is_empty() || !key::matches(event, [&self.key_map.select]) {
            return None;
        }
        let entry = self.selected()?;
        if !entry.is_dir && self.file_allowed && self.entry_allowed(entry) && !self.path.as_os_str().is_empty() {
            return Some(self.path.clone());
        }
        None
    }

    pub fn did_select_disabled_file(&self, event: &KeyEvent) -> std::option::Option<PathBuf> {
        if self.files.is_empty() || !key::matches(event, [&self.key_map.select]) {
            return None;
        }
        let entry = self.selected()?;
        if !entry.is_dir && self.file_allowed && !self.entry_allowed(entry) {
            return Some(entry.path.clone());
        }
        None
    }

    pub fn highlighted_path(&self) -> PathBuf {
        self.selected().map(|e| e.path.clone()).unwrap_or_default()
    }
}

impl Model {
    fn entry_allowed(&self, entry: &FileEntry) -> bool {
        if entry.is_dir {
            return self.dir_allowed;
        }
        if self.allowed_types.is_empty() {
            return true;
        }
        entry.path.extension().and_then(|ext| ext.to_str()).map(|ext| {
            let ext = format!(".{}", ext);
            self.allowed_types.iter().any(|allowed| allowed.eq_ignore_ascii_case(&ext))
        }).unwrap_or(false)
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

#[cfg(unix)]
fn permissions_string(meta: &fs::Metadata) -> String {
    use std::os::unix::fs::PermissionsExt;

    let mode = meta.permissions().mode();
    let kind = if meta.file_type().is_dir() { 'd' } else if meta.file_type().is_symlink() { 'l' } else { '-' };
    let mut out = String::with_capacity(10);
    out.push(kind);
    for shift in [6, 3, 0] {
        out.push(if mode & (0o4 << shift) != 0 { 'r' } else { '-' });
        out.push(if mode & (0o2 << shift) != 0 { 'w' } else { '-' });
        out.push(if mode & (0o1 << shift) != 0 { 'x' } else { '-' });
    }
    out
}

#[cfg(not(unix))]
fn permissions_string(meta: &fs::Metadata) -> String {
    if meta.permissions().readonly() {
        "r--r--r--".to_string()
    } else {
        "rw-rw-rw-".to_string()
    }
}
