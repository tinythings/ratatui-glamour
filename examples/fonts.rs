//! Prints every printable ASCII glyph for both big-text fonts in labelled grids.

use ratatui_glamour::widgets::bigtext::Font;

const COLUMNS: usize = 8;

fn label(character: char) -> String {
    if character == ' ' {
        "space".to_string()
    } else {
        character.to_string()
    }
}

fn print_group(font: Font, title: &str, text: &str) {
    let glyphs = text
        .chars()
        .map(|character| (label(character), font.rows(&character.to_string())))
        .collect::<Vec<_>>();
    if glyphs.is_empty() {
        return;
    }

    println!("{title}");

    let cell_width = glyphs
        .iter()
        .flat_map(|(label, rows)| {
            std::iter::once(label.chars().count()).chain(rows.iter().map(|row| row.chars().count()))
        })
        .max()
        .unwrap_or(1)
        + 3;

    for chunk in glyphs.chunks(COLUMNS) {
        let labels = chunk
            .iter()
            .map(|(label, _)| format!("{label:<cell_width$}"))
            .collect::<String>();
        println!("{}", labels.trim_end());

        for row_index in 0..font.height() {
            let row = chunk
                .iter()
                .map(|(_, rows)| format!("{:<cell_width$}", rows[row_index]))
                .collect::<String>();
            println!("{}", row.trim_end());
        }
        println!();
    }
}

fn print_font(font: Font) {
    println!("================ {font:?} ================\n");

    print_group(font, "Space", " ");
    print_group(font, "Uppercase", "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
    print_group(font, "Lowercase", "abcdefghijklmnopqrstuvwxyz");
    print_group(font, "Digits", "0123456789");
    print_group(font, "Symbols", "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~");

    println!("Wordmark");
    for row in font.rows("ROBERTO") {
        println!("{row}");
    }
    println!();
}

fn main() {
    for font in [Font::Sans, Font::Serif] {
        print_font(font);
    }
}
