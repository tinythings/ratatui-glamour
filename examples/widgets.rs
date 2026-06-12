use std::io;

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_glamour::widgets::{
    filepicker::Model as FilePicker,
    list::{DefaultDelegate, DefaultListItem, Model as ListModel},
    progress::Model as Progress,
    spinner,
    textinput::Model as TextInput,
    timer::Model as Timer,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut spin = spinner::Model::new();
    spin.spinner = spinner::Spinner::mini_dot();
    let mut progress = Progress::new([]);
    progress.set_width(24);
    progress.set_percent(0.42);
    let mut input = TextInput::new();
    input.focus();
    input.placeholder = "type here".into();
    let timer = Timer::new(std::time::Duration::from_secs(90), []);

    let items = vec![
        DefaultListItem {
            title: "alpha".into(),
            description: "first item".into(),
            filter_value: "alpha".into(),
        },
        DefaultListItem {
            title: "beta".into(),
            description: "second item".into(),
            filter_value: "beta".into(),
        },
    ];
    let list = ListModel::new(items, DefaultDelegate::new(), 40, 8);

    let mut picker = FilePicker::new();
    let _ = picker.read_dir();

    loop {
        terminal.draw(|frame| render(frame, &spin, &progress, &input, &timer, &list, &picker))?;
        if event::poll(std::time::Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
            && (key.code == KeyCode::Char('q') || key.code == KeyCode::Esc)
        {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn render(
    frame: &mut Frame,
    spin: &spinner::Model,
    progress: &Progress,
    input: &TextInput,
    timer: &Timer,
    list: &ListModel<DefaultListItem, DefaultDelegate>,
    picker: &FilePicker,
) {
    let area = frame.area();
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(10),
        ])
        .split(cols[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(8)])
        .split(cols[1]);

    block(frame, left[0], "Spinner", Paragraph::new(spin.view()));
    block(frame, left[1], "Progress", Paragraph::new(progress.view()));
    block(frame, left[2], "Timer", Paragraph::new(timer.view()));
    block(frame, left[3], "List", Paragraph::new(list.view()));
    block(frame, right[0], "TextInput", Paragraph::new(input.view()));
    block(frame, right[1], "FilePicker", Paragraph::new(picker.view()));
}

fn block<W: ratatui::widgets::Widget>(frame: &mut Frame, area: Rect, title: &str, widget: W) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(widget, inner);
}
