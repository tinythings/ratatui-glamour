//! Dropdown selector backed by the Bubble-style list widget.
//!
//! The closed control renders a label, a highlighted value pill and a menu
//! affordance. Opening it places a list popup directly below the value,
//! aligned with the pill. The popup uses the same default item rendering as
//! the Bubble list example: a title, a description and a selected accent bar.

use crossterm::event::{KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::buffer::{Buffer, CellDiffOption};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};
use std::num::NonZeroU16;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::border::Border;
use crate::widgets::{
    help,
    key::{self, Binding},
    list::{
        DefaultDelegate, DefaultItemStyles, DefaultListItem, Model as ListModel,
        Styles as ListStyles,
    },
};

const ROW_HEIGHT: u16 = 3;
const ITEM_HEIGHT: u16 = 2;
const DEFAULT_POPUP_WIDTH: u16 = 48;

/// How a dropdown handled a key or mouse event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// The event did not change the dropdown.
    None,
    /// The popup was opened.
    Opened,
    /// The popup was closed without changing the selection.
    Closed,
    /// A new item was selected and the popup closed.
    Selected(usize),
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub cursor_up: Binding,
    pub cursor_down: Binding,
    pub prev_page: Binding,
    pub next_page: Binding,
    pub first: Binding,
    pub last: Binding,
    pub select: Binding,
    pub close: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            cursor_up: Binding::new([key::with_keys(&["up", "k"]), key::with_help("↑/k", "up")]),
            cursor_down: Binding::new([
                key::with_keys(&["down", "j"]),
                key::with_help("↓/j", "down"),
            ]),
            prev_page: Binding::new([
                key::with_keys(&["left", "pgup"]),
                key::with_help("←/pgup", "prev page"),
            ]),
            next_page: Binding::new([
                key::with_keys(&["right", "pgdown"]),
                key::with_help("→/pgdn", "next page"),
            ]),
            first: Binding::new([
                key::with_keys(&["home", "g"]),
                key::with_help("g/home", "first"),
            ]),
            last: Binding::new([
                key::with_keys(&["end", "G"]),
                key::with_help("G/end", "last"),
            ]),
            select: Binding::new([key::with_keys(&["enter"]), key::with_help("↵", "select")]),
            close: Binding::new([key::with_keys(&["esc"]), key::with_help("esc", "close")]),
        }
    }
}

impl help::KeyMap for KeyMap {
    fn short_help(&self) -> Vec<Binding> {
        vec![
            self.cursor_up.clone(),
            self.cursor_down.clone(),
            self.select.clone(),
            self.close.clone(),
        ]
    }

    fn full_help(&self) -> Vec<Vec<Binding>> {
        vec![
            vec![
                self.cursor_up.clone(),
                self.cursor_down.clone(),
                self.prev_page.clone(),
                self.next_page.clone(),
            ],
            vec![
                self.first.clone(),
                self.last.clone(),
                self.select.clone(),
                self.close.clone(),
            ],
        ]
    }
}

#[derive(Clone, Debug)]
pub struct Styles {
    /// Style for the label before the value pill.
    pub label: Style,
    /// Style for the highlighted value pill, including its padding.
    pub value: Style,
    /// Style for the menu affordance.
    pub icon: Style,
    /// Style for the popup border.
    pub border: Style,
    /// Base style for the popup background.
    pub popup: Style,
    /// Styles used by the list body and paginator.
    pub list: ListStyles,
    /// Styles used by the default list items.
    pub item: DefaultItemStyles,
    /// Styles used by the shortcut help line.
    pub help: help::Styles,
}

impl Default for Styles {
    fn default() -> Self {
        let popup = Color::Indexed(235);
        let pink = Color::Indexed(213);
        let purple = Color::Indexed(141);

        let list = ListStyles {
            pagination_style: Style::default().fg(Color::Indexed(244)).bg(popup),
            active_pagination_dot: Style::default()
                .fg(Color::Indexed(252))
                .bg(popup)
                .add_modifier(Modifier::BOLD),
            inactive_pagination_dot: Style::default().fg(Color::Indexed(240)).bg(popup),
            ..ListStyles::default()
        };

        let item = DefaultItemStyles {
            selected_accent: Style::default()
                .fg(purple)
                .bg(popup)
                .add_modifier(Modifier::BOLD),
            normal_title: Style::default().fg(Color::Indexed(252)).bg(popup),
            normal_desc: Style::default().fg(Color::Indexed(244)).bg(popup),
            selected_title: Style::default()
                .fg(pink)
                .bg(popup)
                .add_modifier(Modifier::BOLD),
            selected_desc: Style::default().fg(purple).bg(popup),
            dimmed_title: Style::default().fg(Color::Indexed(240)).bg(popup),
            dimmed_desc: Style::default().fg(Color::Indexed(238)).bg(popup),
            filter_match: Style::default()
                .fg(Color::Indexed(223))
                .bg(popup)
                .add_modifier(Modifier::UNDERLINED),
            selected_filter_match: Style::default()
                .fg(Color::Indexed(225))
                .bg(popup)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        };

        let help = help::Styles {
            ellipsis: Style::default().fg(Color::Indexed(240)).bg(popup),
            short_key: Style::default()
                .fg(Color::Indexed(252))
                .bg(popup)
                .add_modifier(Modifier::BOLD),
            short_desc: Style::default().fg(Color::Indexed(244)).bg(popup),
            short_separator: Style::default().fg(Color::Indexed(238)).bg(popup),
            full_key: Style::default()
                .fg(Color::Indexed(252))
                .bg(popup)
                .add_modifier(Modifier::BOLD),
            full_desc: Style::default().fg(Color::Indexed(244)).bg(popup),
            full_separator: Style::default().fg(Color::Indexed(238)).bg(popup),
        };

        Self {
            label: Style::default().fg(Color::Indexed(244)),
            value: Style::default()
                .fg(Color::Indexed(252))
                .bg(Color::Indexed(237)),
            icon: Style::default()
                .fg(Color::Indexed(235))
                .bg(Color::Indexed(244)),
            border: Style::default().fg(Color::Indexed(240)),
            popup: Style::default().bg(popup),
            list,
            item,
            help,
        }
    }
}

/// How the label column is aligned.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum LabelAlign {
    #[default]
    Left,
    Right,
}

/// A labelled dropdown with a relative list popup.
pub struct Model {
    pub label: String,
    /// Width reserved for the label column, or `None` to size it to the text.
    pub label_width: Option<u16>,
    /// Cells between the label column and the value pill.
    pub label_gap: u16,
    /// Alignment of the label inside its column.
    pub label_align: LabelAlign,
    /// Text appended to the label before alignment, such as `":"`.
    pub label_suffix: String,
    pub placeholder: String,
    pub icon: String,
    /// Number of list rows shown per page.
    pub page_size: usize,
    pub show_paginator: bool,
    pub show_help: bool,
    /// Fixed width of the value control, including its padding and menu icon.
    /// The label column is outside this width. `None` sizes it dynamically.
    pub width: Option<u16>,
    /// Optional popup width. `None` uses the space to the right of the value.
    pub popup_width: Option<u16>,
    /// Padding inside the popup, in cells.
    pub padding: u16,
    pub styles: Styles,
    pub key_map: KeyMap,
    pub help: help::Model,
    selected: usize,
    open: bool,
    list: ListModel<DefaultListItem, DefaultDelegate>,
    field_area: Option<Rect>,
    popup_area: Option<Rect>,
    value_x: u16,
    item_rows: Vec<(usize, Rect)>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new("", Vec::new())
    }
}

impl Model {
    pub fn new(label: impl Into<String>, items: Vec<DefaultListItem>) -> Self {
        let mut list = ListModel::new(
            items,
            DefaultDelegate::new(),
            40,
            desired_list_height(4, true) as usize,
        );
        list.show_title = false;
        list.show_filter = false;
        list.show_status_bar = false;
        list.show_pagination = false;
        list.show_help = false;
        list.filtering_enabled = false;

        let mut help = help::Model::new();
        help.styles = Styles::default().help;
        help.set_width(40);

        Self {
            label: label.into(),
            label_width: None,
            label_gap: 2,
            label_align: LabelAlign::Left,
            label_suffix: String::new(),
            placeholder: "select…".to_string(),
            icon: "☰".to_string(),
            page_size: 4,
            show_paginator: true,
            show_help: true,
            width: None,
            popup_width: None,
            padding: 1,
            styles: Styles::default(),
            key_map: KeyMap::default(),
            help,
            selected: 0,
            open: false,
            list,
            field_area: None,
            popup_area: None,
            value_x: 0,
            item_rows: Vec::new(),
        }
    }

    pub fn items(&self) -> &[DefaultListItem] {
        self.list.items()
    }

    pub fn set_items(&mut self, items: Vec<DefaultListItem>) {
        self.list.set_items(items);
        self.clamp_selection();
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_item(&self) -> Option<&DefaultListItem> {
        self.items().get(self.selected)
    }

    pub fn value(&self) -> String {
        self.selected_item()
            .map(|item| item.title.clone())
            .unwrap_or_else(|| self.placeholder.clone())
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index;
        self.clamp_selection();
        if self.open {
            self.list.select(self.selected);
        }
    }

    pub fn select_next(&mut self) {
        if !self.items().is_empty() {
            self.select((self.selected + 1) % self.items().len());
        }
    }

    pub fn select_previous(&mut self) {
        if !self.items().is_empty() {
            self.select((self.selected + self.items().len() - 1) % self.items().len());
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.configure_page_size();
        self.open = true;
        self.list.select(self.selected);
    }

    pub fn close(&mut self) {
        self.open = false;
        self.item_rows.clear();
    }

    pub fn toggle(&mut self) -> Outcome {
        if self.open {
            self.close();
            Outcome::Closed
        } else {
            self.open();
            Outcome::Opened
        }
    }

    pub fn area(&self) -> Option<Rect> {
        self.field_area
    }

    pub fn popup_area(&self) -> Option<Rect> {
        self.popup_area
    }

    pub fn handle_key(&mut self, event: &KeyEvent) -> Outcome {
        if self.open {
            if key::matches(event, [&self.key_map.select]) {
                let index = self.list.index();
                self.selected = index;
                self.clamp_selection();
                self.close();
                return Outcome::Selected(self.selected);
            }
            if key::matches(event, [&self.key_map.close]) {
                self.close();
                return Outcome::Closed;
            }
            self.list.handle_key(event);
            return Outcome::None;
        }

        if key::matches(event, [&self.key_map.select]) {
            self.open();
            return Outcome::Opened;
        }
        if key::matches(event, [&self.key_map.cursor_up]) {
            self.select_previous();
            return Outcome::Selected(self.selected);
        }
        if key::matches(event, [&self.key_map.cursor_down]) {
            self.select_next();
            return Outcome::Selected(self.selected);
        }
        if key::matches(event, [&self.key_map.first]) {
            self.select(0);
            return Outcome::Selected(self.selected);
        }
        if key::matches(event, [&self.key_map.last]) {
            self.select(self.items().len().saturating_sub(1));
            return Outcome::Selected(self.selected);
        }
        Outcome::None
    }

    pub fn handle_mouse(&mut self, event: &MouseEvent) -> Outcome {
        let point = (event.column, event.row);
        match event.kind {
            MouseEventKind::ScrollUp if self.open => {
                self.list.cursor_up();
                Outcome::None
            }
            MouseEventKind::ScrollDown if self.open => {
                self.list.cursor_down();
                Outcome::None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                if self.open {
                    if self
                        .popup_area
                        .is_some_and(|area| rect_contains(area, point))
                    {
                        if let Some((index, _)) = self
                            .item_rows
                            .iter()
                            .find(|(_, area)| rect_contains(*area, point))
                        {
                            self.selected = *index;
                            self.clamp_selection();
                            self.close();
                            return Outcome::Selected(self.selected);
                        }
                        return Outcome::None;
                    }
                    self.close();
                    return Outcome::Closed;
                }
                if self
                    .field_area
                    .is_some_and(|area| rect_contains(area, point))
                {
                    self.open();
                    return Outcome::Opened;
                }
                Outcome::None
            }
            _ => Outcome::None,
        }
    }

    /// Renders the closed control and, when open, its popup.
    ///
    /// Call this after sibling widgets so an open popup is painted on top.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        self.render_field(area, buf);
        self.render_popup(buf);
    }

    /// Renders only the label, value pill and menu affordance.
    pub fn render_field(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.field_area = Some(area);

        let label_width = self
            .label_width
            .unwrap_or_else(|| UnicodeWidthStr::width(self.label.as_str()) as u16);
        let has_label = self.label_width.is_some() || !self.label.is_empty();
        if !self.label.is_empty() {
            let column = label_width.min(area.width);
            let text = format!("{}{}", self.label, self.label_suffix);
            let text_width = UnicodeWidthStr::width(text.as_str()) as u16;
            let padding = match self.label_align {
                LabelAlign::Left => 0,
                LabelAlign::Right => column.saturating_sub(text_width),
            };
            let text = if padding > 0 {
                format!("{}{}", " ".repeat(padding as usize), text)
            } else {
                text
            };
            let label_area = Rect::new(area.x, area.y, column, 1);
            buf.set_line(
                label_area.x,
                label_area.y,
                &Line::from(Span::styled(text, self.styles.label)),
                label_area.width,
            );
        }

        let gap = if has_label { self.label_gap } else { 0 };
        let value_x = area.x.saturating_add(label_width).saturating_add(gap);
        self.value_x = value_x;
        let icon_width = UnicodeWidthStr::width(self.icon.as_str()) as u16;
        let available = area
            .right()
            .saturating_sub(value_x)
            .min(self.width.unwrap_or(u16::MAX));
        if available <= icon_width || available.saturating_sub(icon_width) < 3 {
            return;
        }

        let pill_space = available.saturating_sub(icon_width);
        let text_space = pill_space.saturating_sub(2) as usize;
        let value = truncate_ellipsis(&self.value(), text_space);
        let text_width = UnicodeWidthStr::width(value.as_str()) as u16;
        let pill = if self.width.is_some() {
            let padding = pill_space.saturating_sub(text_width + 2) as usize;
            format!(" {value}{} ", " ".repeat(padding))
        } else {
            format!(" {value} ")
        };
        let pill_width = UnicodeWidthStr::width(pill.as_str()) as u16;
        let mut spans = vec![Span::styled(pill, self.styles.value)];
        spans.push(Span::styled(self.icon.clone(), self.styles.icon));
        buf.set_line(value_x, area.y, &Line::from(spans), available);

        // Terminals commonly draw the menu glyph narrower than its Unicode
        // width. Force a single-cell diff width so the trailing cell is
        // refreshed instead of leaving stale output behind.
        if let Some(cell) = buf.cell_mut((value_x + pill_width, area.y)) {
            cell.set_diff_option(CellDiffOption::ForcedWidth(NonZeroU16::MIN));
        }

        // Wide glyphs reset their trailing cells while being written. Repaint
        // the icon slot so the selected icon background covers the glyph.
        for offset in 1..icon_width {
            if let Some(cell) = buf.cell_mut((value_x + pill_width + offset, area.y)) {
                cell.set_symbol(" ");
                cell.set_style(self.styles.icon);
            }
        }
    }

    /// Renders the popup for the most recent field layout.
    pub fn render_popup(&mut self, buf: &mut Buffer) {
        if !self.open {
            self.popup_area = None;
            return;
        }
        let Some(field_area) = self.field_area else {
            return;
        };
        let buffer_area = buf.area;
        let padding = self.padding;
        let chrome = padding.saturating_mul(2).saturating_add(2);
        let desired_width = self.popup_width.unwrap_or_else(|| {
            field_area
                .width
                .saturating_sub(self.value_x.saturating_sub(field_area.x))
                .clamp(28, DEFAULT_POPUP_WIDTH)
        });
        let popup_width = desired_width.min(buffer_area.width);
        if popup_width <= chrome.saturating_add(8) {
            return;
        }
        let list_width = popup_width.saturating_sub(chrome);
        self.help.set_width(list_width as usize);

        let help_lines = if self.show_help {
            self.help.view(&self.key_map)
        } else {
            Vec::new()
        };
        let help_height = help_lines.len() as u16;
        let paginator_height = u16::from(self.show_paginator);
        let configured_body_height = self.page_size.max(1) as u16 * ROW_HEIGHT;
        let reserved_height = paginator_height.saturating_add(help_height);
        let max_body_height = buffer_area
            .height
            .saturating_sub(chrome.saturating_add(reserved_height));
        let body_height = configured_body_height.min(max_body_height);
        let popup_height = body_height
            .saturating_add(reserved_height)
            .saturating_add(chrome);
        if popup_height > buffer_area.height {
            return;
        }

        let mut x = self.value_x;
        if x.saturating_add(popup_width) > buffer_area.right() {
            x = buffer_area.right().saturating_sub(popup_width);
        }
        x = x.max(buffer_area.left());
        let mut y = field_area.bottom();
        if y.saturating_add(popup_height) > buffer_area.bottom() {
            y = field_area.y.saturating_sub(popup_height);
        }
        y = y.max(buffer_area.top());
        if y.saturating_add(popup_height) > buffer_area.bottom() {
            y = buffer_area.bottom().saturating_sub(popup_height);
        }

        let popup = Rect::new(x, y, popup_width, popup_height);
        self.popup_area = Some(popup);
        fill_rect(buf, popup, self.styles.popup);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_set(Border::rounded().into_border_set())
            .border_style(self.styles.border)
            .style(self.styles.popup);
        block.render(popup, buf);

        let inner = Rect::new(
            popup.x.saturating_add(1 + padding),
            popup.y.saturating_add(1 + padding),
            popup_width.saturating_sub(chrome),
            popup_height.saturating_sub(chrome),
        );

        self.sync_list();
        let list_height = body_height.saturating_add(paginator_height);
        self.list
            .set_size(list_width as usize, list_height as usize);
        let list_area = Rect::new(inner.x, inner.y, list_width, body_height);
        self.list.render(list_area, buf);

        if self.show_paginator && self.list.paginator.total_pages > 1 {
            let area = Rect::new(inner.x, inner.y + body_height, list_width, 1);
            self.render_paginator(area, buf);
        }

        if self.show_help {
            let area = Rect::new(
                inner.x,
                inner.y + body_height + paginator_height,
                list_width,
                help_height,
            );
            self.help.render(area, buf, &self.key_map);
        }

        self.item_rows.clear();
        let start = self
            .list
            .paginator
            .page
            .saturating_mul(self.list.paginator.per_page);
        let visible = self
            .items()
            .len()
            .saturating_sub(start)
            .min(self.list.paginator.per_page);
        for offset in 0..visible {
            self.item_rows.push((
                start + offset,
                Rect::new(
                    inner.x,
                    inner.y + offset as u16 * ROW_HEIGHT,
                    list_width,
                    ITEM_HEIGHT,
                ),
            ));
        }
    }

    fn render_paginator(&self, area: Rect, buf: &mut Buffer) {
        let mut spans = vec![Span::styled("  ", self.styles.list.pagination_style)];
        for page in 0..self.list.paginator.total_pages {
            if page == self.list.paginator.page {
                spans.push(Span::styled(
                    self.list.paginator.active_dot.clone(),
                    self.styles.list.active_pagination_dot,
                ));
            } else {
                spans.push(Span::styled(
                    self.list.paginator.inactive_dot.clone(),
                    self.styles.list.inactive_pagination_dot,
                ));
            }
        }
        buf.set_line(area.x, area.y, &Line::from(spans), area.width);
    }

    fn configure_page_size(&mut self) {
        let height = desired_list_height(self.page_size, self.show_paginator);
        self.list.set_size(self.list.width(), height as usize);
    }

    fn clamp_selection(&mut self) {
        let len = self.items().len();
        self.selected = if len == 0 {
            0
        } else {
            self.selected.min(len - 1)
        };
    }

    fn sync_list(&mut self) {
        if let Some(bg) = self.styles.popup.bg {
            apply_background(&mut self.styles.list.title_bar, bg);
            apply_background(&mut self.styles.list.title, bg);
            apply_background(&mut self.styles.list.spinner, bg);
            apply_background(&mut self.styles.list.default_filter_character_match, bg);
            apply_background(&mut self.styles.list.status_bar, bg);
            apply_background(&mut self.styles.list.status_empty, bg);
            apply_background(&mut self.styles.list.status_bar_active_filter, bg);
            apply_background(&mut self.styles.list.status_bar_filter_count, bg);
            apply_background(&mut self.styles.list.status_bar_filter_query, bg);
            apply_background(&mut self.styles.list.no_items, bg);
            apply_background(&mut self.styles.list.pagination_style, bg);
            apply_background(&mut self.styles.list.help_style, bg);
            apply_background(&mut self.styles.list.active_pagination_dot, bg);
            apply_background(&mut self.styles.list.inactive_pagination_dot, bg);
            apply_background(&mut self.styles.list.arabic_pagination, bg);
            apply_background(&mut self.styles.list.divider_dot, bg);

            apply_background(&mut self.styles.item.selected_accent, bg);
            apply_background(&mut self.styles.item.normal_title, bg);
            apply_background(&mut self.styles.item.normal_desc, bg);
            apply_background(&mut self.styles.item.selected_title, bg);
            apply_background(&mut self.styles.item.selected_desc, bg);
            apply_background(&mut self.styles.item.dimmed_title, bg);
            apply_background(&mut self.styles.item.dimmed_desc, bg);
            apply_background(&mut self.styles.item.filter_match, bg);
            apply_background(&mut self.styles.item.selected_filter_match, bg);

            apply_background(&mut self.styles.help.ellipsis, bg);
            apply_background(&mut self.styles.help.short_key, bg);
            apply_background(&mut self.styles.help.short_desc, bg);
            apply_background(&mut self.styles.help.short_separator, bg);
            apply_background(&mut self.styles.help.full_key, bg);
            apply_background(&mut self.styles.help.full_desc, bg);
            apply_background(&mut self.styles.help.full_separator, bg);
        }

        self.list.styles = self.styles.list.clone();
        self.help.styles = self.styles.help.clone();
        self.list.show_title = false;
        self.list.show_filter = false;
        self.list.show_status_bar = false;
        self.list.show_pagination = false;
        self.list.show_help = false;
        self.list.filtering_enabled = false;

        let mut delegate = DefaultDelegate::new();
        delegate.styles = self.styles.item.clone();
        self.list.set_delegate(delegate);
    }
}

fn desired_list_height(page_size: usize, show_paginator: bool) -> u16 {
    page_size.max(1) as u16 * ROW_HEIGHT + u16::from(show_paginator)
}

fn apply_background(style: &mut Style, bg: Color) {
    style.bg = Some(bg);
}

fn fill_rect(buf: &mut Buffer, area: Rect, style: Style) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if let Some(cell) = buf.cell_mut((x, y)) {
                cell.set_symbol(" ");
                cell.set_style(style);
            }
        }
    }
}

fn rect_contains(area: Rect, point: (u16, u16)) -> bool {
    point.0 >= area.x && point.0 < area.right() && point.1 >= area.y && point.1 < area.bottom()
}

fn truncate_ellipsis(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    let take = width.saturating_sub(1);
    let mut used = 0usize;
    let mut out = String::new();
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > take {
            break;
        }
        used += ch_width;
        out.push(ch);
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn item(title: &str, description: &str) -> DefaultListItem {
        DefaultListItem::new(title, description)
    }

    fn items(count: usize) -> Vec<DefaultListItem> {
        (1..=count)
            .map(|index| item(&format!("item-{index}"), &format!("description-{index}")))
            .collect()
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn buffer_text(buffer: &Buffer) -> String {
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn render_open(model: &mut Model, width: u16, height: u16) -> Buffer {
        let mut buffer = Buffer::empty(Rect::new(0, 0, width, height));
        model.render_field(Rect::new(0, 0, width, 1), &mut buffer);
        model.open();
        model.render_popup(&mut buffer);
        buffer
    }

    #[test]
    fn defaults_match_the_compact_dropdown_configuration() {
        let model = Model::new("label", items(22));
        assert_eq!(model.page_size, 4);
        assert!(model.show_paginator);
        assert!(model.show_help);
        assert!(model.width.is_none());
        assert!(!model.is_open());
    }

    #[test]
    fn fixed_width_crops_the_value_and_keeps_the_pill_padding() {
        let mut model = Model::new("", vec![item("foobar hello world", "")]);
        model.width = Some(15);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
        model.render_field(Rect::new(0, 0, 40, 1), &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.starts_with(" foobar hel… ☰"), "{text:?}");
        assert_eq!(UnicodeWidthStr::width(text.trim_end()), 15, "{text:?}");
    }

    #[test]
    fn fixed_width_fills_the_pill_before_the_icon() {
        let mut model = Model::new("", vec![item("Cats", "")]);
        model.width = Some(15);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
        model.render_field(Rect::new(0, 0, 40, 1), &mut buffer);
        let text = buffer_text(&buffer);
        let icon = text.find('☰').expect("icon");
        assert_eq!(UnicodeWidthStr::width(&text[..icon]), 13, "{text:?}");
        assert!(text.starts_with(" Cats "), "{text:?}");
        assert_eq!(UnicodeWidthStr::width(text.trim_end()), 15, "{text:?}");
    }

    #[test]
    fn closed_field_renders_label_pill_and_menu_icon() {
        let mut model = Model::new("label", vec![item("some value", "description")]);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 1));
        model.render_field(Rect::new(0, 0, 40, 1), &mut buffer);
        let text = buffer_text(&buffer);
        assert!(text.contains("label   some value ☰"), "{text:?}");
        let icon = text.find('☰').expect("icon");
        let icon_column = UnicodeWidthStr::width(&text[..icon]) as u16;
        assert_eq!(
            buffer[(icon_column + 1, 0)].style().bg,
            Some(Color::Indexed(244))
        );
    }

    #[test]
    fn label_width_aligns_the_value_column() {
        let mut first = Model::new("a", vec![item("one", "")]);
        first.label_width = Some(12);
        let mut second = Model::new("long label", vec![item("two", "")]);
        second.label_width = Some(12);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 40, 2));
        first.render_field(Rect::new(0, 0, 40, 1), &mut buffer);
        second.render_field(Rect::new(0, 1, 40, 1), &mut buffer);
        let text = buffer_text(&buffer);
        let lines = text.lines().collect::<Vec<_>>();
        let first_value = lines[0].find(" one ").expect("first value");
        let second_value = lines[1].find(" two ").expect("second value");
        assert_eq!(first_value, second_value);
    }

    #[test]
    fn right_aligned_labels_share_a_column_and_suffix() {
        let mut first = Model::new("foo", vec![item("value", "")]);
        first.label_width = Some(12);
        first.label_gap = 1;
        first.label_align = LabelAlign::Right;
        first.label_suffix = ":".to_string();
        first.width = Some(20);
        let mut second = Model::new("here is", vec![item("other value", "")]);
        second.label_width = Some(12);
        second.label_gap = 1;
        second.label_align = LabelAlign::Right;
        second.label_suffix = ":".to_string();
        second.width = Some(20);
        let mut buffer = Buffer::empty(Rect::new(0, 0, 50, 2));
        first.render_field(Rect::new(0, 0, 50, 1), &mut buffer);
        second.render_field(Rect::new(0, 1, 50, 1), &mut buffer);
        let text = buffer_text(&buffer);
        let lines = text.lines().collect::<Vec<_>>();
        let first_value = lines[0].find("value").expect("first value");
        let second_value = lines[1].find("other value").expect("second value");
        assert_eq!(
            UnicodeWidthStr::width(&lines[0][..first_value]),
            UnicodeWidthStr::width(&lines[1][..second_value])
        );
        let first_colon = lines[0].find(':').expect("first colon");
        let second_colon = lines[1].find(':').expect("second colon");
        assert_eq!(
            UnicodeWidthStr::width(&lines[0][..first_colon]),
            UnicodeWidthStr::width(&lines[1][..second_colon])
        );
    }

    #[test]
    fn enter_opens_and_enter_selects() {
        let mut model = Model::new("label", items(6));
        assert_eq!(model.handle_key(&key(KeyCode::Enter)), Outcome::Opened);
        assert!(model.is_open());
        assert_eq!(model.handle_key(&key(KeyCode::Down)), Outcome::None);
        assert_eq!(model.handle_key(&key(KeyCode::Enter)), Outcome::Selected(1));
        assert!(!model.is_open());
        assert_eq!(model.value(), "item-2");
    }

    #[test]
    fn escape_closes_without_changing_the_selection() {
        let mut model = Model::new("label", items(3));
        model.select(1);
        model.open();
        assert_eq!(model.handle_key(&key(KeyCode::Esc)), Outcome::Closed);
        assert!(!model.is_open());
        assert_eq!(model.selected(), 1);
    }

    #[test]
    fn popup_shows_items_and_hides_the_list_chrome() {
        let mut model = Model::new("label", items(6));
        model.select(1);
        let buffer = render_open(&mut model, 60, 30);
        let text = buffer_text(&buffer);
        assert!(text.contains("item-1"), "{text}");
        assert!(text.contains("description-1"), "{text}");
        assert!(text.contains("↑/k up"), "{text}");
        assert!(text.contains("↵ select"), "{text}");
        assert!(!text.contains("No items"), "{text}");
        assert!(!text.contains("page 1/"), "{text}");
    }

    #[test]
    fn paginator_and_help_can_be_switched_off() {
        let mut model = Model::new("label", items(12));
        model.show_paginator = false;
        model.show_help = false;
        let buffer = render_open(&mut model, 60, 30);
        let text = buffer_text(&buffer);
        assert!(!text.contains("↑/k up"), "{text}");
        assert!(!text.contains("•"), "{text}");
    }

    #[test]
    fn page_size_limits_the_visible_items() {
        let mut model = Model::new("label", items(12));
        model.page_size = 2;
        let buffer = render_open(&mut model, 60, 30);
        let text = buffer_text(&buffer);
        assert!(text.contains("item-1"), "{text}");
        assert!(text.contains("item-2"), "{text}");
        assert!(!text.contains("item-3"), "{text}");
    }

    #[test]
    fn mouse_opens_the_popup_and_selects_a_row() {
        let mut model = Model::new("label", items(4));
        let mut buffer = Buffer::empty(Rect::new(0, 0, 60, 30));
        model.render_field(Rect::new(5, 2, 50, 1), &mut buffer);
        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 2,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(model.handle_mouse(&click), Outcome::Opened);
        model.render_popup(&mut buffer);

        let click = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 15,
            row: 11,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(model.handle_mouse(&click), Outcome::Selected(2));
        assert_eq!(model.value(), "item-3");
    }
}
