// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Bo Maryniuk

//! A stateful, walkable tree widget with collapsible nodes and rich content.
//!
//! `TreePlus` is the interactive companion to [`crate::tree`]. It owns the
//! selection, collapsed branches, viewport offset and key handling. Nodes are
//! identified by stable string IDs so callers can replace the tree after a
//! reload without losing selection.

use std::collections::BTreeSet;

use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::widgets::key::{self, Binding};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One node's content.
#[derive(Clone, Debug)]
pub enum Content {
    /// Plain text, rendered with [`Styles::content`].
    Text(String),
    /// Pre-styled spans, allowing each part of the row to have its own style.
    Spans(Vec<Span<'static>>),
}

impl Content {
    fn spans(&self, base: Style) -> Vec<Span<'static>> {
        match self {
            Self::Text(text) => vec![Span::styled(
                text.replace('\r', "").replace('\n', " "),
                base,
            )],
            Self::Spans(spans) => spans
                .iter()
                .map(|span| {
                    let mut span = span.clone();
                    span.style = span.style.patch(base);
                    span
                })
                .collect(),
        }
    }
}

impl From<&str> for Content {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<String> for Content {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<Span<'static>>> for Content {
    fn from(value: Vec<Span<'static>>) -> Self {
        Self::Spans(value)
    }
}

impl From<Line<'static>> for Content {
    fn from(value: Line<'static>) -> Self {
        Self::Spans(value.spans)
    }
}

/// A node in a [`TreePlus`].
#[derive(Clone, Debug)]
pub struct Node {
    /// Stable identity used for selection and collapse state. Must be unique.
    pub id: String,
    /// Row content.
    pub content: Content,
    /// Child nodes.
    pub children: Vec<Node>,
    /// Skippable nodes are rendered but never selected by navigation.
    pub skippable: bool,
}

impl Node {
    pub fn new(id: impl Into<String>, content: impl Into<Content>) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            children: Vec::new(),
            skippable: false,
        }
    }

    pub fn child(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = Node>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn skippable(mut self, skippable: bool) -> Self {
        self.skippable = skippable;
        self
    }
}

/// Branch and collapse glyphs. Every value is a string so callers can use
/// ASCII, box drawing, arrows, or any other terminal symbols.
#[derive(Clone, Debug)]
pub struct Symbols {
    pub branch: String,
    pub last: String,
    pub vertical: String,
    pub blank: String,
    pub collapsed: String,
    pub expanded: String,
    pub leaf: String,
}

impl Default for Symbols {
    fn default() -> Self {
        Self {
            branch: "├──".into(),
            last: "└──".into(),
            vertical: "│  ".into(),
            blank: "   ".into(),
            collapsed: "▸ ".into(),
            expanded: "▾ ".into(),
            leaf: "  ".into(),
        }
    }
}

/// Visual styles for a [`TreePlus`].
///
/// `selected` is patched over every span of the selected row. Keep its
/// foreground unset if the row's per-span colours must survive selection.
#[derive(Clone, Debug)]
pub struct Styles {
    pub branch: Style,
    pub arrow: Style,
    pub content: Style,
    pub selected: Style,
    pub skippable: Style,
    pub empty: Style,
}

impl Default for Styles {
    fn default() -> Self {
        Self {
            branch: Style::default().fg(Color::Indexed(240)),
            arrow: Style::default().fg(Color::Indexed(212)),
            content: Style::default(),
            selected: Style::default()
                .bg(Color::Indexed(236))
                .add_modifier(Modifier::BOLD),
            skippable: Style::default().add_modifier(Modifier::DIM),
            empty: Style::default().fg(Color::Indexed(240)),
        }
    }
}

/// Key bindings for [`TreePlus`].
#[derive(Clone, Debug)]
pub struct KeyMap {
    pub up: Binding,
    pub down: Binding,
    pub page_up: Binding,
    pub page_down: Binding,
    pub home: Binding,
    pub end: Binding,
    pub collapse: Binding,
    pub expand: Binding,
    pub toggle: Binding,
    pub activate: Binding,
}

impl Default for KeyMap {
    fn default() -> Self {
        Self {
            up: Binding::new([
                key::with_keys(&["up", "k", "ctrl+p"]),
                key::with_help("↑/k", "up"),
            ]),
            down: Binding::new([
                key::with_keys(&["down", "j", "ctrl+n"]),
                key::with_help("↓/j", "down"),
            ]),
            page_up: Binding::new([
                key::with_keys(&["pgup", "ctrl+b"]),
                key::with_help("pgup", "page up"),
            ]),
            page_down: Binding::new([
                key::with_keys(&["pgdown", "ctrl+f"]),
                key::with_help("pgdown", "page down"),
            ]),
            home: Binding::new([
                key::with_keys(&["home", "g"]),
                key::with_help("g/home", "first"),
            ]),
            end: Binding::new([
                key::with_keys(&["end", "G"]),
                key::with_help("G/end", "last"),
            ]),
            collapse: Binding::new([
                key::with_keys(&["left", "h"]),
                key::with_help("←/h", "collapse"),
            ]),
            expand: Binding::new([
                key::with_keys(&["right", "l"]),
                key::with_help("→/l", "expand"),
            ]),
            toggle: Binding::new([key::with_keys(&[" "]), key::with_help("space", "toggle")]),
            activate: Binding::new([
                key::with_keys(&["enter"]),
                key::with_help("enter", "activate"),
            ]),
        }
    }
}

/// An event produced by [`TreePlus::handle_key`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeEvent {
    None,
    Selected { id: String },
    Activated { id: String },
    Collapsed { id: String },
    Expanded { id: String },
}

#[derive(Clone, Debug)]
struct FlatRow {
    id: String,
    content: Content,
    ancestor_last: Vec<bool>,
    is_last: bool,
    has_children: bool,
    collapsed: bool,
    skippable: bool,
    parent: Option<String>,
}

/// Stateful, walkable tree.
#[derive(Default)]
pub struct TreePlus {
    nodes: Vec<Node>,
    collapsed: BTreeSet<String>,
    selected: Option<String>,
    offset: usize,
    width: usize,
    height: usize,
    pub symbols: Symbols,
    pub styles: Styles,
    pub key_map: KeyMap,
}

impl TreePlus {
    pub fn new(nodes: Vec<Node>) -> Self {
        let mut tree = Self::default();
        tree.set_nodes(nodes);
        tree
    }

    /// Replace the tree while preserving the selected and collapsed IDs.
    pub fn set_nodes(&mut self, nodes: Vec<Node>) {
        let previous = self.selected.clone();
        self.nodes = nodes;
        let ids = collect_ids(&self.nodes);
        self.collapsed.retain(|id| ids.contains(id));
        self.selected = previous.filter(|id| {
            self.flatten()
                .iter()
                .any(|row| &row.id == id && !row.skippable)
        });
        self.normalize_selection();
        self.clamp_offset();
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn set_size(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.clamp_offset();
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn selected_node(&self) -> Option<&Node> {
        let id = self.selected.as_deref()?;
        find_node(&self.nodes, id)
    }

    pub fn is_collapsed(&self, id: &str) -> bool {
        self.collapsed.contains(id)
    }

    /// Visible node IDs in render order.
    pub fn visible_ids(&self) -> Vec<String> {
        self.flatten().into_iter().map(|row| row.id).collect()
    }

    pub fn select(&mut self, id: &str) -> bool {
        let rows = self.flatten();
        if !rows.iter().any(|row| row.id == id && !row.skippable) {
            return false;
        }
        self.selected = Some(id.to_string());
        self.ensure_visible();
        true
    }

    pub fn collapse(&mut self, id: &str) -> bool {
        let Some(node) = find_node(&self.nodes, id) else {
            return false;
        };
        if node.children.is_empty() {
            return false;
        }
        let hidden = self
            .selected
            .as_ref()
            .is_some_and(|selected| is_descendant(&self.nodes, selected, id));
        let inserted = self.collapsed.insert(id.to_string());
        if hidden {
            self.selected = Some(id.to_string());
        }
        self.normalize_selection();
        inserted
    }

    pub fn expand(&mut self, id: &str) -> bool {
        let removed = self.collapsed.remove(id);
        if removed {
            self.normalize_selection();
        }
        removed
    }

    pub fn toggle(&mut self, id: &str) -> Option<bool> {
        let node = find_node(&self.nodes, id)?;
        if node.children.is_empty() {
            return None;
        }
        if self.collapsed.contains(id) {
            self.expand(id);
            Some(true)
        } else {
            self.collapse(id);
            Some(false)
        }
    }

    pub fn handle_key(&mut self, event: &KeyEvent) -> TreeEvent {
        if key::matches(event, [&self.key_map.up]) {
            return self.move_selection(-1);
        }
        if key::matches(event, [&self.key_map.down]) {
            return self.move_selection(1);
        }
        if key::matches(event, [&self.key_map.page_up]) {
            return self.move_selection(-(self.height.max(1) as isize));
        }
        if key::matches(event, [&self.key_map.page_down]) {
            return self.move_selection(self.height.max(1) as isize);
        }
        if key::matches(event, [&self.key_map.home]) {
            return self.select_edge(false);
        }
        if key::matches(event, [&self.key_map.end]) {
            return self.select_edge(true);
        }
        if key::matches(event, [&self.key_map.collapse]) {
            return self.collapse_or_parent();
        }
        if key::matches(event, [&self.key_map.expand]) {
            return self.expand_or_child();
        }
        if key::matches(event, [&self.key_map.toggle]) {
            return self.toggle_selected();
        }
        if key::matches(event, [&self.key_map.activate]) {
            return self
                .selected
                .clone()
                .map(|id| TreeEvent::Activated { id })
                .unwrap_or(TreeEvent::None);
        }
        TreeEvent::None
    }

    /// Draw only the viewport, keeping the selected row visible.
    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        self.width = area.width as usize;
        self.height = area.height as usize;
        self.ensure_visible();
        let lines = self.lines(self.width);
        for (index, line) in lines.iter().skip(self.offset).take(self.height).enumerate() {
            buf.set_line(area.x, area.y + index as u16, line, area.width);
        }
    }

    /// Build all visible lines at the requested width.
    pub fn lines(&self, width: usize) -> Vec<Line<'static>> {
        let rows = self.flatten();
        if rows.is_empty() {
            return vec![Line::from(Span::styled("  (empty)", self.styles.empty))];
        }
        rows.iter().map(|row| self.row_line(row, width)).collect()
    }

    fn row_line(&self, row: &FlatRow, width: usize) -> Line<'static> {
        let selected = self.selected.as_deref() == Some(row.id.as_str());
        let row_style = if row.skippable {
            self.styles.skippable
        } else if selected {
            self.styles.selected
        } else {
            Style::default()
        };
        let mut spans = Vec::new();
        for ancestor_last in &row.ancestor_last {
            let symbol = if *ancestor_last {
                &self.symbols.blank
            } else {
                &self.symbols.vertical
            };
            spans.push(Span::styled(
                symbol.clone(),
                self.styles.branch.patch(row_style),
            ));
        }
        spans.push(Span::styled(
            if row.is_last {
                self.symbols.last.clone()
            } else {
                self.symbols.branch.clone()
            },
            self.styles.branch.patch(row_style),
        ));
        let arrow = if row.has_children {
            if row.collapsed {
                &self.symbols.collapsed
            } else {
                &self.symbols.expanded
            }
        } else {
            &self.symbols.leaf
        };
        spans.push(Span::styled(
            arrow.clone(),
            self.styles.arrow.patch(row_style),
        ));
        let prefix_width: usize = spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum();
        let content = row.content.spans(self.styles.content);
        let mut content: Vec<Span<'static>> = content
            .into_iter()
            .map(|mut span| {
                span.style = span.style.patch(row_style);
                span
            })
            .collect();
        content = truncate_spans(content, width.saturating_sub(prefix_width));
        spans.extend(content);
        let used: usize = spans
            .iter()
            .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
            .sum();
        if selected && width > used {
            spans.push(Span::styled(" ".repeat(width - used), self.styles.selected));
        }
        Line::from(spans)
    }

    fn flatten(&self) -> Vec<FlatRow> {
        let mut rows = Vec::new();
        self.flatten_into(&self.nodes, &mut Vec::new(), None, &mut rows);
        rows
    }

    fn flatten_into(
        &self,
        nodes: &[Node],
        ancestors: &mut Vec<bool>,
        parent: Option<&str>,
        rows: &mut Vec<FlatRow>,
    ) {
        for (index, node) in nodes.iter().enumerate() {
            let is_last = index + 1 == nodes.len();
            let collapsed = self.collapsed.contains(&node.id);
            rows.push(FlatRow {
                id: node.id.clone(),
                content: node.content.clone(),
                ancestor_last: ancestors.clone(),
                is_last,
                has_children: !node.children.is_empty(),
                collapsed,
                skippable: node.skippable,
                parent: parent.map(str::to_string),
            });
            if !node.children.is_empty() && !collapsed {
                ancestors.push(is_last);
                self.flatten_into(&node.children, ancestors, Some(&node.id), rows);
                ancestors.pop();
            }
        }
    }

    fn normalize_selection(&mut self) {
        let rows = self.flatten();
        let valid = self
            .selected
            .as_ref()
            .is_some_and(|id| rows.iter().any(|row| &row.id == id && !row.skippable));
        if !valid {
            self.selected = rows
                .iter()
                .find(|row| !row.skippable)
                .map(|row| row.id.clone());
        }
        self.clamp_offset();
    }

    fn clamp_offset(&mut self) {
        if self.height == 0 {
            self.offset = 0;
            return;
        }
        let rows = self.flatten();
        let max = rows.len().saturating_sub(self.height);
        self.offset = self.offset.min(max);
    }

    fn ensure_visible(&mut self) {
        if self.height == 0 {
            return;
        }
        let rows = self.flatten();
        let Some(index) = self
            .selected
            .as_ref()
            .and_then(|id| rows.iter().position(|row| &row.id == id))
        else {
            self.offset = 0;
            return;
        };
        if index < self.offset {
            self.offset = index;
        }
        if index >= self.offset + self.height {
            self.offset = index + 1 - self.height;
        }
        self.clamp_offset();
    }

    fn selectable_indices(&self, rows: &[FlatRow]) -> Vec<usize> {
        rows.iter()
            .enumerate()
            .filter_map(|(index, row)| (!row.skippable).then_some(index))
            .collect()
    }

    fn move_selection(&mut self, delta: isize) -> TreeEvent {
        let rows = self.flatten();
        let selectable = self.selectable_indices(&rows);
        if selectable.is_empty() {
            return TreeEvent::None;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| selectable.iter().position(|index| rows[*index].id == *id))
            .unwrap_or(0);
        let next = (current as isize + delta).clamp(0, selectable.len() as isize - 1) as usize;
        let id = rows[selectable[next]].id.clone();
        if self.selected.as_deref() == Some(id.as_str()) {
            return TreeEvent::None;
        }
        self.selected = Some(id.clone());
        self.ensure_visible();
        TreeEvent::Selected { id }
    }

    fn select_edge(&mut self, end: bool) -> TreeEvent {
        let rows = self.flatten();
        let selectable = self.selectable_indices(&rows);
        let Some(index) = (if end {
            selectable.last()
        } else {
            selectable.first()
        }) else {
            return TreeEvent::None;
        };
        let id = rows[*index].id.clone();
        if self.selected.as_deref() == Some(id.as_str()) {
            return TreeEvent::None;
        }
        self.selected = Some(id.clone());
        self.ensure_visible();
        TreeEvent::Selected { id }
    }

    fn collapse_or_parent(&mut self) -> TreeEvent {
        let Some(selected) = self.selected.clone() else {
            return TreeEvent::None;
        };
        let Some(node) = find_node(&self.nodes, &selected) else {
            return TreeEvent::None;
        };
        if !node.children.is_empty() && !self.collapsed.contains(&selected) {
            self.collapse(&selected);
            return TreeEvent::Collapsed { id: selected };
        }
        self.select_parent(&selected)
    }

    fn expand_or_child(&mut self) -> TreeEvent {
        let Some(selected) = self.selected.clone() else {
            return TreeEvent::None;
        };
        let Some(node) = find_node(&self.nodes, &selected) else {
            return TreeEvent::None;
        };
        if !node.children.is_empty() && self.collapsed.contains(&selected) {
            self.expand(&selected);
            return TreeEvent::Expanded { id: selected };
        }
        self.select_first_descendant(&selected)
    }

    fn toggle_selected(&mut self) -> TreeEvent {
        let Some(selected) = self.selected.clone() else {
            return TreeEvent::None;
        };
        let Some(node) = find_node(&self.nodes, &selected) else {
            return TreeEvent::None;
        };
        if node.children.is_empty() {
            return TreeEvent::None;
        }
        if self.collapsed.contains(&selected) {
            self.expand(&selected);
            TreeEvent::Expanded { id: selected }
        } else {
            self.collapse(&selected);
            TreeEvent::Collapsed { id: selected }
        }
    }

    fn select_parent(&mut self, id: &str) -> TreeEvent {
        let rows = self.flatten();
        let mut parent = rows
            .iter()
            .find(|row| row.id == id)
            .and_then(|row| row.parent.clone());
        while let Some(candidate) = parent {
            let row = rows.iter().find(|row| row.id == candidate);
            let Some(row) = row else {
                break;
            };
            if !row.skippable {
                if self.selected.as_deref() == Some(row.id.as_str()) {
                    return TreeEvent::None;
                }
                self.selected = Some(row.id.clone());
                self.ensure_visible();
                return TreeEvent::Selected { id: row.id.clone() };
            }
            parent = row.parent.clone();
        }
        TreeEvent::None
    }

    fn select_first_descendant(&mut self, id: &str) -> TreeEvent {
        let rows = self.flatten();
        let Some(index) = rows.iter().position(|row| row.id == id) else {
            return TreeEvent::None;
        };
        let depth = rows[index].ancestor_last.len();
        for row in rows.iter().skip(index + 1) {
            if row.ancestor_last.len() <= depth {
                break;
            }
            if !row.skippable {
                if self.selected.as_deref() == Some(row.id.as_str()) {
                    return TreeEvent::None;
                }
                self.selected = Some(row.id.clone());
                self.ensure_visible();
                return TreeEvent::Selected { id: row.id.clone() };
            }
        }
        TreeEvent::None
    }

    /// Mouse is accepted for API symmetry with other widgets. Tree navigation
    /// is keyboard-driven; callers can map clicks themselves through `select`.
    pub fn handle_mouse(&mut self, _event: &MouseEvent) {}
}

impl ratatui::widgets::Widget for &mut TreePlus {
    fn render(self, area: Rect, buf: &mut Buffer) {
        TreePlus::render(self, area, buf);
    }
}

fn find_node<'a>(nodes: &'a [Node], id: &str) -> Option<&'a Node> {
    for node in nodes {
        if node.id == id {
            return Some(node);
        }
        if let Some(found) = find_node(&node.children, id) {
            return Some(found);
        }
    }
    None
}

fn is_descendant(nodes: &[Node], child: &str, ancestor: &str) -> bool {
    fn walk(nodes: &[Node], child: &str, ancestor: &str, inside: bool) -> bool {
        for node in nodes {
            if node.id == child {
                return inside;
            }
            if walk(
                &node.children,
                child,
                ancestor,
                inside || node.id == ancestor,
            ) {
                return true;
            }
        }
        false
    }
    walk(nodes, child, ancestor, false)
}

fn collect_ids(nodes: &[Node]) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    fn walk(nodes: &[Node], ids: &mut BTreeSet<String>) {
        for node in nodes {
            ids.insert(node.id.clone());
            walk(&node.children, ids);
        }
    }
    walk(nodes, &mut ids);
    ids
}

fn truncate_spans(spans: Vec<Span<'static>>, width: usize) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    let total: usize = spans
        .iter()
        .map(|span| UnicodeWidthStr::width(span.content.as_ref()))
        .sum();
    if total <= width {
        return spans;
    }
    let limit = width.saturating_sub(1);
    let mut out = Vec::new();
    let mut used = 0usize;
    let mut last_style = Style::default();
    for span in spans {
        let mut text = String::new();
        for ch in span.content.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
            if used + ch_width > limit {
                break;
            }
            text.push(ch);
            used += ch_width;
        }
        last_style = span.style;
        if !text.is_empty() {
            out.push(Span::styled(text, span.style));
        }
        if used >= limit {
            break;
        }
    }
    out.push(Span::styled("…", last_style));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn sample() -> TreePlus {
        TreePlus::new(vec![
            Node::new("root", "Root").children([
                Node::new("a", "Alpha"),
                Node::new("section", "Section")
                    .skippable(true)
                    .children([Node::new("a1", "Alpha one")]),
                Node::new("b", "Beta").children([Node::new("b1", "Beta one")]),
            ]),
        ])
    }

    #[test]
    fn navigation_skips_skippable_nodes() {
        let mut tree = sample();
        assert_eq!(tree.selected_id(), Some("root"));
        assert_eq!(
            tree.handle_key(&key(KeyCode::Down)),
            TreeEvent::Selected { id: "a".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Down)),
            TreeEvent::Selected { id: "a1".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Down)),
            TreeEvent::Selected { id: "b".into() }
        );
        assert_ne!(tree.selected_id(), Some("section"));
    }

    #[test]
    fn collapse_hides_descendants_and_expand_restores_them() {
        let mut tree = sample();
        tree.select("b");
        assert!(tree.visible_ids().contains(&"b1".to_string()));
        assert_eq!(
            tree.handle_key(&key(KeyCode::Left)),
            TreeEvent::Collapsed { id: "b".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Left)),
            TreeEvent::Selected { id: "root".into() }
        );
        assert!(!tree.visible_ids().contains(&"b1".to_string()));
        assert!(tree.select("b"));
        assert_eq!(
            tree.handle_key(&key(KeyCode::Right)),
            TreeEvent::Expanded { id: "b".into() }
        );
        assert!(tree.visible_ids().contains(&"b1".to_string()));
    }

    #[test]
    fn toggle_and_activate_emit_events() {
        let mut tree = sample();
        tree.select("b");
        assert_eq!(
            tree.handle_key(&key(KeyCode::Char(' '))),
            TreeEvent::Collapsed { id: "b".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Char(' '))),
            TreeEvent::Expanded { id: "b".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Enter)),
            TreeEvent::Activated { id: "b".into() }
        );
    }

    #[test]
    fn set_nodes_preserves_selection_and_collapse_by_id() {
        let mut tree = sample();
        tree.select("b");
        tree.collapse("b");
        tree.set_nodes(vec![Node::new("root", "Root").children([
            Node::new("b", "Beta reloaded").children([Node::new("b1", "Beta one")]),
        ])]);
        assert_eq!(tree.selected_id(), Some("b"));
        assert!(tree.is_collapsed("b"));
        assert!(!tree.visible_ids().contains(&"b1".to_string()));
    }

    #[test]
    fn compound_content_keeps_per_span_colours_when_selected() {
        let mut tree = TreePlus::new(vec![Node::new(
            "row",
            Content::from(vec![
                Span::styled("ID", Style::default().fg(Color::Indexed(244))),
                Span::styled(" Status", Style::default().fg(Color::Indexed(212))),
                Span::styled(" Description", Style::default().fg(Color::Indexed(250))),
            ]),
        )]);
        let lines = tree.lines(60);
        let spans = &lines[0].spans;
        let id = spans
            .iter()
            .find(|span| span.content.as_ref() == "ID")
            .unwrap();
        let status = spans
            .iter()
            .find(|span| span.content.as_ref() == " Status")
            .unwrap();
        let description = spans
            .iter()
            .find(|span| span.content.as_ref() == " Description")
            .unwrap();
        assert_eq!(id.style.fg, Some(Color::Indexed(244)));
        assert_eq!(status.style.fg, Some(Color::Indexed(212)));
        assert_eq!(description.style.fg, Some(Color::Indexed(250)));
        assert_eq!(id.style.bg, Some(Color::Indexed(236)));
        tree.select("row");
        let lines = tree.lines(60);
        let id = lines[0]
            .spans
            .iter()
            .find(|span| span.content.as_ref() == "ID")
            .unwrap();
        assert_eq!(id.style.fg, Some(Color::Indexed(244)));
    }

    #[test]
    fn custom_arrows_and_branch_symbols_are_used() {
        let mut tree = sample();
        tree.symbols.branch = "+->".into();
        tree.symbols.last = "`->".into();
        tree.symbols.collapsed = ">".into();
        tree.symbols.expanded = "v".into();
        tree.select("b");
        tree.collapse("b");
        let text: String = tree
            .lines(80)
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect();
        assert!(text.contains("`->"), "{text}");
        assert!(text.contains("+->"), "{text}");
        assert!(!text.contains("├──"), "{text}");
        assert!(text.contains('>'), "{text}");
    }

    #[test]
    fn page_home_end_and_selection_survive_rebuilds() {
        let mut tree = TreePlus::new(vec![
            Node::new("one", "One"),
            Node::new("two", "Two"),
            Node::new("three", "Three"),
        ]);
        tree.set_size(40, 1);
        assert_eq!(
            tree.handle_key(&key(KeyCode::End)),
            TreeEvent::Selected { id: "three".into() }
        );
        assert_eq!(
            tree.handle_key(&key(KeyCode::Home)),
            TreeEvent::Selected { id: "one".into() }
        );
        tree.select("two");
        tree.set_nodes(vec![
            Node::new("one", "One"),
            Node::new("two", "Two updated"),
            Node::new("three", "Three"),
        ]);
        assert_eq!(tree.selected_id(), Some("two"));
    }
}
