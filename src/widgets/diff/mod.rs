//! File diff rendering.
//!
//! [`Model`] renders a file diff in split, stacked or unified layout. The
//! default auto layout uses split at widths of 80 columns or more and a
//! single-column stacked view below that threshold. It accepts two file
//! contents through [`Model::from_contents`] or precomputed unified diff text
//! through [`Model::from_unified`].
//!
//! The split layout mirrors Charmbracelet Crush: within a hunk, each deletion
//! is paired with the first following insertion before the next equal line.

mod parse;
mod render;
mod style;

pub use parse::Stats;
pub use style::{LineStyle, Styles};

use std::fmt;
use std::sync::Arc;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

type HighlightFn = dyn Fn(&str, Style) -> Vec<Span<'static>> + Send + Sync;

/// Callback used to syntax-highlight one source line.
///
/// The callback receives the source line and the diff's base code style.
/// Returned spans should carry the base background so the diff tint remains
/// visible.
#[derive(Clone)]
pub struct Highlighter(Arc<HighlightFn>);

impl Highlighter {
    /// Creates a highlighter from a closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&str, Style) -> Vec<Span<'static>> + Send + Sync + 'static,
    {
        Self(Arc::new(f))
    }

    fn highlight(&self, text: &str, base: Style) -> Vec<Span<'static>> {
        (self.0)(text, base)
    }
}

impl fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Highlighter(..)")
    }
}

/// Diff layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// Automatically choose split for widths of 80 columns or more, and
    /// stacked below that threshold.
    #[default]
    Auto,
    /// Old and new files rendered side by side.
    Split,
    /// Old and new cells rendered one above the other in a single column.
    Stacked,
    /// One combined column with `-` and `+` markers.
    Unified,
}

/// Minimum render width at which [`Layout::Auto`] selects the split layout.
pub const AUTO_SPLIT_MIN_WIDTH: usize = 80;

/// A limited diff view.
#[derive(Clone, Debug, Default)]
pub struct LimitedView {
    /// Visible lines.
    pub lines: Vec<Line<'static>>,
    /// Number of lines omitted after `lines`.
    pub hidden: usize,
}

#[derive(Clone, Debug)]
enum Source {
    Empty,
    Contents { before: String, after: String },
    Unified(String),
}

/// Diff rendering model.
#[derive(Clone, Debug)]
pub struct Model {
    source: Source,
    file_name: Option<String>,
    layout: Layout,
    context_lines: usize,
    line_numbers: bool,
    tab_width: usize,
    width: Option<usize>,
    styles: Styles,
    highlighter: Option<Highlighter>,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// Creates an empty auto-layout diff model.
    pub fn new() -> Self {
        Self {
            source: Source::Empty,
            file_name: None,
            layout: Layout::Auto,
            context_lines: 3,
            line_numbers: true,
            tab_width: 4,
            width: None,
            styles: Styles::default(),
            highlighter: None,
        }
    }

    /// Creates a model from before and after file contents.
    pub fn from_contents(before: impl Into<String>, after: impl Into<String>) -> Self {
        Self {
            source: Source::Contents {
                before: before.into(),
                after: after.into(),
            },
            ..Self::new()
        }
    }

    /// Creates a model from precomputed unified diff text.
    ///
    /// Both standard multi-hunk diffs and older single-hunk diffs without
    /// context are accepted.
    pub fn from_unified(diff: impl Into<String>) -> Self {
        Self {
            source: Source::Unified(diff.into()),
            ..Self::new()
        }
    }

    /// Sets the file name shown above the diff.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Selects the diff layout.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layout = layout;
        self
    }

    /// Selects the side-by-side layout.
    pub fn split(mut self) -> Self {
        self.layout = Layout::Split;
        self
    }

    /// Selects the single-column stacked layout.
    pub fn stacked(mut self) -> Self {
        self.layout = Layout::Stacked;
        self
    }

    /// Selects automatic layout by render width.
    pub fn auto(mut self) -> Self {
        self.layout = Layout::Auto;
        self
    }

    /// Selects the unified layout.
    pub fn unified(mut self) -> Self {
        self.layout = Layout::Unified;
        self
    }

    /// Sets the number of context lines used for content diffs.
    pub fn context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Shows or hides line-number gutters.
    pub fn line_numbers(mut self, show: bool) -> Self {
        self.line_numbers = show;
        self
    }

    /// Sets the number of spaces used for each tab.
    pub fn tab_width(mut self, width: usize) -> Self {
        self.tab_width = width;
        self
    }

    /// Sets the rendering width.
    ///
    /// When unset, [`Model::view`] uses the natural content width.
    pub fn width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    /// Replaces the visual theme.
    pub fn styles(mut self, styles: Styles) -> Self {
        self.styles = styles;
        self
    }

    /// Installs a syntax highlighter for source lines.
    pub fn highlighter(mut self, highlighter: Highlighter) -> Self {
        self.highlighter = Some(highlighter);
        self
    }

    /// Returns added and removed line counts.
    pub fn stats(&self) -> Stats {
        self.parsed().stats
    }

    /// Renders the complete diff.
    pub fn view(&self) -> Vec<Line<'static>> {
        let parsed = self.parsed();
        let width = self
            .width
            .unwrap_or_else(|| render::natural_width(&parsed, self));
        render::render(&parsed, self, width)
    }

    /// Renders the diff at a specific width.
    pub fn view_with_width(&self, width: usize) -> Vec<Line<'static>> {
        let parsed = self.parsed();
        render::render(&parsed, self, width)
    }

    /// Renders at most `max_rows` lines and reports how many were omitted.
    pub fn view_limited(&self, max_rows: usize) -> LimitedView {
        let lines = self.view();
        let total = lines.len();
        let lines = lines.into_iter().take(max_rows).collect();
        LimitedView {
            lines,
            hidden: total.saturating_sub(max_rows),
        }
    }

    /// Renders the diff into `area`, clipping to its height.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        for (offset, line) in self
            .view_with_width(area.width as usize)
            .into_iter()
            .take(area.height as usize)
            .enumerate()
        {
            buf.set_line(area.x, area.y + offset as u16, &line, area.width);
        }
    }

    fn highlight(&self, text: &str, base: Style) -> Vec<Span<'static>> {
        self.highlighter
            .as_ref()
            .map(|highlighter| highlighter.highlight(text, base))
            .unwrap_or_else(|| vec![Span::styled(text.to_string(), base)])
    }

    fn parsed(&self) -> parse::ParsedDiff {
        match &self.source {
            Source::Empty => parse::ParsedDiff::default(),
            Source::Contents { before, after } => {
                let unified = parse::unified_diff(before, after, self.context_lines);
                parse::parse_unified(&unified, self.tab_width)
            }
            Source::Unified(diff) => parse::parse_unified(diff, self.tab_width),
        }
    }
}

impl Widget for &Model {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Model::render(self, area, buf);
    }
}

#[cfg(test)]
#[path = "parse_ut.rs"]
mod parse_ut;

#[cfg(test)]
#[path = "render_ut.rs"]
mod render_ut;
