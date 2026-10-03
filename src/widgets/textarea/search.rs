//! Find and replace state for the textarea.

/// The active search UI.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchMode {
    /// The search bar is hidden.
    #[default]
    Closed,
    /// The find bar is open.
    Find,
    /// The find-and-replace bar is open.
    Replace,
}

/// Which field of the search bar has focus.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchField {
    /// The query field.
    #[default]
    Query,
    /// The replacement field (replace mode only).
    Replacement,
}

/// One match, in logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Match {
    /// The logical line.
    pub row: usize,
    /// The character column the match starts at.
    pub col: usize,
    /// The number of characters in the match.
    pub len: usize,
}

/// The find/replace state: the query, its matches, and the active match.
#[derive(Clone, Debug, Default)]
pub struct Search {
    /// The open mode.
    pub mode: SearchMode,
    /// The focused field.
    pub field: SearchField,
    /// The search query.
    pub query: String,
    /// The replacement text.
    pub replacement: String,
    /// Every match, in document order.
    pub matches: Vec<Match>,
    /// The index of the active match.
    pub current: usize,
}

impl Search {
    /// Whether the search bar is open.
    pub fn open(&self) -> bool {
        self.mode != SearchMode::Closed
    }

    /// Whether the search bar is hidden.
    pub fn closed(&self) -> bool {
        self.mode == SearchMode::Closed
    }

    /// The number of matches.
    pub fn count(&self) -> usize {
        self.matches.len()
    }

    /// The active match, if any.
    pub fn current_match(&self) -> Option<Match> {
        self.matches.get(self.current).copied()
    }

    /// Recomputes the matches against the logical lines, non-overlapping.
    pub fn recompute(&mut self, lines: &[Vec<char>]) {
        self.matches.clear();
        let needle: Vec<char> = self.query.chars().collect();
        if needle.is_empty() {
            self.current = 0;
            return;
        }
        for (row, line) in lines.iter().enumerate() {
            let mut col = 0;
            while col + needle.len() <= line.len() {
                if line[col..col + needle.len()] == needle[..] {
                    self.matches.push(Match {
                        row,
                        col,
                        len: needle.len(),
                    });
                    col += needle.len();
                } else {
                    col += 1;
                }
            }
        }
        if self.current >= self.matches.len() {
            self.current = 0;
        }
    }

    /// Moves the active match by `delta`, wrapping around.
    pub fn advance(&mut self, delta: isize) {
        if self.matches.is_empty() {
            return;
        }
        let len = self.matches.len() as isize;
        self.current = (self.current as isize + delta).rem_euclid(len) as usize;
    }

    /// Replaces the active match in `lines`; returns whether one was replaced.
    pub fn replace_current(&mut self, lines: &mut [Vec<char>], replacement: &str) -> bool {
        let Some(m) = self.current_match() else {
            return false;
        };
        let repl: Vec<char> = replacement.chars().collect();
        lines[m.row].splice(m.col..m.col + m.len, repl);
        self.recompute(lines);
        true
    }

    /// Replaces every match in `lines`; returns how many were replaced.
    pub fn replace_all(&mut self, lines: &mut [Vec<char>], replacement: &str) -> usize {
        let needle: Vec<char> = self.query.chars().collect();
        if needle.is_empty() {
            return 0;
        }
        let repl: Vec<char> = replacement.chars().collect();
        let mut count = 0;
        for line in lines.iter_mut() {
            let mut out: Vec<char> = Vec::with_capacity(line.len());
            let mut col = 0;
            while col < line.len() {
                if col + needle.len() <= line.len() && line[col..col + needle.len()] == needle[..] {
                    out.extend_from_slice(&repl);
                    col += needle.len();
                    count += 1;
                } else {
                    out.push(line[col]);
                    col += 1;
                }
            }
            *line = out;
        }
        self.recompute(lines);
        count
    }
}
