use std::any::Any;

use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::widgets::{help::{self, KeyMap as _}, key::{self, Binding}, paginator, spinner, textinput};

pub trait Item: Any {
    fn filter_value(&self) -> String;
}

pub trait ItemDelegate {
    fn render(&self, item: &dyn Item, selected: bool, width: usize) -> Vec<Line<'static>>;
    fn height(&self) -> usize { 1 }
    fn spacing(&self) -> usize { 0 }
}

pub trait DefaultItem: Item {
    fn title(&self) -> String;
    fn description(&self) -> String;
}

#[derive(Clone, Debug)]
pub struct DefaultListItem {
    pub title: String,
    pub description: String,
    pub filter_value: String,
}

impl Item for DefaultListItem {
    fn filter_value(&self) -> String {
        self.filter_value.clone()
    }
}

impl DefaultItem for DefaultListItem {
    fn title(&self) -> String {
        self.title.clone()
    }

    fn description(&self) -> String {
        self.description.clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FilterState {
    Unfiltered,
    Filtering,
    FilterApplied,
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub cursor_up: Binding,
    pub cursor_down: Binding,
    pub next_page: Binding,
    pub prev_page: Binding,
    pub go_to_start: Binding,
    pub go_to_end: Binding,
    pub filter: Binding,
    pub clear_filter: Binding,
    pub cancel_while_filtering: Binding,
    pub accept_while_filtering: Binding,
    pub show_full_help: Binding,
    pub close_full_help: Binding,
    pub quit: Binding,
    pub force_quit: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            cursor_up: Binding::new([key::with_keys(&["up", "k"]), key::with_help("↑/k", "up")]),
            cursor_down: Binding::new([key::with_keys(&["down", "j"]), key::with_help("↓/j", "down")]),
            prev_page: Binding::new([key::with_keys(&["left", "h", "pgup", "b", "u"]), key::with_help("←/h/pgup", "prev page")]),
            next_page: Binding::new([key::with_keys(&["right", "l", "pgdown", "f", "d"]), key::with_help("→/l/pgdn", "next page")]),
            go_to_start: Binding::new([key::with_keys(&["home", "g"]), key::with_help("g/home", "go to start")]),
            go_to_end: Binding::new([key::with_keys(&["end", "G"]), key::with_help("G/end", "go to end")]),
            filter: Binding::new([key::with_keys(&["/"]), key::with_help("/", "filter")]),
            clear_filter: Binding::new([key::with_keys(&["esc"]), key::with_help("esc", "clear filter")]),
            cancel_while_filtering: Binding::new([key::with_keys(&["esc"]), key::with_help("esc", "cancel")]),
            accept_while_filtering: Binding::new([key::with_keys(&["enter", "tab", "shift+tab", "ctrl+k", "up", "ctrl+j", "down"]), key::with_help("enter", "apply filter")]),
            show_full_help: Binding::new([key::with_keys(&["?"]), key::with_help("?", "more")]),
            close_full_help: Binding::new([key::with_keys(&["?"]), key::with_help("?", "close help")]),
            quit: Binding::new([key::with_keys(&["q", "esc"]), key::with_help("q", "quit")]),
            force_quit: Binding::new([key::with_keys(&["ctrl+c"])]),
        }
    }
}

impl help::KeyMap for KeyMap {
    fn short_help(&self) -> Vec<Binding> {
        vec![self.cursor_up.clone(), self.cursor_down.clone(), self.filter.clone(), self.show_full_help.clone()]
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        vec![
            vec![self.cursor_up.clone(), self.cursor_down.clone(), self.prev_page.clone(), self.next_page.clone()],
            vec![self.go_to_start.clone(), self.go_to_end.clone(), self.filter.clone(), self.clear_filter.clone()],
            vec![self.show_full_help.clone(), self.quit.clone()],
        ]
    }
}

#[derive(Clone, Debug)]
pub struct Styles {
    pub title_bar: Style,
    pub title: Style,
    pub spinner: Style,
    pub default_filter_character_match: Style,
    pub status_bar: Style,
    pub status_empty: Style,
    pub status_bar_active_filter: Style,
    pub status_bar_filter_count: Style,
    pub no_items: Style,
    pub pagination_style: Style,
    pub help_style: Style,
    pub active_pagination_dot: Style,
    pub inactive_pagination_dot: Style,
    pub arabic_pagination: Style,
    pub divider_dot: Style,
    pub filter: textinput::Styles,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            title_bar: Style::default(),
            title: Style::default().fg(Color::Indexed(230)).bg(Color::Indexed(62)).add_modifier(Modifier::BOLD),
            spinner: Style::default().fg(Color::Indexed(243)),
            default_filter_character_match: Style::default().add_modifier(Modifier::UNDERLINED),
            status_bar: Style::default().fg(Color::Indexed(245)),
            status_empty: Style::default().fg(Color::Indexed(243)),
            status_bar_active_filter: Style::default().fg(Color::Indexed(252)),
            status_bar_filter_count: Style::default().fg(Color::Indexed(240)),
            no_items: Style::default().fg(Color::Indexed(243)),
            pagination_style: Style::default().fg(Color::Indexed(245)),
            help_style: Style::default().fg(Color::Indexed(245)),
            active_pagination_dot: Style::default().fg(Color::Indexed(245)),
            inactive_pagination_dot: Style::default().fg(Color::Indexed(238)),
            arabic_pagination: Style::default().fg(Color::Indexed(245)),
            divider_dot: Style::default().fg(Color::Indexed(238)),
            filter: textinput::Styles::default(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DefaultItemStyles {
    pub normal_title: Style,
    pub normal_desc: Style,
    pub selected_title: Style,
    pub selected_desc: Style,
    pub dimmed_title: Style,
    pub dimmed_desc: Style,
    pub filter_match: Style,
}

impl Default for DefaultItemStyles {
    fn default() -> Self {
        Self {
            normal_title: Style::default().fg(Color::Indexed(252)),
            normal_desc: Style::default().fg(Color::Indexed(245)),
            selected_title: Style::default().fg(Color::Indexed(213)).add_modifier(Modifier::BOLD),
            selected_desc: Style::default().fg(Color::Indexed(177)),
            dimmed_title: Style::default().fg(Color::Indexed(245)),
            dimmed_desc: Style::default().fg(Color::Indexed(240)),
            filter_match: Style::default().add_modifier(Modifier::UNDERLINED),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DefaultDelegate {
    pub show_description: bool,
    pub styles: DefaultItemStyles,
    height: usize,
    spacing: usize,
}

impl Default for DefaultDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultDelegate {
    pub fn new() -> Self {
        Self {
            show_description: true,
            styles: DefaultItemStyles::default(),
            height: 2,
            spacing: 1,
        }
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
    }

    pub fn set_spacing(&mut self, spacing: usize) {
        self.spacing = spacing;
    }
}

impl ItemDelegate for DefaultDelegate {
    fn render(&self, item: &dyn Item, selected: bool, width: usize) -> Vec<Line<'static>> {
        let Some(item) = (item as &dyn Any).downcast_ref::<DefaultListItem>() else {
            return vec![Line::styled(truncate_ellipsis(&item.filter_value(), width), Style::default())];
        };
        let title = truncate_ellipsis(&item.title(), width.saturating_sub(2));
        let desc = truncate_ellipsis(&item.description(), width.saturating_sub(2));
        let (title_style, desc_style) = if selected {
            (self.styles.selected_title, self.styles.selected_desc)
        } else {
            (self.styles.normal_title, self.styles.normal_desc)
        };
        if self.show_description {
            vec![
                Line::styled(format!("  {title}"), title_style),
                Line::styled(format!("  {desc}"), desc_style),
            ]
        } else {
            vec![Line::styled(format!("  {title}"), title_style)]
        }
    }

    fn height(&self) -> usize {
        if self.show_description { self.height } else { 1 }
    }

    fn spacing(&self) -> usize {
        self.spacing
    }
}

fn truncate_ellipsis(s: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    let take = width.saturating_sub(1);
    let mut out: String = chars.into_iter().take(take).collect();
    out.push('…');
    out
}

pub struct Model<T: Item, D: ItemDelegate> {
    pub show_title: bool,
    pub show_filter: bool,
    pub show_status_bar: bool,
    pub show_pagination: bool,
    pub show_help: bool,
    pub filtering_enabled: bool,
    pub item_name_singular: String,
    pub item_name_plural: String,
    pub title: String,
    pub styles: Styles,
    pub infinite_scrolling: bool,
    pub key_map: KeyMap,
    pub spinner: spinner::Model,
    pub show_spinner: bool,
    pub width: usize,
    pub height: usize,
    pub paginator: paginator::Model,
    pub cursor: usize,
    pub help: help::Model,
    pub filter_input: textinput::Model,
    pub filter_state: FilterState,
    pub status_message: String,
    items: Vec<T>,
    filtered_indices: Vec<usize>,
    delegate: D,
}

impl<T: Item, D: ItemDelegate> Model<T, D> {
    pub fn new(items: Vec<T>, delegate: D, width: usize, height: usize) -> Self {
        let mut spinner_model = spinner::Model::new();
        spinner_model.spinner = spinner::Spinner::line();
        let mut filter_input = textinput::Model::new();
        filter_input.prompt = "Filter: ".to_string();
        filter_input.char_limit = 64;
        filter_input.focus();
        let mut paginator = paginator::Model::new([]);
        paginator.r#type = paginator::Type::Dots;
        paginator.active_dot = "•".into();
        paginator.inactive_dot = "•".into();
        let mut model = Self {
            show_title: true,
            show_filter: true,
            show_status_bar: true,
            show_pagination: true,
            show_help: true,
            filtering_enabled: true,
            item_name_singular: "item".into(),
            item_name_plural: "items".into(),
            title: "List".into(),
            styles: Styles::default(),
            infinite_scrolling: false,
            key_map: KeyMap::default(),
            spinner: spinner_model,
            show_spinner: false,
            width,
            height,
            paginator,
            cursor: 0,
            help: help::Model::new(),
            filter_input,
            filter_state: FilterState::Unfiltered,
            status_message: String::new(),
            items,
            filtered_indices: Vec::new(),
            delegate,
        };
        model.rebuild_filter();
        model.update_pagination();
        model
    }

    pub fn items(&self) -> &[T] { &self.items }
    pub fn set_items(&mut self, items: Vec<T>) { self.items = items; self.cursor = 0; self.rebuild_filter(); self.update_pagination(); self.update_keybindings(); }
    pub fn set_filter_text(&mut self, filter: &str) {
        self.filter_state = FilterState::Filtering;
        self.filter_input.set_value(filter);
        self.rebuild_filter();
        self.filter_state = FilterState::FilterApplied;
        self.go_to_start();
        self.filter_input.cursor_end();
        self.update_pagination();
        self.update_keybindings();
    }
    pub fn set_filter_state(&mut self, state: FilterState) {
        self.go_to_start();
        self.filter_state = state;
        self.filter_input.cursor_end();
        self.filter_input.focus();
        self.update_keybindings();
    }
    pub fn select(&mut self, index: usize) {
        let page_size = self.page_size().max(1);
        let bounded = index.min(self.filtered_len().saturating_sub(1));
        self.paginator.page = bounded / page_size;
        self.cursor = bounded % page_size;
    }
    pub fn reset_selected(&mut self) { self.cursor = 0; self.update_pagination_for_cursor(); }
    pub fn reset_filter(&mut self) { self.reset_filtering(); }
    pub fn selected_item(&self) -> Option<&T> {
        if self.filter_state == FilterState::Unfiltered {
            return self.items.get(self.index());
        }
        self.filtered_indices.get(self.index()).and_then(|idx| self.items.get(*idx))
    }
    pub fn index(&self) -> usize { self.paginator.page * self.page_size().max(1) + self.cursor }
    pub fn global_index(&self) -> usize {
        if self.filter_state == FilterState::Unfiltered {
            return self.index();
        }
        self.filtered_indices.get(self.index()).copied().unwrap_or(self.index())
    }
    pub fn cursor(&self) -> usize { self.cursor }
    pub fn filter_state(&self) -> FilterState { self.filter_state }
    pub fn filter_value(&self) -> String { self.filter_input.value() }
    pub fn setting_filter(&self) -> bool { self.filter_state == FilterState::Filtering }
    pub fn is_filtered(&self) -> bool { self.filter_state == FilterState::FilterApplied }
    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }
    pub fn next_page(&mut self) { self.paginator.next_page(); self.cursor = self.cursor.min(self.max_cursor_index()); }
    pub fn prev_page(&mut self) { self.paginator.prev_page(); self.cursor = self.cursor.min(self.max_cursor_index()); }
    pub fn cursor_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        } else if self.paginator.page > 0 {
            self.paginator.prev_page();
            self.cursor = self.max_cursor_index();
        } else if self.infinite_scrolling && self.filtered_len() > 0 {
            self.go_to_end();
        }
    }
    pub fn cursor_down(&mut self) {
        let max_cursor_index = self.max_cursor_index();
        if self.cursor < max_cursor_index {
            self.cursor += 1;
        } else if !self.paginator.on_last_page() {
            self.paginator.next_page();
            self.cursor = 0;
        } else if self.infinite_scrolling && self.filtered_len() > 0 {
            self.go_to_start();
        }
    }
    pub fn go_to_start(&mut self) { self.paginator.page = 0; self.cursor = 0; }
    pub fn go_to_end(&mut self) { self.paginator.page = self.paginator.total_pages.saturating_sub(1); self.cursor = self.max_cursor_index(); }
    pub fn set_width(&mut self, width: usize) { self.width = width; self.help.set_width(width); }
    pub fn set_height(&mut self, height: usize) { self.height = height; self.update_pagination(); }
    pub fn set_size(&mut self, width: usize, height: usize) { self.set_width(width); self.set_height(height); }
    pub fn set_item(&mut self, index: usize, item: T) { if index < self.items.len() { self.items[index] = item; self.rebuild_filter(); } }
    pub fn insert_item(&mut self, index: usize, item: T) { let idx = index.min(self.items.len()); self.items.insert(idx, item); self.rebuild_filter(); }
    pub fn remove_item(&mut self, index: usize) { if index < self.items.len() { self.items.remove(index); self.rebuild_filter(); } }
    pub fn set_delegate(&mut self, delegate: D) { self.delegate = delegate; self.update_pagination(); }
    pub fn visible_items(&self) -> Vec<&T> {
        if self.filter_state == FilterState::Unfiltered {
            return self.items.iter().collect();
        }
        self.filtered_indices.iter().filter_map(|idx| self.items.get(*idx)).collect()
    }
    pub fn matches_for_item(&self, index: usize) -> Vec<usize> {
        let query = self.filter_input.value().to_lowercase();
        if query.is_empty() { return Vec::new(); }
        let Some(item) = self.filtered_indices.get(index).and_then(|idx| self.items.get(*idx)) else { return Vec::new(); };
        let hay = item.filter_value().to_lowercase();
        hay.find(&query).map(|start| (start..start + query.chars().count()).collect()).unwrap_or_default()
    }
    pub fn set_spinner(&mut self, spin: spinner::Spinner) { self.spinner.spinner = spin; }
    pub fn start_spinner(&mut self) { self.show_spinner = true; }
    pub fn stop_spinner(&mut self) { self.show_spinner = false; }
    pub fn toggle_spinner(&mut self) { self.show_spinner = !self.show_spinner; }
    pub fn disable_quit_keybindings(&mut self) {
        self.key_map.quit.set_enabled(false);
        self.key_map.force_quit.set_enabled(false);
    }
    pub fn new_status_message(&mut self, msg: impl Into<String>) { self.status_message = msg.into(); }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        if self.filter_state == FilterState::Filtering {
            if key::matches(event, [&self.key_map.cancel_while_filtering]) {
                self.filter_state = if self.filter_input.value().is_empty() { FilterState::Unfiltered } else { FilterState::FilterApplied };
                self.update_keybindings();
            } else if key::matches(event, [&self.key_map.accept_while_filtering]) {
                self.filter_state = if self.filter_input.value().is_empty() { FilterState::Unfiltered } else { FilterState::FilterApplied };
                self.rebuild_filter();
                self.update_keybindings();
            } else {
                self.filter_input.handle_key(event);
                self.rebuild_filter();
                self.update_keybindings();
            }
            return;
        }

        if key::matches(event, [&self.key_map.cursor_up]) {
            self.cursor_up();
        } else if key::matches(event, [&self.key_map.cursor_down]) {
            self.cursor_down();
        } else if key::matches(event, [&self.key_map.prev_page]) {
            self.prev_page();
        } else if key::matches(event, [&self.key_map.next_page]) {
            self.next_page();
        } else if key::matches(event, [&self.key_map.go_to_start]) {
            self.go_to_start();
        } else if key::matches(event, [&self.key_map.go_to_end]) {
            self.go_to_end();
        } else if key::matches(event, [&self.key_map.filter]) && self.filtering_enabled {
            self.filter_state = FilterState::Filtering;
            self.filter_input.focus();
            self.update_keybindings();
        } else if key::matches(event, [&self.key_map.clear_filter]) {
            self.filter_input.set_value("");
            self.filter_state = FilterState::Unfiltered;
            self.rebuild_filter();
            self.update_keybindings();
        } else if key::matches(event, [&self.key_map.show_full_help]) {
            self.help.show_all = !self.help.show_all;
            self.update_keybindings();
        }
        self.update_pagination_for_cursor();
    }

    pub fn view(&self) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        if self.show_title {
            let mut spans = vec![Span::styled(self.title.clone(), self.styles.title)];
            if self.show_spinner { spans.push(Span::raw(" ")); spans.extend(self.spinner.view().spans); }
            out.push(Line::from(spans));
        }
        if self.show_filter && self.filter_state == FilterState::Filtering {
            out.push(self.filter_input.view());
        }
        let body_rows = self.page_items();
        if body_rows.is_empty() {
            out.push(Line::styled(format!("No {}", self.item_name_plural), self.styles.no_items));
        } else {
            for (idx, item) in body_rows {
                let selected = idx == self.cursor;
                out.extend(self.delegate.render(item as &dyn Item, selected, self.width));
                for _ in 0..self.delegate.spacing() { out.push(Line::default()); }
            }
        }
        if self.show_status_bar {
            out.push(self.status_line());
        }
        if self.show_pagination {
            out.push(Line::styled(self.paginator.view(), self.styles.pagination_style));
        }
        if self.show_help {
            out.extend(self.help.view(&self.key_map));
        }
        out
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (i, line) in self.view().into_iter().take(area.height as usize).enumerate() {
            buf.set_line(area.x, area.y + i as u16, &line, area.width);
        }
    }

    fn rebuild_filter(&mut self) {
        let query = self.filter_input.value().to_lowercase();
        self.filtered_indices = self.items.iter().enumerate().filter_map(|(idx, item)| {
            if query.is_empty() || item.filter_value().to_lowercase().contains(&query) { Some(idx) } else { None }
        }).collect();
        if self.cursor >= self.filtered_len() { self.cursor = self.filtered_len().saturating_sub(1); }
        self.update_pagination();
    }

    fn reset_filtering(&mut self) {
        if self.filter_state == FilterState::Unfiltered { return; }
        self.filter_state = FilterState::Unfiltered;
        self.filter_input.reset();
        self.rebuild_filter();
        self.update_keybindings();
    }

    fn update_pagination(&mut self) {
        let index = self.index();
        self.paginator.per_page = self.page_size().max(1);
        self.paginator.set_total_pages(self.filtered_len().max(1));
        self.paginator.page = index / self.paginator.per_page;
        if self.paginator.page >= self.paginator.total_pages {
            self.paginator.page = self.paginator.total_pages.saturating_sub(1);
        }
        self.cursor = index % self.paginator.per_page;
        self.cursor = self.cursor.min(self.max_cursor_index());
    }

    fn update_pagination_for_cursor(&mut self) {
        self.cursor = self.cursor.min(self.max_cursor_index());
    }

    fn max_cursor_index(&self) -> usize {
        self.paginator.items_on_page(self.filtered_len()).saturating_sub(1)
    }

    fn page_size(&self) -> usize {
        let reserved = usize::from(self.show_title) + usize::from(self.show_filter && self.filter_state == FilterState::Filtering) + usize::from(self.show_status_bar) + usize::from(self.show_pagination) + if self.show_help { if self.help.show_all { 3 } else { 1 } } else { 0 };
        let row_height = self.delegate.height() + self.delegate.spacing();
        self.height.saturating_sub(reserved).checked_div(row_height.max(1)).unwrap_or(0).max(1)
    }

    fn filtered_len(&self) -> usize {
        if self.filter_state == FilterState::Unfiltered {
            self.items.len()
        } else {
            self.filtered_indices.len()
        }
    }

    fn page_items(&self) -> Vec<(usize, &T)> {
        let page_size = self.page_size();
        let start = self.paginator.page * page_size;
        let end = (start + page_size).min(self.filtered_len());
        if self.filter_state == FilterState::Unfiltered {
            return self.items[start..end].iter().enumerate().map(|(i, item)| (start + i, item)).collect();
        }
        self.filtered_indices[start..end].iter().enumerate().filter_map(|(i, idx)| self.items.get(*idx).map(|item| (start + i, item))).collect()
    }

    fn status_line(&self) -> Line<'static> {
        let total = self.items.len();
        let visible = self.filtered_indices.len();
        let filter = self.filter_input.value();
        let text = if filter.is_empty() {
            format!("{} {}", visible, if visible == 1 { &self.item_name_singular } else { &self.item_name_plural })
        } else {
            format!("{} of {} {} for '{}'", visible, total, self.item_name_plural, filter)
        };
        Line::styled(text, self.styles.status_bar)
    }

    fn update_keybindings(&mut self) {
        match self.filter_state {
            FilterState::Filtering => {
                self.key_map.cursor_up.set_enabled(false);
                self.key_map.cursor_down.set_enabled(false);
                self.key_map.next_page.set_enabled(false);
                self.key_map.prev_page.set_enabled(false);
                self.key_map.go_to_start.set_enabled(false);
                self.key_map.go_to_end.set_enabled(false);
                self.key_map.filter.set_enabled(false);
                self.key_map.clear_filter.set_enabled(false);
                self.key_map.cancel_while_filtering.set_enabled(true);
                self.key_map.accept_while_filtering.set_enabled(!self.filter_input.value().is_empty());
                self.key_map.quit.set_enabled(false);
                self.key_map.show_full_help.set_enabled(false);
                self.key_map.close_full_help.set_enabled(false);
            }
            _ => {
                let has_items = !self.items.is_empty();
                self.key_map.cursor_up.set_enabled(has_items);
                self.key_map.cursor_down.set_enabled(has_items);
                let has_pages = self.paginator.total_pages > 1;
                self.key_map.next_page.set_enabled(has_pages);
                self.key_map.prev_page.set_enabled(has_pages);
                self.key_map.go_to_start.set_enabled(has_items);
                self.key_map.go_to_end.set_enabled(has_items);
                self.key_map.filter.set_enabled(self.filtering_enabled && has_items);
                self.key_map.clear_filter.set_enabled(self.filter_state == FilterState::FilterApplied);
                self.key_map.cancel_while_filtering.set_enabled(false);
                self.key_map.accept_while_filtering.set_enabled(false);
                self.key_map.quit.set_enabled(true);
                let min_help = self.help.show_all || self.key_map.full_help().into_iter().flatten().filter(|binding| binding.enabled()).count() > 1;
                self.key_map.show_full_help.set_enabled(min_help);
                self.key_map.close_full_help.set_enabled(min_help);
            }
        }
    }
}
