//! Renders a static side-by-side diff with ANSI colours.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Span,
};
use ratatui_glamour::{
    ansi,
    widgets::diff::{Highlighter, Model, Styles},
};

const BEFORE: &str = "\
fn main() {
    let name = \"world\";
    println!(\"hello {name}\");
}

fn helper() -> usize {
    1
}

fn footer() {
    println!(\"done\");
}
";

const AFTER: &str = "\
fn main() {
    let name = \"Roberto\";
    println!(\"hello {name}\");
}

fn helper() -> usize {
    2
}

fn footer() {
    println!(\"ready\");
}
";

fn main() {
    render_diff(64, "Auto · 64 columns · single-column stacked");
    println!();
    render_diff(100, "Auto · 100 columns · two-column split");
}

fn render_diff(width: usize, label: &str) {
    let diff = Model::from_contents(BEFORE, AFTER)
        .file_name("src/example.rs")
        .width(width)
        .styles(Styles::dark())
        .highlighter(Highlighter::new(highlight_rust));

    let lines = diff.view();
    let height = lines.len() as u16;
    if height == 0 {
        return;
    }

    println!("{label}");
    let area = Rect::new(0, 0, width as u16, height);
    let mut buffer = Buffer::empty(area);
    diff.render(area, &mut buffer);
    print!("{}", ansi::render_buffer(&buffer, area));
}

fn highlight_rust(line: &str, base: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut word = String::new();
    for character in line.chars() {
        if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            push_word(&mut spans, &mut word, base);
            spans.push(Span::styled(character.to_string(), base));
        }
    }
    push_word(&mut spans, &mut word, base);
    spans
}

fn push_word(spans: &mut Vec<Span<'static>>, word: &mut String, base: Style) {
    if word.is_empty() {
        return;
    }
    let style = if matches!(
        word.as_str(),
        "fn" | "let"
            | "use"
            | "pub"
            | "struct"
            | "impl"
            | "for"
            | "in"
            | "return"
            | "mod"
            | "self"
            | "Self"
            | "mut"
            | "match"
            | "if"
            | "else"
    ) {
        base.fg(Color::Magenta)
    } else if word.chars().next().is_some_and(char::is_uppercase) {
        base.fg(Color::Cyan)
    } else {
        base
    };
    spans.push(Span::styled(std::mem::take(word), style));
}
