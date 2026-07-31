use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use ratatui_glamour::widgets::{
    filepicker::Model as FilePicker,
    list::{DefaultDelegate, DefaultListItem, Model as ListModel},
    passwordinput::Model as PasswordInput,
    progress::Model as Progress,
    spinner,
    textinput::Model as TextInput,
    timer::Model as Timer,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Input,
    Password,
    List,
    Picker,
}

impl Focus {
    fn next(self) -> Self {
        match self {
            Self::Input => Self::Password,
            Self::Password => Self::List,
            Self::List => Self::Picker,
            Self::Picker => Self::Input,
        }
    }
}

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
    let mut password = PasswordInput::new();
    password.set_placeholder("password");
    let mut timer = Timer::new(Duration::from_secs(90), []);

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
    let mut list = ListModel::new(items, DefaultDelegate::new(), 40, 8);

    let mut picker = FilePicker::new();
    let _ = picker.read_dir();

    let mut focus = Focus::Input;

    loop {
        spin.update(spin.tick());
        timer.update_tick(timer.tick_msg());
        progress.update(progress.frame_msg());

        terminal.draw(|frame| {
            render(
                frame,
                &spin,
                &progress,
                &input,
                &password,
                &timer,
                &list,
                &picker,
                focus,
            )
        })?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if key.code == KeyCode::Char('q')
                        || (key.code == KeyCode::Esc
                            && !key.modifiers.contains(KeyModifiers::SHIFT))
                    {
                        break;
                    }
                    if key.code == KeyCode::Tab {
                        match focus {
                            Focus::Input => input.blur(),
                            Focus::Password => password.blur(),
                            _ => {}
                        }
                        focus = focus.next();
                        match focus {
                            Focus::Input => input.focus(),
                            Focus::Password => password.focus(),
                            _ => {}
                        }
                        continue;
                    }
                    match focus {
                        Focus::Input => input.handle_key(&key),
                        Focus::Password => {
                            if key.code == KeyCode::Enter {
                                input.set_value(&format!("{} :-)", password.value()));
                                password.reset();
                            } else {
                                password.handle_key(&key);
                            }
                        }
                        Focus::List => list.handle_key(&key),
                        Focus::Picker => {
                            let _ = picker.handle_key(&key);
                        }
                    }
                }
                _ => {}
            }
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
    password: &PasswordInput,
    timer: &Timer,
    list: &ListModel<DefaultListItem, DefaultDelegate>,
    picker: &FilePicker,
    focus: Focus,
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
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(cols[0]);

    let focus_style = Style::default().fg(Color::Yellow);
    let normal_style = Style::default().fg(Color::Blue);

    let input_style = if focus == Focus::Input {
        focus_style
    } else {
        normal_style
    };
    let password_style = if focus == Focus::Password {
        focus_style
    } else {
        normal_style
    };
    let list_style = if focus == Focus::List {
        focus_style
    } else {
        normal_style
    };
    let picker_style = if focus == Focus::Picker {
        focus_style
    } else {
        normal_style
    };

    block(frame, left[0], "Spinner", Paragraph::new(spin.view()), normal_style);
    block(frame, left[1], "Progress", Paragraph::new(progress.view()), normal_style);
    block(frame, left[2], "Timer", Paragraph::new(timer.view()), normal_style);
    block(frame, left[3], "List", Paragraph::new(list.view()), list_style);
    block(
        frame,
        left[4],
        "TextInput",
        Paragraph::new(input.view()),
        input_style,
    );
    block(
        frame,
        left[5],
        "Password",
        Paragraph::new(password.view()),
        password_style,
    );
    block(
        frame,
        cols[1],
        "FilePicker",
        Paragraph::new(picker.view()),
        picker_style,
    );
}

fn block<W: ratatui::widgets::Widget>(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    widget: W,
    border_style: Style,
) {
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(widget, inner);
}
