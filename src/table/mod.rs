use std::sync::Arc;

use crossterm::event::KeyEvent;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::border::Border;
use crate::widgets::{
    key::{self, Binding},
    viewport,
};

pub const HEADER_ROW: isize = -1;

pub type StyleFn = Arc<dyn Fn(isize, usize) -> Style + Send + Sync + 'static>;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StringData {
    rows: Vec<Vec<String>>,
    columns: usize,
}

impl StringData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_rows<I, R, S>(rows: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut data = Self::new();
        for row in rows {
            data.push(row);
        }
        data
    }

    pub fn push<R, S>(&mut self, row: R)
    where
        R: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let row: Vec<String> = row.into_iter().map(Into::into).collect();
        self.columns = self.columns.max(row.len());
        self.rows.push(row);
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    pub fn columns(&self) -> usize {
        self.columns
    }

    pub fn at(&self, row: usize, col: usize) -> &str {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .map(String::as_str)
            .unwrap_or("")
    }
}

#[derive(Clone)]
pub struct Table {
    headers: Vec<String>,
    data: StringData,
    border: Border,
    border_style: Style,
    base_style: Style,
    style_fn: StyleFn,
    width: Option<u16>,
    height: Option<u16>,
    y_offset: usize,
    wrap: bool,
    border_top: bool,
    border_bottom: bool,
    border_left: bool,
    border_right: bool,
    border_header: bool,
    border_column: bool,
    border_row: bool,
}

impl Default for Table {
    fn default() -> Self {
        Self {
            headers: Vec::new(),
            data: StringData::new(),
            border: Border::normal(),
            border_style: Style::default(),
            base_style: Style::default(),
            style_fn: Arc::new(|_, _| Style::default()),
            width: None,
            height: None,
            y_offset: 0,
            wrap: true,
            border_top: true,
            border_bottom: true,
            border_left: true,
            border_right: true,
            border_header: true,
            border_column: true,
            border_row: false,
        }
    }
}

impl Table {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn headers<I, S>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.headers = headers.into_iter().map(Into::into).collect();
        self
    }

    pub fn row<I, S>(mut self, row: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.data.push(row);
        self
    }

    pub fn rows<I, R, S>(mut self, rows: I) -> Self
    where
        I: IntoIterator<Item = R>,
        R: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for row in rows {
            self.data.push(row);
        }
        self
    }

    pub fn data(mut self, data: StringData) -> Self {
        self.data = data;
        self
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = style;
        self
    }

    pub fn base_style(mut self, style: Style) -> Self {
        self.base_style = style;
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style_fn = Arc::new(move |_, _| style);
        self
    }

    pub fn style_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(isize, usize) -> Style + Send + Sync + 'static,
    {
        self.style_fn = Arc::new(f);
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn height(mut self, height: u16) -> Self {
        self.height = Some(height);
        self
    }

    pub fn y_offset(mut self, offset: usize) -> Self {
        self.y_offset = offset;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn border_top(mut self, enabled: bool) -> Self {
        self.border_top = enabled;
        self
    }
    pub fn border_bottom(mut self, enabled: bool) -> Self {
        self.border_bottom = enabled;
        self
    }
    pub fn border_left(mut self, enabled: bool) -> Self {
        self.border_left = enabled;
        self
    }
    pub fn border_right(mut self, enabled: bool) -> Self {
        self.border_right = enabled;
        self
    }
    pub fn border_header(mut self, enabled: bool) -> Self {
        self.border_header = enabled;
        self
    }
    pub fn border_column(mut self, enabled: bool) -> Self {
        self.border_column = enabled;
        self
    }
    pub fn border_row(mut self, enabled: bool) -> Self {
        self.border_row = enabled;
        self
    }

    pub fn plan(&self, area: Rect) -> TablePlan {
        let width = self.width.unwrap_or(area.width).max(1) as usize;
        let height = self.height.unwrap_or(area.height).max(1) as usize;
        let column_count = self.headers.len().max(self.data.columns()).max(1);
        let rows = self.data.rows();
        let mut max_widths = vec![0usize; column_count];
        let mut medians = vec![0usize; column_count];

        for col in 0..column_count {
            let mut widths = Vec::new();
            if let Some(header) = self.headers.get(col) {
                let w = display_width(header);
                max_widths[col] = max_widths[col].max(w);
                widths.push(w);
            }
            for row in 0..rows {
                let value = self.data.at(row, col);
                let w = max_line_width(value);
                max_widths[col] = max_widths[col].max(w);
                widths.push(w);
            }
            medians[col] = median(&mut widths);
        }

        let available_width = width.saturating_sub(self.horizontal_border_cells(column_count));
        let mut col_widths = max_widths.clone();
        let total = sum(&col_widths);
        if total < available_width {
            expand_widths(&mut col_widths, available_width);
        } else if total > available_width {
            shrink_widths(&mut col_widths, &medians, available_width);
        }

        let has_headers = !self.headers.is_empty();
        let mut header_height = 0usize;
        if has_headers {
            header_height = 1;
        }

        let row_heights: Vec<usize> = (0..rows)
            .map(|row| {
                let mut rh = 1;
                for (col, width) in col_widths.iter().enumerate() {
                    let cell = self.data.at(row, col);
                    let wrapped = if self.wrap {
                        wrap_lines(cell, *width)
                    } else {
                        truncate_lines(cell, *width, 1)
                    };
                    rh = rh.max(wrapped.len().max(1));
                }
                rh
            })
            .collect();

        let available_height = height
            .saturating_sub(bool_to_usize(self.border_top))
            .saturating_sub(bool_to_usize(self.border_bottom))
            .saturating_sub(header_height)
            .saturating_sub(bool_to_usize(has_headers && self.border_header));

        let first_visible = self.y_offset.min(rows);
        let mut used = 0usize;
        let mut visible_rows = Vec::new();
        for (row, row_height) in row_heights
            .iter()
            .enumerate()
            .take(rows)
            .skip(first_visible)
        {
            let needed = *row_height + bool_to_usize(self.border_row && !visible_rows.is_empty());
            if used + needed > available_height {
                break;
            }
            visible_rows.push(row);
            used += needed;
        }
        if visible_rows.is_empty() && first_visible < rows && available_height > 0 {
            visible_rows.push(first_visible);
        }
        let overflow = visible_rows
            .last()
            .map(|row| *row + 1 < rows)
            .unwrap_or(false);

        TablePlan {
            col_widths,
            row_heights,
            header_height,
            first_visible,
            visible_rows,
            overflow,
            area: Rect::new(area.x, area.y, width as u16, height as u16),
        }
    }

    fn horizontal_border_cells(&self, columns: usize) -> usize {
        bool_to_usize(self.border_left)
            + bool_to_usize(self.border_right)
            + columns.saturating_sub(1) * bool_to_usize(self.border_column)
    }

    fn cell_style(&self, row: isize, col: usize) -> Style {
        (self.style_fn)(row, col).patch(self.base_style)
    }
}

#[derive(Clone, Debug)]
pub struct TablePlan {
    pub col_widths: Vec<usize>,
    pub row_heights: Vec<usize>,
    pub header_height: usize,
    pub first_visible: usize,
    pub visible_rows: Vec<usize>,
    pub overflow: bool,
    pub area: Rect,
}

impl Widget for &Table {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let plan = self.plan(area);
        render_table(self, &plan, buf);
    }
}

fn render_table(table: &Table, plan: &TablePlan, buf: &mut Buffer) {
    let mut y = plan.area.y;
    let x = plan.area.x;
    let columns = plan.col_widths.len();

    if table.border_top {
        draw_horizontal_border(
            buf,
            x,
            y,
            &plan.col_widths,
            table,
            table.border.top_left,
            table.border.middle_top,
            table.border.top_right,
            table.border.top,
        );
        y += 1;
    }

    if !table.headers.is_empty() {
        let lines: Vec<Vec<String>> = table
            .headers
            .iter()
            .enumerate()
            .map(|(col, header)| truncate_lines(header, plan.col_widths[col], 1))
            .collect();
        draw_row(
            buf,
            x,
            y,
            &plan.col_widths,
            1,
            &lines,
            |col| table.cell_style(HEADER_ROW, col),
            table,
        );
        y += 1;

        if table.border_header {
            draw_horizontal_border(
                buf,
                x,
                y,
                &plan.col_widths,
                table,
                table.border.middle_left,
                table.border.middle,
                table.border.middle_right,
                table.border.top,
            );
            y += 1;
        }
    }

    for (vis_idx, row) in plan.visible_rows.iter().enumerate() {
        let cell_lines: Vec<Vec<String>> = (0..columns)
            .map(|col| {
                let cell = table.data.at(*row, col);
                if table.wrap {
                    wrap_lines(cell, plan.col_widths[col])
                } else {
                    truncate_lines(cell, plan.col_widths[col], plan.row_heights[*row])
                }
            })
            .collect();
        draw_row(
            buf,
            x,
            y,
            &plan.col_widths,
            plan.row_heights[*row],
            &cell_lines,
            |col| table.cell_style(*row as isize, col),
            table,
        );
        y += plan.row_heights[*row] as u16;

        if table.border_row && vis_idx + 1 < plan.visible_rows.len() {
            draw_horizontal_border(
                buf,
                x,
                y,
                &plan.col_widths,
                table,
                table.border.middle_left,
                table.border.middle,
                table.border.middle_right,
                table.border.bottom,
            );
            y += 1;
        }
    }

    if plan.overflow
        && y < plan
            .area
            .bottom()
            .saturating_sub(bool_to_u16(table.border_bottom))
    {
        let cell_lines: Vec<Vec<String>> = (0..columns).map(|_| vec!["…".into()]).collect();
        draw_row(
            buf,
            x,
            y,
            &plan.col_widths,
            1,
            &cell_lines,
            |_| table.base_style,
            table,
        );
        y += 1;
    }

    if table.border_bottom && y < plan.area.bottom() {
        draw_horizontal_border(
            buf,
            x,
            y,
            &plan.col_widths,
            table,
            table.border.bottom_left,
            table.border.middle_bottom,
            table.border.bottom_right,
            table.border.bottom,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_horizontal_border(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    widths: &[usize],
    table: &Table,
    left: &str,
    mid: &str,
    right: &str,
    fill: &str,
) {
    let mut cx = x;
    if table.border_left {
        buf.set_string(cx, y, left, table.border_style);
        cx += 1;
    }
    for (idx, width) in widths.iter().enumerate() {
        for _ in 0..*width {
            buf.set_string(cx, y, fill, table.border_style);
            cx += 1;
        }
        if idx + 1 < widths.len() && table.border_column {
            buf.set_string(cx, y, mid, table.border_style);
            cx += 1;
        }
    }
    if table.border_right {
        buf.set_string(cx, y, right, table.border_style);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_row<F>(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    widths: &[usize],
    height: usize,
    cell_lines: &[Vec<String>],
    style_for: F,
    table: &Table,
) where
    F: Fn(usize) -> Style,
{
    for line_idx in 0..height {
        let mut cx = x;
        if table.border_left {
            buf.set_string(
                cx,
                y + line_idx as u16,
                table.border.left,
                table.border_style,
            );
            cx += 1;
        }
        for (col, width) in widths.iter().enumerate() {
            let style = style_for(col);
            let content = cell_lines
                .get(col)
                .and_then(|lines| lines.get(line_idx))
                .cloned()
                .unwrap_or_default();
            let line = fit_to_width(&content, *width);
            let spans = vec![Span::styled(line, style)];
            buf.set_line(cx, y + line_idx as u16, &Line::from(spans), *width as u16);
            cx += *width as u16;
            if col + 1 < widths.len() && table.border_column {
                buf.set_string(
                    cx,
                    y + line_idx as u16,
                    table.border.left,
                    table.border_style,
                );
                cx += 1;
            }
        }
        if table.border_right {
            buf.set_string(
                cx,
                y + line_idx as u16,
                table.border.right,
                table.border_style,
            );
        }
    }
}

fn expand_widths(widths: &mut [usize], target: usize) {
    let mut total = sum(widths);
    while total < target {
        let mut min_idx = 0usize;
        for (idx, width) in widths.iter().enumerate().skip(1) {
            if *width < widths[min_idx] {
                min_idx = idx;
            }
        }
        widths[min_idx] += 1;
        total += 1;
    }
}

fn shrink_widths(widths: &mut [usize], medians: &[usize], target: usize) {
    while sum(widths) > target {
        let mut best_idx = None;
        let mut best_diff = isize::MIN;
        for (idx, width) in widths.iter().enumerate() {
            if *width <= 1 {
                continue;
            }
            let diff = *width as isize - medians[idx].max(1) as isize;
            if diff > best_diff {
                best_diff = diff;
                best_idx = Some(idx);
            }
        }
        let Some(idx) = best_idx else { break };
        widths[idx] -= 1;
    }
}

fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    for raw_line in text.replace("\r\n", "\n").split('\n') {
        let mut current = String::new();
        let mut used = 0usize;
        for ch in raw_line.chars() {
            let cw = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
            if used + cw > width && !current.is_empty() {
                out.push(fit_to_width(&current, width));
                current.clear();
                used = 0;
            }
            current.push(ch);
            used += cw;
        }
        out.push(fit_to_width(&current, width));
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

fn truncate_lines(text: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut out = Vec::new();
    for raw_line in text.replace("\r\n", "\n").split('\n').take(max_lines) {
        out.push(truncate_to_width(raw_line, width));
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if used + cw > width {
            break;
        }
        out.push(ch);
        used += cw;
    }
    fit_to_width(&out, width)
}

fn fit_to_width(text: &str, width: usize) -> String {
    let mut out = truncate_to_width_raw(text, width);
    let used = display_width(&out);
    if used < width {
        out.push_str(&" ".repeat(width - used));
    }
    out
}

fn truncate_to_width_raw(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if used + cw > width {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out
}

fn display_width(text: &str) -> usize {
    UnicodeWidthStr::width(text)
}

fn max_line_width(text: &str) -> usize {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(display_width)
        .max()
        .unwrap_or(0)
}

fn sum(values: &[usize]) -> usize {
    values.iter().sum()
}

fn median(values: &mut [usize]) -> usize {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    if values.len().is_multiple_of(2) {
        let mid = values.len() / 2;
        (values[mid - 1] + values[mid]) / 2
    } else {
        values[values.len() / 2]
    }
}

fn bool_to_usize(value: bool) -> usize {
    usize::from(value)
}

fn bool_to_u16(value: bool) -> u16 {
    u16::from(value)
}

pub type Row = Vec<String>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column {
    pub title: String,
    pub width: usize,
}

impl Column {
    pub fn new(title: impl Into<String>, width: usize) -> Self {
        Self {
            title: title.into(),
            width,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Styles {
    pub header: Style,
    pub cell: Style,
    pub selected: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            header: Style::default().add_modifier(ratatui::style::Modifier::BOLD),
            cell: Style::default(),
            selected: Style::default()
                .fg(ratatui::style::Color::Indexed(231))
                .bg(ratatui::style::Color::Indexed(93))
                .add_modifier(ratatui::style::Modifier::BOLD),
        }
    }
}

#[derive(Clone, Debug)]
pub struct KeyMap {
    pub line_up: Binding,
    pub line_down: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub half_page_up: Binding,
    pub half_page_down: Binding,
    pub go_to_top: Binding,
    pub go_to_bottom: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            line_up: Binding::new([key::with_keys(&["up", "k"]), key::with_help("↑/k", "up")]),
            line_down: Binding::new([
                key::with_keys(&["down", "j"]),
                key::with_help("↓/j", "down"),
            ]),
            page_up: Binding::new([
                key::with_keys(&["b", "pgup"]),
                key::with_help("b/pgup", "page up"),
            ]),
            page_down: Binding::new([
                key::with_keys(&["f", "pgdown", "space"]),
                key::with_help("f/pgdn", "page down"),
            ]),
            half_page_up: Binding::new([
                key::with_keys(&["u", "ctrl+u"]),
                key::with_help("u", "1/2 page up"),
            ]),
            half_page_down: Binding::new([
                key::with_keys(&["d", "ctrl+d"]),
                key::with_help("d", "1/2 page down"),
            ]),
            go_to_top: Binding::new([
                key::with_keys(&["home", "g"]),
                key::with_help("g/home", "top"),
            ]),
            go_to_bottom: Binding::new([
                key::with_keys(&["end", "G"]),
                key::with_help("G/end", "bottom"),
            ]),
        }
    }
}

pub struct Model {
    columns: Vec<Column>,
    rows: Vec<Row>,
    cursor: usize,
    focus: bool,
    width: usize,
    height: usize,
    pub styles: Styles,
    pub key_map: KeyMap,
    pub viewport: viewport::Model,
    pub header_padding: usize,
    pub cell_padding: usize,
}

impl Model {
    pub fn new(columns: Vec<Column>, rows: Vec<Row>) -> Self {
        let mut viewport = viewport::Model::new([]);
        viewport.set_height(0);
        viewport.set_width(0);
        Self {
            columns,
            rows,
            cursor: 0,
            focus: false,
            width: 0,
            height: 0,
            styles: Styles::default(),
            key_map: KeyMap::default(),
            viewport,
            header_padding: 1,
            cell_padding: 1,
        }
    }

    pub fn columns(&self) -> &[Column] {
        &self.columns
    }
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn focused(&self) -> bool {
        self.focus
    }
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn set_columns(&mut self, columns: Vec<Column>) {
        self.columns = columns;
        self.sync_viewport();
    }

    pub fn set_rows(&mut self, rows: Vec<Row>) {
        self.rows = rows;
        self.cursor = self.cursor.min(self.rows.len().saturating_sub(1));
        self.sync_viewport();
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.rows.len().saturating_sub(1));
        self.sync_viewport();
        self.ensure_cursor_visible();
    }

    pub fn focus(&mut self) {
        self.focus = true;
        self.sync_viewport();
    }

    pub fn blur(&mut self) {
        self.focus = false;
        self.sync_viewport();
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
        self.viewport.set_width(width);
        self.sync_viewport();
    }

    pub fn set_height(&mut self, height: usize) {
        self.height = height;
        self.viewport.set_height(self.body_height());
        self.sync_viewport();
        self.ensure_cursor_visible();
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.viewport.set_width(width);
        self.viewport.set_height(self.body_height());
        self.sync_viewport();
        self.ensure_cursor_visible();
    }

    pub fn selected_row(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    pub fn handle_key(&mut self, event: &KeyEvent) {
        if !self.focus || self.rows.is_empty() {
            return;
        }

        if key::matches(event, [&self.key_map.line_up]) {
            self.move_up(1);
        } else if key::matches(event, [&self.key_map.line_down]) {
            self.move_down(1);
        } else if key::matches(event, [&self.key_map.page_up]) {
            self.move_up(self.body_height().max(1));
        } else if key::matches(event, [&self.key_map.page_down]) {
            self.move_down(self.body_height().max(1));
        } else if key::matches(event, [&self.key_map.half_page_up]) {
            self.move_up((self.body_height().max(1) / 2).max(1));
        } else if key::matches(event, [&self.key_map.half_page_down]) {
            self.move_down((self.body_height().max(1) / 2).max(1));
        } else if key::matches(event, [&self.key_map.go_to_top]) {
            self.set_cursor(0);
        } else if key::matches(event, [&self.key_map.go_to_bottom]) {
            self.set_cursor(self.rows.len().saturating_sub(1));
        }
    }

    pub fn move_up(&mut self, n: usize) {
        self.cursor = self.cursor.saturating_sub(n);
        self.sync_viewport();
        self.ensure_cursor_visible();
    }

    pub fn move_down(&mut self, n: usize) {
        self.cursor = (self.cursor + n).min(self.rows.len().saturating_sub(1));
        self.sync_viewport();
        self.ensure_cursor_visible();
    }

    pub fn view(&self) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        out.push(self.header_line());
        let offset = self.viewport.y_offset();
        let height = self.body_height();
        for row in offset..(offset + height).min(self.rows.len()) {
            out.push(self.render_row_line(row));
        }
        out
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        for (idx, line) in self
            .view()
            .into_iter()
            .take(area.height as usize)
            .enumerate()
        {
            buf.set_line(area.x, area.y + idx as u16, &line, area.width);
        }
    }

    fn body_height(&self) -> usize {
        self.height.saturating_sub(1)
    }

    fn sync_viewport(&mut self) {
        self.viewport.set_height(self.body_height());
        self.viewport.set_width(self.width);
        let content = (0..self.rows.len())
            .map(|row| self.render_row_line(row).to_string())
            .collect::<Vec<_>>()
            .join("\n");
        self.viewport.set_content(&content);
    }

    fn ensure_cursor_visible(&mut self) {
        let height = self.body_height();
        if height == 0 || self.rows.is_empty() {
            self.viewport.set_y_offset(0);
            return;
        }
        self.viewport.ensure_visible(self.cursor, 0, self.width);
    }

    fn header_line(&self) -> Line<'static> {
        let mut spans = Vec::new();
        for (idx, col) in self.columns.iter().enumerate() {
            if idx > 0 {
                spans.push(Span::raw(""));
            }
            spans.push(Span::styled(
                padded_truncate(&col.title, col.width, self.header_padding),
                self.styles.header,
            ));
        }
        Line::from(spans)
    }

    fn render_row_line(&self, row_idx: usize) -> Line<'static> {
        let row = &self.rows[row_idx];
        let style = if row_idx == self.cursor {
            self.styles.selected
        } else {
            self.styles.cell
        };
        let mut spans = Vec::new();
        for (idx, col) in self.columns.iter().enumerate() {
            if idx > 0 {
                spans.push(Span::styled(String::new(), style));
            }
            let value = row.get(idx).map(String::as_str).unwrap_or("");
            spans.push(Span::styled(
                padded_truncate(value, col.width, self.cell_padding),
                style,
            ));
        }
        Line::from(spans)
    }
}

fn padded_truncate(text: &str, width: usize, padding: usize) -> String {
    let inner = width.saturating_sub(padding.saturating_mul(2));
    let rendered = truncate_with_ellipsis(text, inner);
    let mut out = String::new();
    out.push_str(&" ".repeat(padding));
    out.push_str(&fit_to_width(&rendered, inner));
    out.push_str(&" ".repeat(padding));
    out
}

fn truncate_with_ellipsis(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if display_width(text) <= width {
        return text.to_string();
    }
    if width == 1 {
        return "…".into();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
        if used + cw > width - 1 {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}
