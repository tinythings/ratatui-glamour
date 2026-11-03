use similar::{Algorithm, TextDiff};

/// Number of added and removed lines in a parsed diff.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    /// Inserted line count.
    pub additions: usize,
    /// Deleted line count.
    pub removals: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Equal,
    Insert,
    Delete,
}

#[derive(Clone, Debug)]
pub(super) struct RawLine {
    pub kind: Kind,
    pub old_number: Option<usize>,
    pub new_number: Option<usize>,
    pub text: String,
}

#[derive(Clone, Debug)]
pub(super) struct Cell {
    pub number: usize,
    pub text: String,
    pub kind: Kind,
}

#[derive(Clone, Debug)]
pub(super) struct SplitRow {
    pub old: Option<Cell>,
    pub new: Option<Cell>,
}

#[derive(Clone, Debug)]
pub(super) struct Hunk {
    pub header: String,
    pub lines: Vec<RawLine>,
    pub rows: Vec<SplitRow>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ParsedDiff {
    pub hunks: Vec<Hunk>,
    pub stats: Stats,
}

/// Computes a Myers unified diff with standard file headers.
pub(super) fn unified_diff(before: &str, after: &str, context_lines: usize) -> String {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Myers)
        .diff_lines(before, after);
    let mut unified = diff.unified_diff();
    unified.context_radius(context_lines).header("a", "b");
    unified.to_string()
}

/// Parses unified diff text into hunks and split rows.
pub(super) fn parse_unified(diff: &str, tab_width: usize) -> ParsedDiff {
    let mut hunks = Vec::new();
    let mut stats = Stats::default();
    let mut current: Option<HunkBuilder> = None;

    for raw in diff.lines() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some((old_start, new_start)) = parse_hunk_header(line) {
            if let Some(builder) = current.take() {
                hunks.push(builder.finish());
            }
            current = Some(HunkBuilder::new(
                line.trim().to_string(),
                old_start,
                new_start,
            ));
            continue;
        }
        if let Some(builder) = current.as_mut() {
            builder.push(line, tab_width, &mut stats);
        }
    }

    if let Some(builder) = current {
        hunks.push(builder.finish());
    }

    ParsedDiff { hunks, stats }
}

struct HunkBuilder {
    header: String,
    old_number: usize,
    new_number: usize,
    lines: Vec<RawLine>,
}

impl HunkBuilder {
    fn new(header: String, old_start: usize, new_start: usize) -> Self {
        Self {
            header,
            old_number: old_start.max(1),
            new_number: new_start.max(1),
            lines: Vec::new(),
        }
    }

    fn push(&mut self, line: &str, tab_width: usize, stats: &mut Stats) {
        if line.starts_with('\\') {
            return;
        }
        let (kind, text) = if let Some(text) = line.strip_prefix(' ') {
            (Kind::Equal, text)
        } else if let Some(text) = line.strip_prefix('-') {
            (Kind::Delete, text)
        } else if let Some(text) = line.strip_prefix('+') {
            (Kind::Insert, text)
        } else if line.is_empty() {
            (Kind::Equal, "")
        } else {
            return;
        };
        let text = expand_tabs(text, tab_width);
        let (old_number, new_number) = match kind {
            Kind::Equal => {
                let numbers = (Some(self.old_number), Some(self.new_number));
                self.old_number += 1;
                self.new_number += 1;
                numbers
            }
            Kind::Delete => {
                let numbers = (Some(self.old_number), None);
                self.old_number += 1;
                stats.removals += 1;
                numbers
            }
            Kind::Insert => {
                let numbers = (None, Some(self.new_number));
                self.new_number += 1;
                stats.additions += 1;
                numbers
            }
        };
        self.lines.push(RawLine {
            kind,
            old_number,
            new_number,
            text,
        });
    }

    fn finish(self) -> Hunk {
        let rows = split_rows(&self.lines);
        Hunk {
            header: self.header,
            lines: self.lines,
            rows,
        }
    }
}

fn parse_hunk_header(line: &str) -> Option<(usize, usize)> {
    let rest = line.strip_prefix("@@ ")?;
    let end = rest.find(" @@")?;
    let mut ranges = rest[..end].split(' ');
    let old = ranges.next()?.strip_prefix('-')?;
    let new = ranges.next()?.strip_prefix('+')?;
    Some((parse_start(old)?, parse_start(new)?))
}

fn parse_start(range: &str) -> Option<usize> {
    range.split(',').next()?.parse().ok()
}

fn expand_tabs(text: &str, tab_width: usize) -> String {
    if tab_width == 0 || !text.contains('\t') {
        return text.to_string();
    }
    text.replace('\t', &" ".repeat(tab_width))
}

/// Pairs deletions with following insertions inside a hunk, mirroring Crush.
fn split_rows(lines: &[RawLine]) -> Vec<SplitRow> {
    let mut lines = lines.to_vec();
    let mut rows = Vec::with_capacity(lines.len());
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index].clone();
        match line.kind {
            Kind::Equal => rows.push(SplitRow {
                old: Some(Cell {
                    number: line.old_number.unwrap_or(1),
                    text: line.text.clone(),
                    kind: Kind::Equal,
                }),
                new: Some(Cell {
                    number: line.new_number.unwrap_or(1),
                    text: line.text,
                    kind: Kind::Equal,
                }),
            }),
            Kind::Insert => rows.push(SplitRow {
                old: None,
                new: Some(Cell {
                    number: line.new_number.unwrap_or(1),
                    text: line.text,
                    kind: Kind::Insert,
                }),
            }),
            Kind::Delete => {
                let mut row = SplitRow {
                    old: Some(Cell {
                        number: line.old_number.unwrap_or(1),
                        text: line.text,
                        kind: Kind::Delete,
                    }),
                    new: None,
                };
                let mut candidate = index + 1;
                while candidate < lines.len() {
                    match lines[candidate].kind {
                        Kind::Insert => {
                            let insert = lines.remove(candidate);
                            row.new = Some(Cell {
                                number: insert.new_number.unwrap_or(1),
                                text: insert.text,
                                kind: Kind::Insert,
                            });
                            break;
                        }
                        Kind::Equal => break,
                        Kind::Delete => candidate += 1,
                    }
                }
                rows.push(row);
            }
        }
        index += 1;
    }

    rows
}
