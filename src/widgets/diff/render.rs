use ratatui::style::Style;
use ratatui::text::{Line, Span};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::parse::{Cell, Hunk, Kind, ParsedDiff, RawLine};
use super::{Layout, Model};

pub(super) fn natural_width(parsed: &ParsedDiff, model: &Model) -> usize {
    if model.layout == Layout::Auto {
        let split = natural_width_for(parsed, model, Layout::Split);
        if split >= super::AUTO_SPLIT_MIN_WIDTH {
            return split;
        }
        return natural_width_for(parsed, model, Layout::Stacked);
    }
    natural_width_for(parsed, model, model.layout)
}

fn natural_width_for(parsed: &ParsedDiff, model: &Model, layout: Layout) -> usize {
    let number_width = number_width(parsed);
    let mut width = 0;

    if let Some(name) = model.file_name.as_deref().filter(|name| !name.is_empty()) {
        width = width.max(number_width + 1 + 2 + UnicodeWidthStr::width(name));
    }

    for hunk in &parsed.hunks {
        width = width.max(number_width + 1 + UnicodeWidthStr::width(hunk.header.as_str()));
        match layout {
            Layout::Split => {
                let old = hunk
                    .rows
                    .iter()
                    .filter_map(|row| row.old.as_ref())
                    .map(|cell| UnicodeWidthStr::width(cell.text.as_str()))
                    .max()
                    .unwrap_or(0);
                let new = hunk
                    .rows
                    .iter()
                    .filter_map(|row| row.new.as_ref())
                    .map(|cell| UnicodeWidthStr::width(cell.text.as_str()))
                    .max()
                    .unwrap_or(0);
                width = width.max(2 * number_width + 5 + old + new);
            }
            Layout::Stacked => {
                let code = hunk
                    .rows
                    .iter()
                    .flat_map(|row| [row.old.as_ref(), row.new.as_ref()])
                    .flatten()
                    .map(|cell| UnicodeWidthStr::width(cell.text.as_str()))
                    .max()
                    .unwrap_or(0);
                let fixed = if model.line_numbers {
                    number_width + 3
                } else {
                    2
                };
                width = width.max(fixed + code);
            }
            Layout::Unified => {
                let code = hunk
                    .lines
                    .iter()
                    .map(|line| UnicodeWidthStr::width(line.text.as_str()) + 1)
                    .max()
                    .unwrap_or(0);
                let fixed = if model.line_numbers {
                    number_width + 3
                } else {
                    2
                };
                width = width.max(fixed + code);
            }
            Layout::Auto => {}
        }
    }

    width
}

pub(super) fn effective_layout(layout: Layout, width: usize) -> Layout {
    match layout {
        Layout::Auto if width >= super::AUTO_SPLIT_MIN_WIDTH => Layout::Split,
        Layout::Auto => Layout::Stacked,
        explicit => explicit,
    }
}

pub(super) fn render(parsed: &ParsedDiff, model: &Model, width: usize) -> Vec<Line<'static>> {
    if width == 0 {
        return Vec::new();
    }

    let number_width = number_width(parsed);
    let layout = effective_layout(model.layout, width);
    let mut lines = Vec::new();

    if let Some(name) = model.file_name.as_deref().filter(|name| !name.is_empty()) {
        lines.push(file_header_line(name, model, width, number_width));
    }

    for hunk in &parsed.hunks {
        lines.push(hunk_line(hunk, model, width, number_width));
        match layout {
            Layout::Split => lines.extend(
                hunk.rows
                    .iter()
                    .map(|row| split_row_line(row, model, width, number_width)),
            ),
            Layout::Stacked => lines.extend(
                hunk.rows
                    .iter()
                    .flat_map(|row| stacked_row_lines(row, model, width, number_width)),
            ),
            Layout::Unified => lines.extend(
                hunk.lines
                    .iter()
                    .map(|line| unified_line(line, model, width, number_width)),
            ),
            Layout::Auto => {}
        }
    }

    lines
}

fn number_width(parsed: &ParsedDiff) -> usize {
    parsed
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .flat_map(|line| [line.old_number, line.new_number])
        .flatten()
        .max()
        .map(|number| number.to_string().len())
        .unwrap_or(1)
        .max(1)
}

fn file_header_line(name: &str, model: &Model, width: usize, number_width: usize) -> Line<'static> {
    let style = model.styles.filename;
    let mut spans = gutter_prefix(model, number_width, style);
    let remaining = width.saturating_sub(prefix_width(model, number_width));
    spans.push(Span::styled(
        fit_padded(&format!("  {name}"), remaining),
        style.code,
    ));
    Line::from(spans)
}

fn hunk_line(hunk: &Hunk, model: &Model, width: usize, number_width: usize) -> Line<'static> {
    let style = model.styles.divider;
    let mut spans = if model.line_numbers {
        vec![
            Span::styled(format!("{:>number_width$}", "…"), style.line_number),
            Span::raw(" "),
        ]
    } else {
        Vec::new()
    };
    let remaining = width.saturating_sub(prefix_width(model, number_width));
    spans.push(Span::styled(
        fit_padded(&hunk.header, remaining),
        style.code,
    ));
    Line::from(spans)
}

fn split_row_line(
    row: &super::parse::SplitRow,
    model: &Model,
    width: usize,
    number_width: usize,
) -> Line<'static> {
    let fixed = 2 * number_width + 3;
    let code_total = width.saturating_sub(fixed);
    let old_width = code_total / 2;
    let new_width = code_total - old_width;

    let mut spans = side_spans(row.old.as_ref(), old_width, model, number_width);
    spans.push(Span::styled(" │ ", model.styles.divider.code));
    spans.extend(side_spans(row.new.as_ref(), new_width, model, number_width));
    Line::from(spans)
}

fn stacked_row_lines(
    row: &super::parse::SplitRow,
    model: &Model,
    width: usize,
    number_width: usize,
) -> Vec<Line<'static>> {
    match (&row.old, &row.new) {
        (Some(old), Some(new)) if old.kind == Kind::Equal && new.kind == Kind::Equal => {
            vec![stacked_cell_line(old, model, width, number_width)]
        }
        (old, new) => {
            let mut lines = Vec::new();
            if let Some(cell) = old {
                lines.push(stacked_cell_line(cell, model, width, number_width));
            }
            if let Some(cell) = new {
                lines.push(stacked_cell_line(cell, model, width, number_width));
            }
            lines
        }
    }
}

fn stacked_cell_line(
    cell: &Cell,
    model: &Model,
    width: usize,
    number_width: usize,
) -> Line<'static> {
    let style = match cell.kind {
        Kind::Equal => model.styles.equal,
        Kind::Insert => model.styles.insert,
        Kind::Delete => model.styles.delete,
    };
    let symbol = match cell.kind {
        Kind::Equal => " ",
        Kind::Insert => "+",
        Kind::Delete => "-",
    };
    let fixed = if model.line_numbers {
        number_width + 3
    } else {
        2
    };
    let code_width = width.saturating_sub(fixed);
    let mut spans = Vec::new();
    if model.line_numbers {
        spans.push(Span::styled(
            format!("{:>number_width$}", cell.number),
            style.line_number,
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(symbol.to_string(), style.symbol));
    spans.push(Span::raw(" "));
    spans.extend(code_spans(model, &cell.text, code_width, style.code));
    Line::from(spans)
}

fn side_spans(
    cell: Option<&Cell>,
    code_width: usize,
    model: &Model,
    number_width: usize,
) -> Vec<Span<'static>> {
    let Some(cell) = cell else {
        let missing = model.styles.missing;
        return vec![
            Span::styled(" ".repeat(number_width), missing.line_number),
            Span::styled(" ".repeat(code_width), missing.code),
        ];
    };
    let style = match cell.kind {
        Kind::Equal => model.styles.equal,
        Kind::Insert => model.styles.insert,
        Kind::Delete => model.styles.delete,
    };
    let symbol = match cell.kind {
        Kind::Equal => " ",
        Kind::Insert => "+",
        Kind::Delete => "-",
    };
    let mut spans = vec![
        Span::styled(format!("{:<number_width$}", cell.number), style.line_number),
        Span::styled(symbol.to_string(), style.symbol),
    ];
    spans.extend(code_spans(
        model,
        &cell.text,
        code_width.saturating_sub(1),
        style.code,
    ));
    spans
}

fn unified_line(line: &RawLine, model: &Model, width: usize, number_width: usize) -> Line<'static> {
    let style = match line.kind {
        Kind::Equal => model.styles.equal,
        Kind::Insert => model.styles.insert,
        Kind::Delete => model.styles.delete,
    };
    let number = match line.kind {
        Kind::Insert => line.new_number,
        Kind::Equal | Kind::Delete => line.old_number,
    };
    let symbol = match line.kind {
        Kind::Equal => " ",
        Kind::Insert => "+",
        Kind::Delete => "-",
    };
    let fixed = if model.line_numbers {
        number_width + 3
    } else {
        2
    };
    let code_width = width.saturating_sub(fixed);
    let mut spans = Vec::new();
    if model.line_numbers {
        spans.push(Span::styled(
            gutter_text(number, number_width),
            style.line_number,
        ));
        spans.push(Span::raw(" "));
    }
    spans.push(Span::styled(symbol.to_string(), style.symbol));
    spans.push(Span::raw(" "));
    spans.extend(code_spans(model, &line.text, code_width, style.code));
    Line::from(spans)
}

fn gutter_prefix(
    model: &Model,
    number_width: usize,
    style: super::LineStyle,
) -> Vec<Span<'static>> {
    if !model.line_numbers {
        return Vec::new();
    }
    vec![
        Span::styled(format!("{:>number_width$}", "…"), style.line_number),
        Span::raw(" "),
    ]
}

fn prefix_width(model: &Model, number_width: usize) -> usize {
    if model.line_numbers {
        number_width + 1
    } else {
        0
    }
}

fn gutter_text(number: Option<usize>, width: usize) -> String {
    number
        .map(|number| format!("{number:>width$}"))
        .unwrap_or_else(|| " ".repeat(width))
}

fn code_spans(model: &Model, text: &str, width: usize, base: Style) -> Vec<Span<'static>> {
    if width == 0 {
        return Vec::new();
    }
    fit_spans(model.highlight(text, base), width, base)
}

fn fit_spans(spans: Vec<Span<'static>>, width: usize, base: Style) -> Vec<Span<'static>> {
    let mut cells = spans
        .into_iter()
        .flat_map(|span| {
            let style = span.style;
            span.content
                .chars()
                .map(|ch| (ch, style))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut used = 0usize;
    let mut keep = 0usize;
    for (ch, _) in &cells {
        let char_width = UnicodeWidthChar::width(*ch).unwrap_or(0);
        if used + char_width > width {
            break;
        }
        used += char_width;
        keep += 1;
    }

    if keep < cells.len() {
        while keep > 0 && used + 1 > width {
            let char_width = UnicodeWidthChar::width(cells[keep - 1].0).unwrap_or(0);
            used = used.saturating_sub(char_width);
            keep -= 1;
        }
        cells.truncate(keep);
        if width > 0 {
            cells.push(('…', base));
            used += 1;
        }
    }

    while used < width {
        cells.push((' ', base));
        used += 1;
    }

    merge_cells(cells)
}

fn merge_cells(cells: Vec<(char, Style)>) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut content = String::new();
    let mut current: Option<Style> = None;
    for (ch, style) in cells {
        if let Some(current_style) = current
            && current_style != style
        {
            spans.push(Span::styled(std::mem::take(&mut content), current_style));
        }
        current = Some(style);
        content.push(ch);
    }
    if let Some(style) = current {
        spans.push(Span::styled(content, style));
    }
    spans
}

fn fit_padded(text: &str, width: usize) -> String {
    let text = truncate_cells(text, width);
    let used = UnicodeWidthStr::width(text.as_str());
    if used < width {
        format!("{text}{}", " ".repeat(width - used))
    } else {
        text
    }
}

fn truncate_cells(text: &str, width: usize) -> String {
    if UnicodeWidthStr::width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let target = width.saturating_sub(1);
    let mut used = 0;
    let mut truncated = String::new();
    for character in text.chars() {
        let character_width = UnicodeWidthChar::width(character).unwrap_or(0);
        if used + character_width > target {
            break;
        }
        truncated.push(character);
        used += character_width;
    }
    truncated.push('…');
    truncated
}
