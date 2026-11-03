use ratatui::style::{Color, Style};

/// Styles for one side of a diff line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineStyle {
    /// Gutter containing the line number.
    pub line_number: Style,
    /// Leading ` `/`+`/`-` marker.
    pub symbol: Style,
    /// The line content.
    pub code: Style,
}

/// Complete visual theme for a diff view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Styles {
    /// File name header.
    pub filename: LineStyle,
    /// Hunk separator and hunk header.
    pub divider: LineStyle,
    /// Blank side of a split row.
    pub missing: LineStyle,
    /// Unchanged context line.
    pub equal: LineStyle,
    /// Inserted line.
    pub insert: LineStyle,
    /// Deleted line.
    pub delete: LineStyle,
}

impl Default for Styles {
    fn default() -> Self {
        Self::dark()
    }
}

impl Styles {
    /// Dark theme matching Charmtone's default diff palette.
    pub fn dark() -> Self {
        let green = Color::Rgb(0x12, 0xc7, 0x8f);
        let red = Color::Rgb(0xeb, 0x42, 0x68);
        let subtle = Color::Rgb(0x85, 0x83, 0x92);
        let text = Color::Rgb(0xec, 0xeb, 0xf0);
        let panel = Color::Rgb(0x20, 0x1f, 0x26);
        let surface = Color::Rgb(0x3a, 0x39, 0x43);
        let insert_bg = Color::Rgb(0x30, 0x3a, 0x30);
        let insert_gutter = Color::Rgb(0x29, 0x32, 0x29);
        let delete_bg = Color::Rgb(0x3a, 0x30, 0x30);
        let delete_gutter = Color::Rgb(0x33, 0x29, 0x29);

        Self {
            filename: LineStyle {
                line_number: Style::default().fg(subtle).bg(panel),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(subtle).bg(panel),
            },
            divider: LineStyle {
                line_number: Style::default().fg(subtle).bg(panel),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(subtle).bg(panel),
            },
            missing: LineStyle {
                line_number: Style::default().bg(surface),
                symbol: Style::default().bg(surface),
                code: Style::default().bg(surface),
            },
            equal: LineStyle {
                line_number: Style::default().fg(subtle).bg(surface),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(text).bg(panel),
            },
            insert: LineStyle {
                line_number: Style::default().fg(green).bg(insert_gutter),
                symbol: Style::default().fg(green).bg(insert_bg),
                code: Style::default().fg(text).bg(insert_bg),
            },
            delete: LineStyle {
                line_number: Style::default().fg(red).bg(delete_gutter),
                symbol: Style::default().fg(red).bg(delete_bg),
                code: Style::default().fg(text).bg(delete_bg),
            },
        }
    }

    /// Light theme for terminals with a bright background.
    pub fn light() -> Self {
        let green = Color::Rgb(0x1b, 0x7f, 0x43);
        let red = Color::Rgb(0xb7, 0x1c, 0x1c);
        let text = Color::Rgb(0x20, 0x1f, 0x26);
        let subtle = Color::Rgb(0x60, 0x5f, 0x6b);
        let panel = Color::Rgb(0xf7, 0xf6, 0xfb);
        let surface = Color::Rgb(0xd8, 0xd8, 0xe0);
        let insert_bg = Color::Rgb(0xe8, 0xf5, 0xe9);
        let insert_gutter = Color::Rgb(0xc8, 0xe6, 0xc9);
        let delete_bg = Color::Rgb(0xff, 0xeb, 0xee);
        let delete_gutter = Color::Rgb(0xff, 0xcd, 0xd2);

        Self {
            filename: LineStyle {
                line_number: Style::default().fg(subtle).bg(panel),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(subtle).bg(panel),
            },
            divider: LineStyle {
                line_number: Style::default().fg(subtle).bg(panel),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(subtle).bg(panel),
            },
            missing: LineStyle {
                line_number: Style::default().bg(surface),
                symbol: Style::default().bg(surface),
                code: Style::default().bg(surface),
            },
            equal: LineStyle {
                line_number: Style::default().fg(text).bg(panel),
                symbol: Style::default().fg(subtle).bg(panel),
                code: Style::default().fg(text).bg(panel),
            },
            insert: LineStyle {
                line_number: Style::default().fg(green).bg(insert_gutter),
                symbol: Style::default().fg(green).bg(insert_bg),
                code: Style::default().fg(text).bg(insert_bg),
            },
            delete: LineStyle {
                line_number: Style::default().fg(red).bg(delete_gutter),
                symbol: Style::default().fg(red).bg(delete_bg),
                code: Style::default().fg(text).bg(delete_bg),
            },
        }
    }
}
