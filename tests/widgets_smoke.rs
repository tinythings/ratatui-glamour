use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{style::Style, text::Line};
use std::time::Duration;
use std::{fs, path::PathBuf};

use ratatui_glamour::widgets::{cursor::Model as Cursor, filepicker::Model as FilePicker, key, list::{Item as ListItem, ItemDelegate, Model as ListModel}, paginator::{Model as Paginator, Type}, progress::Model as Progress, stopwatch::Model as Stopwatch, textinput::Model as TextInput, textarea::Model as TextArea, timer::Model as Timer, viewport::Model as Viewport};

#[test]
fn key_binding_matches_ctrl_combo() {
    let binding = key::Binding::new([key::with_keys(&["ctrl+a"])]);
    let event = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert!(key::matches(&event, [&binding]));
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
fn textarea_inserts_multiple_lines() {
    let mut area = TextArea::new();
    area.focus();
    area.insert_string("Foo\nBar");
    assert_eq!(area.value(), "Foo\nBar");
    assert_eq!(area.line(), 1);
    assert_eq!(area.column(), 3);
}

struct SimpleItem(&'static str);

impl ListItem for SimpleItem {
    fn filter_value(&self) -> String { self.0.to_string() }
}

struct SimpleDelegate;

impl ItemDelegate for SimpleDelegate {
    fn render(&self, item: &dyn ListItem, _selected: bool, _width: usize) -> Vec<Line<'static>> {
        vec![Line::styled(item.filter_value(), Style::default())]
    }
}

#[test]
fn list_filters_items() {
    let mut list = ListModel::new(vec![SimpleItem("alpha"), SimpleItem("beta")], SimpleDelegate, 40, 10);
    list.handle_key(&KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    list.handle_key(&KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
    list.handle_key(&KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(list.selected_item().map(|i| i.filter_value()), Some("beta".to_string()));
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
    assert!(progress.view().contains("50%"));
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
