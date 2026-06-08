use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub type StyleFn = Arc<dyn Fn(&[TreeNode], usize) -> Style + Send + Sync + 'static>;
pub type Enumerator = Arc<dyn Fn(&[TreeNode], usize) -> String + Send + Sync + 'static>;
pub type Indenter = Arc<dyn Fn(&[TreeNode], usize) -> String + Send + Sync + 'static>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    value: String,
    hidden: bool,
    children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(value: impl Into<String>) -> Self {
        Self { value: value.into(), hidden: false, children: Vec::new() }
    }

    pub fn hidden(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn push(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn children_ref(&self) -> &[TreeNode] {
        &self.children
    }
}

pub fn default_enumerator(children: &[TreeNode], index: usize) -> String {
    if index + 1 == children.len() { "└──".into() } else { "├──".into() }
}

pub fn rounded_enumerator(children: &[TreeNode], index: usize) -> String {
    if index + 1 == children.len() { "╰──".into() } else { "├──".into() }
}

pub fn default_indenter(children: &[TreeNode], index: usize) -> String {
    if index + 1 == children.len() { "   ".into() } else { "│  ".into() }
}

fn default_prefix_style(_: &[TreeNode], _: usize) -> Style {
    Style::default()
}

#[derive(Clone)]
pub struct Tree {
    root: Option<String>,
    hidden: bool,
    width: Option<u16>,
    children: Vec<TreeNode>,
    root_style: Style,
    enumerator_style: StyleFn,
    indenter_style: StyleFn,
    item_style: StyleFn,
    enumerator: Enumerator,
    indenter: Indenter,
}

impl Default for Tree {
    fn default() -> Self {
        Self {
            root: None,
            hidden: false,
            width: None,
            children: Vec::new(),
            root_style: Style::default(),
            enumerator_style: Arc::new(default_prefix_style),
            indenter_style: Arc::new(default_prefix_style),
            item_style: Arc::new(default_prefix_style),
            enumerator: Arc::new(default_enumerator),
            indenter: Arc::new(default_indenter),
        }
    }
}

impl Tree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root(mut self, root: impl Into<String>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn hide(mut self, hidden: bool) -> Self {
        self.hidden = hidden;
        self
    }

    pub fn width(mut self, width: u16) -> Self {
        self.width = Some(width);
        self
    }

    pub fn root_style(mut self, style: Style) -> Self {
        self.root_style = style;
        self
    }

    pub fn enumerator_style(mut self, style: Style) -> Self {
        self.enumerator_style = Arc::new(move |_, _| style);
        self
    }

    pub fn enumerator_style_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[TreeNode], usize) -> Style + Send + Sync + 'static,
    {
        self.enumerator_style = Arc::new(f);
        self
    }

    pub fn indenter_style(mut self, style: Style) -> Self {
        self.indenter_style = Arc::new(move |_, _| style);
        self
    }

    pub fn indenter_style_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[TreeNode], usize) -> Style + Send + Sync + 'static,
    {
        self.indenter_style = Arc::new(f);
        self
    }

    pub fn item_style(mut self, style: Style) -> Self {
        self.item_style = Arc::new(move |_, _| style);
        self
    }

    pub fn item_style_fn<F>(mut self, f: F) -> Self
    where
        F: Fn(&[TreeNode], usize) -> Style + Send + Sync + 'static,
    {
        self.item_style = Arc::new(f);
        self
    }

    pub fn enumerator<F>(mut self, f: F) -> Self
    where
        F: Fn(&[TreeNode], usize) -> String + Send + Sync + 'static,
    {
        self.enumerator = Arc::new(f);
        self
    }

    pub fn indenter<F>(mut self, f: F) -> Self
    where
        F: Fn(&[TreeNode], usize) -> String + Send + Sync + 'static,
    {
        self.indenter = Arc::new(f);
        self
    }

    pub fn child(mut self, child: TreeNode) -> Self {
        self.children.push(child);
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = TreeNode>) -> Self {
        self.children.extend(children);
        self
    }

    pub fn push(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    pub fn lines(&self) -> Vec<Line<'static>> {
        let mut out = Vec::new();
        if self.hidden {
            return out;
        }

        if let Some(root) = &self.root
            && !root.is_empty()
        {
            out.push(Line::styled(pad_to_width(root.clone(), self.width), self.root_style));
        }

        self.render_nodes(&self.children, String::new(), &mut out);
        out
    }

    fn render_nodes(&self, nodes: &[TreeNode], inherited_prefix: String, out: &mut Vec<Line<'static>>) {
        let visible: Vec<&TreeNode> = nodes.iter().filter(|node| !node.hidden).collect();
        if visible.is_empty() {
            return;
        }

        let siblings: Vec<TreeNode> = visible.iter().map(|node| (*node).clone()).collect();
        let prefixes: Vec<String> = (0..siblings.len()).map(|idx| (self.enumerator)(&siblings, idx)).collect();
        let max_prefix_width = prefixes.iter().map(|text| UnicodeWidthStr::width(text.as_str())).max().unwrap_or(0);

        for (idx, node) in visible.iter().enumerate() {
            let enum_text = prefixes[idx].clone();
            let indent_text = (self.indenter)(&siblings, idx);
            let enum_style = (self.enumerator_style)(&siblings, idx);
            let indent_style = (self.indenter_style)(&siblings, idx);
            let item_style = (self.item_style)(&siblings, idx);
            let prefix_pad = max_prefix_width.saturating_sub(UnicodeWidthStr::width(enum_text.as_str()));

            let content_width = self
                .width
                .map(|width| {
                    let prefix_width = UnicodeWidthStr::width(inherited_prefix.as_str()) + prefix_pad + UnicodeWidthStr::width(enum_text.as_str());
                    width as usize - prefix_width.min(width as usize)
                })
                .unwrap_or(usize::MAX);
            let node_lines = wrap_value(&node.value, content_width.max(1));

            for (line_idx, value_line) in node_lines.iter().enumerate() {
                let mut segments = Vec::with_capacity(4);
                if !inherited_prefix.is_empty() {
                    segments.push((inherited_prefix.clone(), indent_style));
                }
                if prefix_pad > 0 {
                    segments.push((" ".repeat(prefix_pad), enum_style));
                }
                if line_idx == 0 {
                    segments.push((enum_text.clone(), enum_style));
                } else {
                    segments.push((indent_text.clone(), indent_style));
                }
                segments.push((value_line.clone(), item_style));
                out.push(styled_line(pad_segments_to_width(segments, self.width)));
            }

            if !node.children.is_empty() {
                self.render_nodes(&node.children, format!("{inherited_prefix}{indent_text}"), out);
            }
        }
    }
}

impl Widget for &Tree {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let tree = self.clone().width(area.width);
        let lines = tree.lines();
        for (row, line) in lines.into_iter().take(area.height as usize).enumerate() {
            buf.set_line(area.x, area.y + row as u16, &line, area.width);
        }
    }
}

fn styled_line(parts: Vec<(String, Style)>) -> Line<'static> {
    Line::from(parts.into_iter().map(|(text, style)| Span::styled(text, style)).collect::<Vec<_>>())
}

fn pad_segments_to_width(mut parts: Vec<(String, Style)>, width: Option<u16>) -> Vec<(String, Style)> {
    if let Some(width) = width {
        let used = parts.iter().map(|(text, _)| UnicodeWidthStr::width(text.as_str())).sum::<usize>();
        if used < width as usize {
            let style = parts.last().map(|(_, style)| *style).unwrap_or_default();
            parts.push((" ".repeat(width as usize - used), style));
        }
    }
    parts
}

fn pad_to_width(mut text: String, width: Option<u16>) -> String {
    if let Some(width) = width {
        let used = UnicodeWidthStr::width(text.as_str());
        if used < width as usize {
            text.push_str(&" ".repeat(width as usize - used));
        }
    }
    text
}

fn wrap_value(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    for raw_line in text.replace("\r\n", "\n").split('\n') {
        let mut current = String::new();
        let mut used = 0usize;
        for ch in raw_line.chars() {
            let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0).max(1);
            if used + ch_width > width && !current.is_empty() {
                out.push(current.clone());
                current.clear();
                used = 0;
            }
            current.push(ch);
            used += ch_width;
        }
        out.push(current);
    }

    if out.is_empty() { vec![String::new()] } else { out }
}
