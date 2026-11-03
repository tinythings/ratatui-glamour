use ratatui::{
    style::Color,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use super::*;

fn line_text(line: &Line<'_>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn text(model: &Model) -> String {
    model
        .view()
        .iter()
        .map(line_text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn split_replacement_is_aligned_on_one_row() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(60)
        .split();
    let rendered = text(&diff);
    assert!(rendered.contains("-two"), "{rendered}");
    assert!(rendered.contains("+three"), "{rendered}");
    let row = diff
        .view()
        .into_iter()
        .find(|line| line_text(line).contains("two"))
        .unwrap();
    let row = line_text(&row);
    assert!(row.contains("two") && row.contains("three"), "{row}");
}

#[test]
fn split_insert_only_leaves_the_old_side_blank() {
    let diff = Model::from_contents("one\n", "one\ntwo\n")
        .width(60)
        .split();
    let rendered = text(&diff);
    let row = rendered
        .lines()
        .find(|line| line.contains("two"))
        .expect("insert row");
    let (left, right) = row.split_once('│').expect("divider");
    assert!(left.trim().is_empty(), "{row}");
    assert!(right.contains("two"), "{row}");
}

#[test]
fn enabled_line_numbers_are_rendered_without_extra_spacing() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(60)
        .split();
    let rendered = text(&diff);
    let row = rendered
        .lines()
        .find(|line| line.contains("two") && line.contains("three"))
        .expect("replacement row");
    assert!(row.contains("2-two"), "{row}");
    assert!(row.contains("2+three"), "{row}");
}

#[test]
fn disabled_line_numbers_still_render_symbols() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(60)
        .line_numbers(false)
        .split();
    let rendered = text(&diff);
    assert!(rendered.contains("-two"), "{rendered}");
    assert!(rendered.contains("+three"), "{rendered}");
}

#[test]
fn file_name_is_rendered_above_the_diff() {
    let diff = Model::from_contents("one\n", "two\n")
        .file_name("src/file.rs")
        .width(60);
    let rendered = text(&diff);
    assert!(rendered.contains("src/file.rs"), "{rendered}");
}

#[test]
fn auto_narrow_width_uses_stacked_and_truncates() {
    let before = "one very long line that will not fit\n";
    let after = "another very long line that will not fit\n";
    let diff = Model::from_contents(before, after).width(40);
    let lines = diff.view();
    for line in &lines {
        assert!(
            UnicodeWidthStr::width(line_text(line).as_str()) <= 40,
            "{}",
            line_text(line)
        );
    }
    let rendered = text(&diff);
    assert!(!rendered.contains('│'), "{rendered}");
    assert!(rendered.contains("- one"), "{rendered}");
    assert!(rendered.contains("+ another"), "{rendered}");
    assert!(rendered.contains('…'), "{rendered}");
}

#[test]
fn explicit_split_overrides_the_auto_threshold() {
    let diff = Model::from_contents("one\n", "two\n").width(40).split();
    let rendered = text(&diff);
    assert!(rendered.contains('│'), "{rendered}");
}

#[test]
fn stacked_layout_reuses_the_split_cell_styles() {
    let styles = Styles::dark();
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(40)
        .styles(styles);
    let lines = diff.view();
    let insert = lines
        .iter()
        .find(|line| line_text(line).contains("+ three"))
        .expect("stacked insert line");
    let delete = lines
        .iter()
        .find(|line| line_text(line).contains("- two"))
        .expect("stacked delete line");
    assert!(
        insert
            .spans
            .iter()
            .any(|span| span.style == styles.insert.code)
    );
    assert!(
        delete
            .spans
            .iter()
            .any(|span| span.style == styles.delete.code)
    );
    assert!(
        insert
            .spans
            .iter()
            .any(|span| span.style == styles.insert.symbol)
    );
    assert!(
        delete
            .spans
            .iter()
            .any(|span| span.style == styles.delete.symbol)
    );
}

#[test]
fn auto_layout_uses_stacked_below_eighty_columns() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n").width(79);
    let rendered = text(&diff);
    assert!(!rendered.contains('│'), "{rendered}");
    assert!(rendered.contains("- two"), "{rendered}");
    assert!(rendered.contains("+ three"), "{rendered}");
}

#[test]
fn auto_layout_uses_split_at_eighty_columns() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n").width(80);
    let rendered = text(&diff);
    assert!(rendered.contains('│'), "{rendered}");
}

#[test]
fn explicit_unified_overrides_the_auto_threshold() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(100)
        .unified();
    let rendered = text(&diff);
    assert!(!rendered.contains('│'), "{rendered}");
}

#[test]
fn auto_view_uses_natural_width_for_layout_selection() {
    let short = Model::from_contents("a\n", "b\n");
    assert!(!text(&short).contains('│'), "{}", text(&short));
    let long = Model::from_contents(
        "one very long line that definitely exceeds the split threshold\n",
        "another very long line that definitely exceeds it\n",
    );
    assert!(text(&long).contains('│'), "{}", text(&long));
}

#[test]
fn view_limited_reports_hidden_rows() {
    let before = (1..=20)
        .map(|line| format!("line {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let after = (1..=20)
        .map(|line| format!("changed {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    let diff = Model::from_contents(before, after).width(80);
    let total = diff.view().len();
    let limited = diff.view_limited(5);
    assert_eq!(limited.lines.len(), 5);
    assert_eq!(limited.hidden, total - 5);
}

#[test]
fn unified_layout_uses_one_column() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n").unified();
    let rendered = text(&diff);
    assert!(rendered.contains("- two"), "{rendered}");
    assert!(rendered.contains("+ three"), "{rendered}");
    assert!(!rendered.contains('│'), "{rendered}");
}

#[test]
fn stats_are_counted_from_the_diff() {
    let diff = Model::from_contents("one\ntwo\nthree\n", "one\nTHREE\nfour\n");
    assert_eq!(
        diff.stats(),
        Stats {
            additions: 2,
            removals: 2
        }
    );
}

#[test]
fn tabs_are_expanded() {
    let diff = Model::from_contents("one\n\told\n", "one\n\tnew\n").width(60);
    let rendered = text(&diff);
    assert!(!rendered.contains('\t'), "{rendered}");
    assert!(rendered.contains("    old"), "{rendered}");
}

#[test]
fn highlighter_keeps_the_diff_background() {
    let diff = Model::from_contents("one\ntwo\n", "one\nthree\n")
        .width(60)
        .highlighter(Highlighter::new(|line, base| {
            let style = if line.contains("two") {
                base.fg(Color::Magenta)
            } else {
                base
            };
            vec![Span::styled(line.to_string(), style)]
        }));
    let line = diff
        .view()
        .into_iter()
        .find(|line| line_text(line).contains("two"))
        .expect("highlighted line");
    let span = line
        .spans
        .iter()
        .find(|span| span.content.contains("two"))
        .expect("highlighted span");
    assert_eq!(span.style.fg, Some(Color::Magenta));
    assert!(span.style.bg.is_some());
}

#[test]
fn highlighted_long_lines_are_truncated_to_width() {
    let diff = Model::from_contents("one very long line", "another very long line")
        .width(40)
        .highlighter(Highlighter::new(|line, base| {
            vec![Span::styled(line.to_string(), base.fg(Color::Magenta))]
        }));
    for line in &diff.view() {
        assert!(
            UnicodeWidthStr::width(line_text(line).as_str()) <= 40,
            "{}",
            line_text(line)
        );
    }
    assert!(text(&diff).contains('…'));
}

#[test]
fn insert_and_delete_styles_use_diff_colors() {
    let diff = Model::from_contents("one\n", "two\n").width(60).split();
    let insert = diff
        .view()
        .into_iter()
        .find(|line| line_text(line).contains("+two"))
        .expect("insert line");
    let delete = diff
        .view()
        .into_iter()
        .find(|line| line_text(line).contains("-one"))
        .expect("delete line");
    assert!(
        insert
            .spans
            .iter()
            .any(|span| span.style.fg == Some(Color::Rgb(0x12, 0xc7, 0x8f)))
    );
    assert!(
        delete
            .spans
            .iter()
            .any(|span| span.style.fg == Some(Color::Rgb(0xeb, 0x42, 0x68)))
    );
}
