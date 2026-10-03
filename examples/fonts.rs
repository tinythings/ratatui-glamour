//! Prints both big-text fonts over the full printable ASCII range.

use ratatui::style::Color;
use ratatui_glamour::widgets::bigtext::{Font, Model};

const STOPS: [Color; 2] = [Color::Rgb(0x6b, 0x50, 0xff), Color::Rgb(0xff, 0x60, 0xff)];

fn print(font: Font, label: &str, text: &str) {
    println!("--- {label} ---");
    for line in Model::with_font(text, font, &STOPS).view() {
        println!(
            "{}",
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        );
    }
    println!();
}

fn main() {
    let groups = [
        ("upper", "ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        ("lower", "abcdefghijklmnopqrstuvwxyz"),
        ("digits", "0123456789"),
        ("symbols", "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~"),
    ];

    for font in [Font::Serif, Font::Sans] {
        println!("================ {font:?} ================\n");
        for (label, text) in groups {
            print(font, label, text);
        }
    }
}
