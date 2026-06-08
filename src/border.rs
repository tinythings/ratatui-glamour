use ratatui::symbols::border;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Border {
    pub top: &'static str,
    pub bottom: &'static str,
    pub left: &'static str,
    pub right: &'static str,
    pub top_left: &'static str,
    pub top_right: &'static str,
    pub bottom_left: &'static str,
    pub bottom_right: &'static str,
    pub middle_left: &'static str,
    pub middle_right: &'static str,
    pub middle: &'static str,
    pub middle_top: &'static str,
    pub middle_bottom: &'static str,
}

impl Border {
    pub const fn new(
        top: &'static str,
        bottom: &'static str,
        left: &'static str,
        right: &'static str,
        top_left: &'static str,
        top_right: &'static str,
        bottom_left: &'static str,
        bottom_right: &'static str,
        middle_left: &'static str,
        middle_right: &'static str,
        middle: &'static str,
        middle_top: &'static str,
        middle_bottom: &'static str,
    ) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
            top_left,
            top_right,
            bottom_left,
            bottom_right,
            middle_left,
            middle_right,
            middle,
            middle_top,
            middle_bottom,
        }
    }

    pub const fn empty() -> Self {
        Self::new("", "", "", "", "", "", "", "", "", "", "", "", "")
    }

    pub const fn normal() -> Self {
        Self::new("─", "─", "│", "│", "┌", "┐", "└", "┘", "├", "┤", "┼", "┬", "┴")
    }

    pub const fn rounded() -> Self {
        Self::new("─", "─", "│", "│", "╭", "╮", "╰", "╯", "├", "┤", "┼", "┬", "┴")
    }

    pub const fn block() -> Self {
        Self::new("█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█", "█")
    }

    pub const fn outer_half_block() -> Self {
        Self::new("▀", "▄", "▌", "▐", "▛", "▜", "▙", "▟", "", "", "", "", "")
    }

    pub const fn inner_half_block() -> Self {
        Self::new("▄", "▀", "▐", "▌", "▗", "▖", "▝", "▘", "", "", "", "", "")
    }

    pub const fn thick() -> Self {
        Self::new("━", "━", "┃", "┃", "┏", "┓", "┗", "┛", "┣", "┫", "╋", "┳", "┻")
    }

    pub const fn double() -> Self {
        Self::new("═", "═", "║", "║", "╔", "╗", "╚", "╝", "╠", "╣", "╬", "╦", "╩")
    }

    pub const fn hidden() -> Self {
        Self::new(" ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ", " ")
    }

    pub const fn markdown() -> Self {
        Self::new("-", "-", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|", "|")
    }

    pub const fn ascii() -> Self {
        Self::new("-", "-", "|", "|", "+", "+", "+", "+", "+", "+", "+", "+", "+")
    }

    pub const fn into_border_set(self) -> border::Set<'static> {
        border::Set {
            top_left: self.top_left,
            top_right: self.top_right,
            bottom_left: self.bottom_left,
            bottom_right: self.bottom_right,
            vertical_left: self.left,
            vertical_right: self.right,
            horizontal_top: self.top,
            horizontal_bottom: self.bottom,
        }
    }
}
