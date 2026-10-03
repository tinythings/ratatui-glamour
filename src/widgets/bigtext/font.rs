//! Fonts for the big-text widget.
//!
//! Two fonts are available:
//!
//! - [`Font::Serif`] — the public-domain 8×8 `font8x8` bitmap, packed two rows
//!   per terminal cell.
//! - [`Font::Sans`] — a 3-row half-block font in the same visual language as
//!   the Crush wordmark (`█` verticals, `▀`/`▄` thin strokes). It covers the
//!   full printable ASCII range, so any string renders.

use font8x8::{BASIC_FONTS, UnicodeFonts};

/// Columns of the 8×8 Serif bitmap.
const SERIF_WIDTH: usize = 8;
/// Bitmap rows of the Serif glyph, packed two per terminal row.
const SERIF_BITMAP_ROWS: usize = 8;

/// The available big-text fonts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    /// 8×8 bitmap glyphs, two bitmap rows per terminal cell.
    Serif,
    /// 3-row half-block glyphs in the Crush style.
    Sans,
}

impl Font {
    /// The rendered height in terminal rows.
    pub const fn height(self) -> usize {
        match self {
            Font::Serif => SERIF_BITMAP_ROWS / 2,
            Font::Sans => 3,
        }
    }

    /// Renders `text` into `height()` rows of cell characters
    /// (`' '`, `'▀'`, `'▄'`, `'█'`). Rows may have different lengths; the
    /// caller pads to the widest row.
    pub fn rows(self, text: &str) -> Vec<String> {
        match self {
            Font::Serif => serif_rows(text),
            Font::Sans => sans_rows(text),
        }
    }
}

/// Expands an 8×8 bitmap to terminal cells with half-blocks.
fn serif_rows(text: &str) -> Vec<String> {
    let glyphs = text.chars().map(serif_bitmap).collect::<Vec<_>>();
    let mut rows = vec![String::new(); SERIF_BITMAP_ROWS / 2];
    for (index, glyph) in glyphs.iter().enumerate() {
        if index > 0 {
            for row in &mut rows {
                row.push(' ');
            }
        }
        for (row_index, row) in rows.iter_mut().enumerate() {
            let top = glyph[row_index * 2];
            let bottom = glyph[row_index * 2 + 1];
            for column in 0..SERIF_WIDTH {
                let upper = top & (1 << column) != 0;
                let lower = bottom & (1 << column) != 0;
                row.push(match (upper, lower) {
                    (true, true) => '█',
                    (true, false) => '▀',
                    (false, true) => '▄',
                    (false, false) => ' ',
                });
            }
        }
    }
    rows
}

/// The 8×8 bitmap for `character`, falling back to `?`.
fn serif_bitmap(character: char) -> [u8; SERIF_BITMAP_ROWS] {
    BASIC_FONTS
        .get(character)
        .or_else(|| BASIC_FONTS.get('?'))
        .unwrap_or([0xff; SERIF_BITMAP_ROWS])
}

/// Joins the 3-row Sans glyphs for `text`.
fn sans_rows(text: &str) -> Vec<String> {
    let mut rows = vec![String::new(); 3];
    for (index, character) in text.chars().enumerate() {
        if index > 0 {
            for row in &mut rows {
                row.push(' ');
            }
        }
        for (row, line) in sans(character).iter().enumerate() {
            rows[row].push_str(line);
        }
    }
    rows
}

/// The 3-row Sans glyph for `character`. Each row has the same width.
#[allow(clippy::too_many_lines)]
fn sans(character: char) -> [&'static str; 3] {
    match character {
        'A' => ["▄▀▀▀▄", "█▀▀▀█", "▀   ▀"],
        'B' => ["█▀▀▀▄", "█▀▀▀█", "▀▀▀▀ "],
        'C' => ["▄▀▀▀▀", "█    ", " ▀▀▀▀"],
        'D' => ["█▀▀▀▄", "█   █", "▀▀▀▀ "],
        'E' => ["█▀▀▀▀", "█▀▀▀▀", "▀▀▀▀▀"],
        'F' => ["█▀▀▀▀", "█▀▀▀ ", "▀    "],
        'G' => ["▄▀▀▀▀", "█ ▀▀█", " ▀▀▀ "],
        'H' => ["█   █", "█▀▀▀█", "▀   ▀"],
        'I' => ["█", "█", "▀"],
        'J' => ["   █", "   █", "▀▀▀ "],
        'K' => ["█   █", "█ ▀▀ ", "▀   ▀"],
        'L' => ["█    ", "█    ", "▀▀▀▀▀"],
        'M' => ["█   █", "██ ██", "▀   ▀"],
        'N' => ["█   █", "██  █", "▀   ▀"],
        'O' => ["▄▀▀▀▄", "█   █", " ▀▀▀ "],
        'P' => ["█▀▀▀▄", "█▀▀▀ ", "▀    "],
        'Q' => ["▄▀▀▀▄", "█   █", " ▀▀▀▄"],
        'R' => ["█▀▀▀▄", "█▀▀▀▄", "▀   ▀"],
        'S' => ["▄▀▀▀▀", "▀▀▀▀█", "▀▀▀▀ "],
        'T' => ["▀▀▀▀▀", "  █  ", "  ▀  "],
        'U' => ["█   █", "█   █", " ▀▀▀ "],
        'V' => ["█   █", "█   █", " ▀ ▀ "],
        'W' => ["█   █", "██ ██", " ▀ ▀ "],
        'X' => ["█   █", " ▀▀▀ ", "▀   ▀"],
        'Y' => ["█   █", " ▀▄▀ ", "  ▀  "],
        'Z' => ["▀▀▀▀▀", "  ▀▀ ", "▀▀▀▀▀"],
        'a' => ["    ", "▄▀▀▄", " ▀▀█"],
        'b' => ["█   ", "█▀▀▄", "▀▀▀ "],
        'c' => ["    ", "▄▀▀▀", " ▀▀▀"],
        'd' => ["   █", "▄▀▀█", " ▀▀ "],
        'e' => ["    ", "▄▀▀▄", " ▀▀ "],
        'f' => [" ▀▀", "█▀ ", "▀  "],
        'g' => ["    ", "▄▀▀█", " ▀▀█"],
        'h' => ["█   ", "█▀▀▄", "▀  ▀"],
        'i' => ["▀", "█", "▀"],
        'j' => [" ▀", " █", "█ "],
        'k' => ["█   ", "█ ▀█", "▀  ▀"],
        'l' => ["█", "█", "▀"],
        'm' => ["     ", "██▀██", "▀ ▀ ▀"],
        'n' => ["    ", "█▀▀▄", "▀  ▀"],
        'o' => ["    ", "▄▀▀▄", " ▀▀ "],
        'p' => ["    ", "█▀▀▄", "▀▀▀ "],
        'q' => ["    ", "▄▀▀█", " ▀▀ "],
        'r' => ["    ", "█▀▀ ", "▀   "],
        's' => ["    ", "▄▀▀▀", "▀▀▀ "],
        't' => ["█   ", "█▀▀ ", " ▀▀ "],
        'u' => ["    ", "█  █", "▀▀▀ "],
        'v' => ["    ", "█  █", " ▀▀ "],
        'w' => ["     ", "█ █ █", " ▀ ▀ "],
        'x' => ["    ", "█  █", "▀▀▀ "],
        'y' => ["    ", "█  █", "▀▀▀█"],
        'z' => ["    ", "▀▀▀▀", "▀▀▀▀"],
        '0' => ["▄▀▀▀▄", "█ █ █", " ▀▀▀ "],
        '1' => ["  █  ", "  █  ", "  ▀  "],
        '2' => ["▀▀▀▀▄", " ▀▀▀ ", "▀▀▀▀▀"],
        '3' => ["▀▀▀▀▄", " ▀▀▀█", "▀▀▀▀ "],
        '4' => ["█   █", "▀▀▀▀█", "    ▀"],
        '5' => ["▀▀▀▀▀", "▀▀▀▀▄", "▀▀▀▀ "],
        '6' => ["▄▀▀▀▀", "█▀▀▀▄", "▀▀▀▀ "],
        '7' => ["▀▀▀▀▀", "   ▀ ", "   ▀ "],
        '8' => ["▄▀▀▀▄", "█▀▀▀█", "▀▀▀▀ "],
        '9' => ["▄▀▀▀▄", "▀▀▀▀█", "▀▀▀▀ "],
        ' ' => [" ", " ", " "],
        '!' => ["█", "█", "▀"],
        '"' => ["█ █", "   ", "   "],
        '#' => ["█ █", "▀▀▀", "█ █"],
        '$' => ["▄▀▀", "▀▀█", "▀▀▀"],
        '%' => ["█  ▄", " ▄▀ ", "▀  █"],
        '&' => ["▄▀▄", "█▀█", "▀ ▀"],
        '\'' => ["█", " ", " "],
        '(' => [" █", "█ ", " ▀"],
        ')' => ["█ ", " █", "▀ "],
        '*' => ["█ █", " ▀ ", "█ █"],
        '+' => ["   ", "▀▀▀", " ▀ "],
        ',' => [" ", " ", "▀"],
        '-' => ["   ", "   ", "▀▀▀"],
        '.' => [" ", " ", "▀"],
        '/' => ["  █", " ▀ ", "▀  "],
        ':' => [" ", "▀", "▀"],
        ';' => [" ", "▀", "▀"],
        '<' => ["  █", "▀  ", "  ▀"],
        '=' => ["   ", "▀▀▀", "▀▀▀"],
        '>' => ["█  ", "  ▀", "█  "],
        '?' => ["▀▀▀", "  █", "  ▀"],
        '@' => ["▄▀▀▄", "█▀▀█", "▀▀▀ "],
        '[' => ["▄█", "█ ", "▀▀"],
        '\\' => ["█  ", " ▀ ", "  ▀"],
        ']' => ["█▄", " █", "▀▀"],
        '^' => [" █ ", "▀ ▀", "   "],
        '_' => ["   ", "   ", "▀▀▀"],
        '`' => ["▀", " ", " "],
        '{' => [" ▄", "█ ", " ▀"],
        '|' => ["█", "█", "█"],
        '}' => ["▄ ", " █", "▀ "],
        '~' => ["▄ ▄", " ▀ ", "   "],
        _ => ["███", "███", "███"],
    }
}
