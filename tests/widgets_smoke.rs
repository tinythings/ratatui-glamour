use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
};
use std::time::Duration;
use std::{fs, path::PathBuf};

use ratatui_glamour::widgets::{
    cursor::Model as Cursor,
    filepicker::Model as FilePicker,
    help, key,
    list::{FilterState, Item as ListItem, ItemDelegate, Model as ListModel, RenderContext},
    paginator::{Model as Paginator, Type},
    progress::Model as Progress,
    stopwatch::Model as Stopwatch,
    textarea::Model as TextArea,
    textinput::Model as TextInput,
    timer::Model as Timer,
    viewport::Model as Viewport,
};

#[test]
fn key_binding_matches_ctrl_combo() {
    let binding = key::Binding::new([key::with_keys(&["ctrl+a"])]);
    let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert!(key::matches(&event, [&binding]));
}

#[test]
fn key_binding_enable_disable_unbind() {
    let mut binding = key::Binding::new([
        key::with_keys(&["k", "up"]),
        key::with_help("↑/k", "move up"),
    ]);
    assert!(binding.enabled());
    binding.set_enabled(false);
    assert!(!binding.enabled());
    binding.set_enabled(true);
    binding.unbind();
    assert!(!binding.enabled());
}

#[test]
fn paginator_arabic_view_formats_both_numbers() {
    let mut paginator = Paginator::new([]);
    paginator.r#type = Type::Arabic;
    paginator.page = 2;
    paginator.total_pages = 7;
    assert_eq!(paginator.view(), "3/7");
}

#[test]
fn textinput_edits_and_suggests() {
    let mut input = TextInput::new();
    input.focus();
    input.show_suggestions = true;
    input.set_suggestions(&["test1".to_string(), "test2".to_string()]);
    input.handle_key(&KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    assert_eq!(input.current_suggestion(), "test1");
    input.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(input.current_suggestion(), "test2");
}

#[test]
fn textinput_blur_hides_suggestion_suffix() {
    let mut input = TextInput::new();
    input.focus();
    input.show_suggestions = true;
    input.set_suggestions(&["test1".to_string(), "test2".to_string()]);
    input.set_value("test");
    input.blur();
    assert!(!input.view().to_string().ends_with("test1"));
}

#[test]
fn textinput_keeps_emoji_value() {
    let mut input = TextInput::new();
    input.focus();
    input.set_width(4);
    input.set_value("🧋🧋🧋");
    assert_eq!(input.value(), "🧋🧋🧋");
    assert_eq!(input.position(), 3);
}

#[test]
fn textinput_word_delete_backward() {
    let mut input = TextInput::new();
    input.focus();
    input.set_value("foo bar baz");
    input.handle_key(&KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT));
    assert_eq!(input.value(), "foo bar ");
}

#[test]
fn textinput_default_dark_styles_match_go_defaults() {
    let input = TextInput::new();
    assert_eq!(
        input.styles().focused.placeholder.fg,
        Some(Color::Indexed(240))
    );
    assert_eq!(
        input.styles().focused.suggestion.fg,
        Some(Color::Indexed(240))
    );
    assert_eq!(input.styles().focused.prompt.fg, Some(Color::Indexed(7)));
    assert_eq!(input.styles().blurred.text.fg, Some(Color::Indexed(245)));
    assert_eq!(input.styles().cursor.style.fg, Some(Color::Indexed(7)));
}

#[test]
fn textinput_focused_placeholder_uses_cursor_cell() {
    let mut input = TextInput::new();
    input.focus();
    input.placeholder = "hello".into();
    input.set_width(5);
    let view = input.view();
    assert_eq!(view.to_string(), "> hello");
    assert!(
        view.spans[1]
            .style
            .add_modifier
            .contains(Modifier::REVERSED)
    );
}

#[test]
fn textinput_blurred_placeholder_does_not_use_cursor_cell() {
    let mut input = TextInput::new();
    input.placeholder = "hello".into();
    input.set_width(5);
    let view = input.view();
    assert_eq!(view.to_string(), "> hello");
    assert_eq!(view.spans[1].style.bg, None);
}

#[test]
fn textinput_enter_does_not_insert_space() {
    let mut input = TextInput::new();
    input.focus();
    input.set_value("test");
    input.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(input.value(), "test");
}

#[test]
fn textinput_end_of_line_suggestion_cursor_uses_suggestion_style() {
    let mut input = TextInput::new();
    input.focus();
    input.show_suggestions = true;
    input.set_suggestions(&["test1".to_string()]);
    input.handle_key(&KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
    input.handle_key(&KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE));
    let view = input.view();
    assert_eq!(view.to_string(), "> test1");
    assert!(
        view.spans[2]
            .style
            .add_modifier
            .contains(Modifier::REVERSED)
    );
}

#[test]
fn textarea_inserts_multiple_lines() {
    let mut area = TextArea::new();
    area.focus();
    area.insert_string("Foo\nBar");
    assert_eq!(area.value(), "Foo\nBar");
    assert_eq!(area.line(), 1);
    assert_eq!(area.column(), 3);
}

#[test]
fn textarea_set_value_resets_and_moves_cursor() {
    let mut area = TextArea::new();
    area.set_value("Foo\nBar\nBaz");
    assert_eq!(area.value(), "Foo\nBar\nBaz");
    assert_eq!(area.line(), 2);
    assert_eq!(area.column(), 3);
    area.set_value("Test");
    assert_eq!(area.value(), "Test");
    assert_eq!(area.line(), 0);
}

#[test]
fn textarea_ctrl_t_transposes_left() {
    let mut area = TextArea::new();
    area.focus();
    area.set_value("abcd");
    area.handle_key(&KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL));
    assert_eq!(area.value(), "abdc");
}

#[test]
fn textarea_alt_word_case_commands_match_go_shape() {
    let mut area = TextArea::new();
    area.focus();
    area.set_value("hello world");
    area.move_to_begin();
    area.handle_key(&KeyEvent::new(KeyCode::Char('u'), KeyModifiers::ALT));
    assert_eq!(area.value(), "HELLO world");
    area.move_to_begin();
    area.handle_key(&KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT));
    assert_eq!(area.value(), "hello world");
    area.move_to_begin();
    area.handle_key(&KeyEvent::new(KeyCode::Char('c'), KeyModifiers::ALT));
    assert_eq!(area.value(), "Hello world");
}

#[test]
fn textarea_ctrl_u_at_line_start_merges_previous_line() {
    let mut area = TextArea::new();
    area.focus();
    area.set_value("foo\nbar");
    area.move_to_end();
    area.cursor_start();
    area.handle_key(&KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(area.value(), "foobar");
    assert_eq!(area.line(), 0);
}

#[test]
fn textarea_vertical_navigation_keeps_visual_column_for_wide_chars() {
    let mut area = TextArea::new();
    area.focus();
    area.set_width(20);
    area.set_value("你好你好\nHello");
    area.move_to_begin();
    area.set_cursor_column(2);

    let info = area.line_info();
    assert_eq!(info.char_offset, 4);
    assert_eq!(info.column_offset, 2);

    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let info = area.line_info();
    assert_eq!(info.char_offset, 4);
    assert_eq!(info.column_offset, 4);
}

#[test]
fn textarea_vertical_navigation_remembers_horizontal_position() {
    let mut area = TextArea::new();
    area.focus();
    area.set_width(40);
    area.set_value("Hello\nWorld\nThis is a long line.");

    assert_eq!(area.line(), 2);
    assert_eq!(area.column(), 20);

    area.handle_key(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(area.line(), 1);
    assert_eq!(area.column(), 5);

    area.handle_key(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    assert_eq!(area.line(), 0);
    assert_eq!(area.column(), 5);

    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(area.line(), 2);
    assert_eq!(area.column(), 20);

    area.handle_key(&KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
    area.handle_key(&KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    assert_eq!(area.line(), 2);
    assert_eq!(area.column(), 4);
}

#[test]
fn textarea_page_up_and_down_match_go_behavior() {
    let mut area = TextArea::new();
    area.focus();
    area.show_line_numbers = true;
    area.set_height(3);
    area.set_width(20);
    let lines = (1..=10)
        .map(|n| format!("Line {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    area.set_value(&lines);

    area.move_to_begin();
    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    area.viewport.set_y_offset(3);
    area.handle_key(&KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(area.line(), 5);

    area.move_to_begin();
    for _ in 0..5 {
        area.handle_key(&KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    }
    area.viewport.set_y_offset(5);
    area.handle_key(&KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_eq!(area.line(), 2);
}

struct SimpleItem(&'static str);

impl ListItem for SimpleItem {
    fn filter_value(&self) -> String {
        self.0.to_string()
    }
}

struct SimpleDelegate;

impl ItemDelegate for SimpleDelegate {
    fn render(&self, item: &dyn ListItem, _context: RenderContext<'_>) -> Vec<Line<'static>> {
        vec![Line::styled(item.filter_value(), Style::default())]
    }
}

struct HelpMap;

impl help::KeyMap for HelpMap {
    fn short_help(&self) -> Vec<key::Binding> {
        vec![
            key::Binding::new([key::with_keys(&["x"]), key::with_help("enter", "continue")]),
            key::Binding::new([key::with_keys(&["x"]), key::with_help("esc", "back")]),
            key::Binding::new([key::with_keys(&["x"]), key::with_help("?", "help")]),
        ]
    }

    fn full_help(&self) -> Vec<Vec<key::Binding>> {
        vec![
            vec![key::Binding::new([
                key::with_keys(&["x"]),
                key::with_help("enter", "continue"),
            ])],
            vec![
                key::Binding::new([key::with_keys(&["x"]), key::with_help("esc", "back")]),
                key::Binding::new([key::with_keys(&["x"]), key::with_help("?", "help")]),
            ],
            vec![
                key::Binding::new([key::with_keys(&["x"]), key::with_help("H", "home")]),
                key::Binding::new([key::with_keys(&["x"]), key::with_help("ctrl+c", "quit")]),
                key::Binding::new([key::with_keys(&["x"]), key::with_help("ctrl+l", "log")]),
            ],
        ]
    }
}

#[test]
fn list_filters_items() {
    let mut list = ListModel::new(
        vec![SimpleItem("alpha"), SimpleItem("beta")],
        SimpleDelegate,
        40,
        10,
    );
    list.handle_key(&KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    list.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
    list.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        list.selected_item().map(|i| i.filter_value()),
        Some("beta".to_string())
    );
}

#[test]
fn help_full_view_obeys_width() {
    let mut model = help::Model::new();
    model.show_all = true;
    model.full_separator = " | ".into();
    model.set_width(20);
    let lines = model.view(&HelpMap);
    assert!(!lines.is_empty());
    let rendered = lines
        .iter()
        .map(|l| l.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("…") || rendered.contains("continue"));
}

#[test]
fn list_goes_to_end_with_uppercase_g() {
    let mut list = ListModel::new(
        vec![SimpleItem("alpha"), SimpleItem("beta"), SimpleItem("gamma")],
        SimpleDelegate,
        40,
        10,
    );
    list.handle_key(&KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT));
    assert_eq!(list.index(), 2);
}

#[test]
fn list_selected_item_tracks_page_offset() {
    let mut list = ListModel::new(
        vec![
            SimpleItem("one"),
            SimpleItem("two"),
            SimpleItem("three"),
            SimpleItem("four"),
        ],
        SimpleDelegate,
        40,
        5,
    );
    list.next_page();
    assert_eq!(list.index(), 1);
    assert_eq!(
        list.selected_item().map(|i| i.filter_value()),
        Some("two".to_string())
    );
}

#[test]
fn list_set_filter_text_and_state_match_go_behavior() {
    let mut list = ListModel::new(
        vec![SimpleItem("foo"), SimpleItem("bar"), SimpleItem("baz")],
        SimpleDelegate,
        40,
        10,
    );
    list.set_filter_text("ba");
    list.set_filter_state(FilterState::Unfiltered);
    assert_eq!(list.visible_items().len(), 3);
    list.set_filter_state(FilterState::Filtering);
    assert_eq!(
        list.visible_items()
            .iter()
            .map(|item| item.filter_value())
            .collect::<Vec<_>>(),
        vec!["bar".to_string(), "baz".to_string()]
    );
    list.set_filter_state(FilterState::FilterApplied);
    assert_eq!(
        list.visible_items()
            .iter()
            .map(|item| item.filter_value())
            .collect::<Vec<_>>(),
        vec!["bar".to_string(), "baz".to_string()]
    );
}

#[test]
fn list_default_delegate_renders_selected_accent() {
    let list = ListModel::new(
        vec![
            ratatui_glamour::widgets::list::DefaultListItem {
                title: "alpha".into(),
                description: "first".into(),
                filter_value: "alpha".into(),
            },
            ratatui_glamour::widgets::list::DefaultListItem {
                title: "beta".into(),
                description: "second".into(),
                filter_value: "beta".into(),
            },
        ],
        ratatui_glamour::widgets::list::DefaultDelegate::new(),
        24,
        8,
    );
    let lines = list.view();
    assert!(lines.iter().any(|line| line.to_string().starts_with("│ ")));
}

#[test]
fn list_filtering_empty_query_dims_rows() {
    let mut list = ListModel::new(
        vec![
            ratatui_glamour::widgets::list::DefaultListItem {
                title: "alpha".into(),
                description: "first".into(),
                filter_value: "alpha".into(),
            },
            ratatui_glamour::widgets::list::DefaultListItem {
                title: "beta".into(),
                description: "second".into(),
                filter_value: "beta".into(),
            },
        ],
        ratatui_glamour::widgets::list::DefaultDelegate::new(),
        24,
        12,
    );
    list.set_filter_state(FilterState::Filtering);
    let lines = list.view();
    assert_eq!(
        lines[5].spans[1].style.fg,
        Some(ratatui::style::Color::Indexed(240))
    );
}

#[test]
fn list_filter_matches_are_exposed_as_char_positions() {
    let mut list = ListModel::new(
        vec![SimpleItem("alpha"), SimpleItem("beta")],
        SimpleDelegate,
        40,
        10,
    );
    list.set_filter_text("ph");
    assert_eq!(list.matches_for_item(0), vec![2, 3]);
}

#[test]
fn list_default_delegate_truncates_by_display_width() {
    let list = ListModel::new(
        vec![ratatui_glamour::widgets::list::DefaultListItem {
            title: "你好你好你好".into(),
            description: "wide".into(),
            filter_value: "你好你好你好".into(),
        }],
        ratatui_glamour::widgets::list::DefaultDelegate::new(),
        8,
        8,
    );
    let lines = list.view();
    assert!(lines[1].to_string().contains('…'));
}

#[test]
fn list_pagination_line_includes_dots_and_counts() {
    let list = ListModel::new(
        vec![
            SimpleItem("one"),
            SimpleItem("two"),
            SimpleItem("three"),
            SimpleItem("four"),
        ],
        SimpleDelegate,
        18,
        6,
    );
    let rendered = list
        .view()
        .iter()
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("1/2"));
    assert!(rendered.contains('•'));
}

#[test]
fn viewport_soft_wraps_and_scrolls() {
    let mut viewport = Viewport::new([]);
    viewport.set_width(5);
    viewport.set_height(1);
    viewport.soft_wrap = true;
    viewport.set_content("abcdef");
    assert_eq!(viewport.visible_lines(), vec!["abcde".to_string()]);
    viewport.scroll_down(1);
    assert_eq!(viewport.visible_lines(), vec!["f".to_string()]);
}

#[test]
fn viewport_highlight_moves_into_view() {
    let mut viewport = Viewport::new([]);
    viewport.set_width(10);
    viewport.set_height(1);
    viewport.set_content("a\nb\nc\nd");
    viewport.set_highlights(vec![[6, 7]]);
    assert_eq!(viewport.y_offset(), 3);
}

#[test]
fn viewport_replacing_content_clears_highlights() {
    let mut viewport = Viewport::new([]);
    viewport.set_width(10);
    viewport.set_height(1);
    viewport.set_content("a\nb\nc\nd");
    viewport.set_highlights(vec![[6, 7]]);
    viewport.set_content("short");
    assert_eq!(viewport.y_offset(), 0);
}

#[test]
fn timer_ticks_down() {
    let mut timer = Timer::new(Duration::from_secs(2), []);
    let msg = timer.tick_msg();
    assert!(timer.update_tick(msg));
    assert_eq!(timer.timeout, Duration::from_secs(1));
}

#[test]
fn stopwatch_ticks_up() {
    let mut sw = Stopwatch::new([]);
    sw.update_start_stop(sw.start());
    let msg = sw.tick_msg();
    assert!(sw.update_tick(msg));
    assert_eq!(sw.elapsed(), Duration::from_secs(1));
}

#[test]
fn progress_renders_percentage() {
    let mut progress = Progress::new([]);
    progress.set_width(10);
    progress.set_percent(0.5);
    assert!(progress.view_as(0.5).contains("50%"));
}

#[test]
fn cursor_focus_and_blink() {
    let mut cursor = Cursor::new();
    cursor.focus();
    cursor.set_char("x");
    let msg = cursor.blink_msg().unwrap();
    assert!(cursor.update_blink(msg));
}

#[test]
fn filepicker_reads_directory() {
    let root = PathBuf::from("target/test-filepicker");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    let mut picker = FilePicker::new();
    picker.current_directory = root.clone();
    picker.read_dir().unwrap();
    assert!(!picker.files().is_empty());
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn filepicker_goto_last_uses_uppercase_g() {
    let root = PathBuf::from("target/test-filepicker-g");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    fs::write(root.join("b.txt"), b"x").unwrap();
    let mut picker = FilePicker::new();
    picker.current_directory = root.clone();
    picker.set_height(10);
    picker.read_dir().unwrap();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT))
        .unwrap();
    assert_eq!(
        picker.selected_index(),
        picker.files().len().saturating_sub(1)
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn filepicker_enter_selects_file() {
    let root = PathBuf::from("target/test-filepicker-enter");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    let mut picker = FilePicker::new();
    picker.current_directory = root.clone();
    picker.set_height(10);
    picker.read_dir().unwrap();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(picker.path.ends_with("a.txt"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn filepicker_allowed_types_gate_selection() {
    let root = PathBuf::from("target/test-filepicker-types");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    fs::write(root.join("b.md"), b"x").unwrap();
    let mut picker = FilePicker::new();
    picker.current_directory = root.clone();
    picker.allowed_types = vec![".md".to_string()];
    picker.set_height(10);
    picker.read_dir().unwrap();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(picker.path.as_os_str().is_empty());
    picker
        .handle_key(&KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
        .unwrap();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
        .unwrap();
    assert!(picker.path.ends_with("b.md"));
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn filepicker_restores_previous_selection_when_going_back() {
    let root = PathBuf::from("target/test-filepicker-stack");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("dir")).unwrap();
    fs::write(root.join("a.txt"), b"x").unwrap();
    fs::write(root.join("dir").join("inside.txt"), b"x").unwrap();

    let mut picker = FilePicker::new();
    picker.current_directory = root.clone();
    picker.set_height(10);
    picker.read_dir().unwrap();
    let before = picker.selected_index();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Right, KeyModifiers::NONE))
        .unwrap();
    picker
        .handle_key(&KeyEvent::new(KeyCode::Left, KeyModifiers::NONE))
        .unwrap();
    assert_eq!(picker.selected_index(), before);

    let _ = fs::remove_dir_all(&root);
}
