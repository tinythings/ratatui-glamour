use ratatui::{buffer::Buffer, layout::Rect, style::Style, widgets::Widget};

use crate::tree::{Tree, TreeNode};

pub type Enumerator = fn(&[TreeNode], usize) -> String;

pub fn alphabet(_: &[TreeNode], i: usize) -> String {
    const ABC: usize = 26;
    if i >= ABC * ABC + ABC {
        format!("{}{}{}.", nth_alpha(i / ABC / ABC - 1), nth_alpha((i / ABC) % ABC - 1), nth_alpha(i % ABC))
    } else if i >= ABC {
        format!("{}{}.", nth_alpha(i / ABC - 1), nth_alpha(i % ABC))
    } else {
        format!("{}.", nth_alpha(i % ABC))
    }
}

pub fn arabic(_: &[TreeNode], i: usize) -> String {
    format!("{}.", i + 1)
}

pub fn roman(_: &[TreeNode], mut i: usize) -> String {
    let roman = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"), (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")];
    i += 1;
    let mut out = String::new();
    for (value, symbol) in roman {
        while i >= value {
            i -= value;
            out.push_str(symbol);
        }
    }
    out.push('.');
    out
}

pub fn bullet(_: &[TreeNode], _: usize) -> String {
    "•".into()
}

pub fn asterisk(_: &[TreeNode], _: usize) -> String {
    "*".into()
}

pub fn dash(_: &[TreeNode], _: usize) -> String {
    "-".into()
}

#[derive(Clone, Default)]
pub struct List {
    tree: Tree,
}

impl List {
    pub fn new() -> Self {
        Self { tree: Tree::new().enumerator(bullet).indenter(|_, _| " ".into()) }
    }

    pub fn width(mut self, width: u16) -> Self {
        self.tree = self.tree.width(width);
        self
    }

    pub fn hide(mut self, hidden: bool) -> Self {
        self.tree = self.tree.hide(hidden);
        self
    }

    pub fn enumerator(mut self, enumerator: Enumerator) -> Self {
        self.tree = self.tree.enumerator(enumerator);
        self
    }

    pub fn enumerator_style(mut self, style: Style) -> Self {
        self.tree = self.tree.enumerator_style(style);
        self
    }

    pub fn indenter_style(mut self, style: Style) -> Self {
        self.tree = self.tree.indenter_style(style);
        self
    }

    pub fn item_style(mut self, style: Style) -> Self {
        self.tree = self.tree.item_style(style);
        self
    }

    pub fn item(mut self, item: impl Into<ListItem>) -> Self {
        self.tree = self.tree.child(item.into().into_node());
        self
    }

    pub fn items<I, T>(mut self, items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ListItem>,
    {
        for item in items {
            self.tree = self.tree.child(item.into().into_node());
        }
        self
    }

    pub fn tree(&self) -> &Tree {
        &self.tree
    }
}

impl Widget for &List {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.tree().render(area, buf);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListItem {
    Text(String),
    Nested(Vec<ListItem>),
}

impl ListItem {
    fn into_node(self) -> TreeNode {
        match self {
            Self::Text(text) => TreeNode::new(text),
            Self::Nested(items) => {
                let mut iter = items.into_iter();
                let Some(first) = iter.next() else {
                    return TreeNode::new(String::new());
                };
                let mut node = first.into_node();
                for item in iter {
                    node.push(item.into_node());
                }
                node
            }
        }
    }
}

impl From<&str> for ListItem {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

impl From<String> for ListItem {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<Vec<ListItem>> for ListItem {
    fn from(value: Vec<ListItem>) -> Self {
        Self::Nested(value)
    }
}

fn nth_alpha(index: usize) -> char {
    (b'A' + (index % 26) as u8) as char
}
