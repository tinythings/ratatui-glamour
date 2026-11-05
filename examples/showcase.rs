use std::{
    cell::RefCell,
    env, io,
    path::{Path, PathBuf},
    rc::Rc,
    time::{Duration, Instant},
};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs, Widget},
};
use ratatui_glamour::{
    border::Border,
    canvas::{Compositor, Layer},
    color::{blend_1d, blend_2d},
    list::{List, ListItem, arabic, bullet},
    surface::{
        centered_rect, gradient_rounded_panel_lines, place_with_pattern, render_classic_tabs_row,
        render_gradient_rounded_panel,
    },
    table::{Column as TableColumn, Model as TableModel, Styles as TableModelStyles},
    tree::{Tree, TreeNode, rounded_enumerator},
    widgets::{
        checkbox::Model as WidgetCheckbox,
        dropdown::{LabelAlign as DropdownLabelAlign, Model as WidgetDropdown},
        filepicker::Model as WidgetFilePicker,
        list::{DefaultDelegate as WidgetListDelegate, DefaultListItem, Model as WidgetList},
        progress::Model as WidgetProgress,
        radiogroup::{Model as WidgetRadioGroup, Orientation as RadioOrientation},
        textarea::Model as WidgetTextArea,
        textinput::Model as WidgetTextInput,
    },
};
use unicode_width::UnicodeWidthStr;

const TAB_TITLES: [&str; 9] = [
    "Gradients",
    "Borders",
    "Tree + List",
    "Table",
    "Layers",
    "Layout",
    "Widgets",
    "Widgets 2",
    "Forms",
];
const LAYOUT_WIDTH: u16 = 96;
const LAYOUT_HEIGHT: u16 = 52;

fn list_default_items() -> Vec<DefaultListItem> {
    [
        ("Raspberry Pi’s", "I have ’em all over my house"),
        ("Nutella", "It's good on toast"),
        ("Bitter melon", "It cools you down"),
        ("Nice socks", "And by that I mean socks without holes"),
        ("Eight hours of sleep", "I had this once"),
        ("Cats", "Usually"),
        ("Plantasia, the album", "My plants love it too"),
        ("Pour over coffee", "It takes forever to make though"),
        ("VR", "Virtual reality...what is there to say?"),
        ("Noguchi Lamps", "Such pleasing organic forms"),
        ("Linux", "Pretty much the best OS"),
        ("Business school", "Just kidding"),
        ("Pottery", "Wet clay is a great feeling"),
        ("Shampoo", "Nothing like clean hair"),
        ("Table tennis", "It’s surprisingly exhausting"),
        ("Milk crates", "Great for packing in your extra stuff"),
        ("Afternoon tea", "Especially the tea sandwich part"),
        ("Stickers", "The thicker the vinyl the better"),
        ("20° Weather", "Celsius, not Fahrenheit"),
        ("Warm light", "Like around 2700 Kelvin"),
        (
            "The vernal equinox",
            "The autumnal equinox is pretty good too",
        ),
        ("Gaffer’s tape", "Basically sticky fabric"),
        ("Terrycloth", "In other words, towel fabric"),
    ]
    .into_iter()
    .map(|(title, description)| DefaultListItem::new(title, description))
    .collect()
}

type FormFocusHook = Box<dyn FnMut(&str)>;
type FormChangeHook = Box<dyn FnMut(&str, &str)>;

enum FormWidget {
    Text(Box<WidgetTextInput>),
    Dropdown(Box<WidgetDropdown>),
    Checkbox(Box<WidgetCheckbox>),
    Radio(Box<WidgetRadioGroup>),
}

impl FormWidget {
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        match self {
            Self::Text(input) => {
                input.set_width(area.width.saturating_sub(2) as usize);
                input.render(area, buf);
            }
            Self::Dropdown(dropdown) => {
                dropdown.width = Some(area.width);
                dropdown.render_field(area, buf);
            }
            Self::Checkbox(checkbox) => checkbox.render(area, buf),
            Self::Radio(radio) => radio.render(area, buf),
        }
    }

    fn render_popup(&mut self, buf: &mut Buffer) {
        if let Self::Dropdown(dropdown) = self {
            dropdown.render_popup(buf);
        }
    }

    fn focus(&mut self) {
        if let Self::Text(input) = self {
            input.focus();
        }
    }

    fn blur(&mut self) {
        if let Self::Text(input) = self {
            input.blur();
        }
    }

    fn close(&mut self) {
        if let Self::Dropdown(dropdown) = self {
            dropdown.close();
        }
    }

    fn is_open(&self) -> bool {
        matches!(self, Self::Dropdown(dropdown) if dropdown.is_open())
    }

    fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    fn value(&self) -> String {
        match self {
            Self::Text(input) => input.value(),
            Self::Dropdown(dropdown) => dropdown.value(),
            Self::Checkbox(checkbox) => {
                if checkbox.checked {
                    "on".to_string()
                } else {
                    "off".to_string()
                }
            }
            Self::Radio(radio) => radio
                .options
                .get(radio.selected)
                .cloned()
                .unwrap_or_default(),
        }
    }

    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        match self {
            Self::Text(input) => {
                input.handle_key(key);
                true
            }
            Self::Dropdown(dropdown) => {
                let _ = dropdown.handle_key(key);
                true
            }
            Self::Checkbox(checkbox) => checkbox.handle_key(key),
            Self::Radio(radio) => {
                if matches!(key.code, KeyCode::Char(' ') | KeyCode::Enter) {
                    radio.select_next();
                    true
                } else {
                    radio.handle_key(key)
                }
            }
        }
    }

    fn handle_mouse(&mut self, mouse: &MouseEvent) -> bool {
        match self {
            Self::Dropdown(dropdown) => {
                let _ = dropdown.handle_mouse(mouse);
                true
            }
            _ => false,
        }
    }
}

struct FormField {
    label: String,
    description: String,
    widget: FormWidget,
}

struct FormSection {
    title: String,
    fields: Vec<FormField>,
}

/// A small Qt-style form packer: add sections and widgets, attach signals,
/// and the form handles layout, focus, painting and input.
struct Form {
    label_width: u16,
    widget_width: u16,
    gap: u16,
    sections: Vec<FormSection>,
    focus: usize,
    row_areas: Vec<(usize, Rect)>,
    widget_areas: Vec<(usize, Rect)>,
    on_focus: Option<FormFocusHook>,
    on_change: Option<FormChangeHook>,
    next_blink: Instant,
}

impl Form {
    fn new(label_width: u16, widget_width: u16, gap: u16) -> Self {
        Self {
            label_width,
            widget_width,
            gap,
            sections: Vec::new(),
            focus: usize::MAX,
            row_areas: Vec::new(),
            widget_areas: Vec::new(),
            on_focus: None,
            on_change: None,
            next_blink: Instant::now(),
        }
    }

    fn section(&mut self, title: impl Into<String>) -> &mut Self {
        self.sections.push(FormSection {
            title: title.into(),
            fields: Vec::new(),
        });
        self
    }

    fn text(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        value: &str,
    ) -> &mut Self {
        self.add(
            label,
            description,
            FormWidget::Text(Box::new(form_text_input(value))),
        )
    }

    fn dropdown(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        items: Vec<DefaultListItem>,
    ) -> &mut Self {
        let mut dropdown = WidgetDropdown::new("", items);
        dropdown.width = Some(self.widget_width);
        dropdown.popup_width = Some(56);
        self.add(label, description, FormWidget::Dropdown(Box::new(dropdown)))
    }

    fn checkbox(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        text: &str,
        checked: bool,
    ) -> &mut Self {
        let mut checkbox = WidgetCheckbox::new(text);
        checkbox.checked = checked;
        checkbox.style = Style::default().fg(Color::Indexed(252));
        checkbox.checked_style = Style::default()
            .fg(Color::Indexed(219))
            .add_modifier(Modifier::BOLD);
        checkbox.unchecked_style = Style::default().fg(Color::Indexed(238));
        self.add(label, description, FormWidget::Checkbox(Box::new(checkbox)))
    }

    fn radio(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        options: [&str; 2],
    ) -> &mut Self {
        let mut radio = WidgetRadioGroup::new(options);
        radio.option_gap = 2;
        radio.style = Style::default().fg(Color::Indexed(252));
        radio.checked_style = Style::default()
            .fg(Color::Indexed(219))
            .add_modifier(Modifier::BOLD);
        radio.unchecked_style = Style::default().fg(Color::Indexed(238));
        self.add(label, description, FormWidget::Radio(Box::new(radio)))
    }

    fn add(
        &mut self,
        label: impl Into<String>,
        description: impl Into<String>,
        widget: FormWidget,
    ) -> &mut Self {
        if self.sections.is_empty() {
            self.section("Form");
        }
        self.sections
            .last_mut()
            .expect("form section")
            .fields
            .push(FormField {
                label: label.into(),
                description: description.into(),
                widget,
            });
        self
    }

    fn on_focus(&mut self, hook: impl FnMut(&str) + 'static) -> &mut Self {
        self.on_focus = Some(Box::new(hook));
        self
    }

    fn on_change(&mut self, hook: impl FnMut(&str, &str) + 'static) -> &mut Self {
        self.on_change = Some(Box::new(hook));
        self
    }

    fn total_fields(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.fields.len())
            .sum()
    }

    fn field(&self, index: usize) -> Option<&FormField> {
        self.sections
            .iter()
            .flat_map(|section| section.fields.iter())
            .nth(index)
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut FormField> {
        self.sections
            .iter_mut()
            .flat_map(|section| section.fields.iter_mut())
            .nth(index)
    }

    fn focused_section(&self) -> Option<usize> {
        let mut offset = 0usize;
        for (index, section) in self.sections.iter().enumerate() {
            let end = offset + section.fields.len();
            if self.focus >= offset && self.focus < end {
                return Some(index);
            }
            offset = end;
        }
        None
    }

    fn focus_first(&mut self) {
        if self.total_fields() > 0 {
            self.focus_field(0);
        }
    }

    fn focus_field(&mut self, index: usize) {
        let total = self.total_fields();
        if total == 0 {
            self.focus = usize::MAX;
            return;
        }
        let index = index.min(total - 1);
        let changed = self.focus != index;
        self.focus = index;
        for section in &mut self.sections {
            for field in &mut section.fields {
                field.widget.blur();
                field.widget.close();
            }
        }
        if let Some(field) = self.field_mut(index) {
            field.widget.focus();
        }
        if changed {
            let label = self
                .field(index)
                .map(|field| field.label.clone())
                .unwrap_or_default();
            if let Some(hook) = self.on_focus.as_mut() {
                hook(&label);
            }
        }
        self.reset_blink();
    }

    fn reset_blink(&mut self) {
        let now = Instant::now();
        let speed = {
            let Some(field) = self.field_mut(self.focus) else {
                return;
            };
            match &mut field.widget {
                FormWidget::Text(input) => {
                    input.virtual_cursor.focus();
                    Some(input.virtual_cursor.blink_speed)
                }
                _ => None,
            }
        };
        if let Some(speed) = speed {
            self.next_blink = now + speed;
        }
    }

    fn tick(&mut self) {
        let now = Instant::now();
        if now < self.next_blink {
            return;
        }
        let speed = {
            let Some(field) = self.field_mut(self.focus) else {
                return;
            };
            match &mut field.widget {
                FormWidget::Text(input) => {
                    if let Some(msg) = input.virtual_cursor.blink_msg() {
                        input.virtual_cursor.update_blink(msg);
                    }
                    input.virtual_cursor.blink_speed
                }
                _ => return,
            }
        };
        self.next_blink = now + speed;
    }

    fn next_field(&mut self) {
        let total = self.total_fields();
        if total == 0 {
            return;
        }
        let next = if self.focus == usize::MAX {
            0
        } else {
            (self.focus + 1) % total
        };
        self.focus_field(next);
    }

    fn prev_field(&mut self) {
        let total = self.total_fields();
        if total == 0 {
            return;
        }
        let prev = if self.focus == usize::MAX {
            total - 1
        } else {
            (self.focus + total - 1) % total
        };
        self.focus_field(prev);
    }

    fn value(&self, index: usize) -> Option<String> {
        self.field(index).map(|field| field.widget.value())
    }

    fn emit_change(&mut self, index: usize, before: Option<String>) {
        let Some(after) = self.value(index) else {
            return;
        };
        if before.as_deref() == Some(after.as_str()) {
            return;
        }
        let label = self
            .field(index)
            .map(|field| field.label.clone())
            .unwrap_or_default();
        if let Some(hook) = self.on_change.as_mut() {
            hook(&label, &after);
        }
    }

    fn open_field_index(&self) -> Option<usize> {
        let mut index = 0usize;
        for section in &self.sections {
            for field in &section.fields {
                if field.widget.is_open() {
                    return Some(index);
                }
                index += 1;
            }
        }
        None
    }

    fn handle_key(&mut self, key: &KeyEvent) -> bool {
        if key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return false;
        }
        if self.total_fields() == 0 {
            return false;
        }
        let Some(index) = self.focused_index() else {
            return false;
        };

        if self
            .field(index)
            .is_some_and(|field| field.widget.is_open())
        {
            match key.code {
                KeyCode::Tab => {
                    if let Some(field) = self.field_mut(index) {
                        field.widget.close();
                    }
                    self.next_field();
                    return true;
                }
                KeyCode::BackTab => {
                    if let Some(field) = self.field_mut(index) {
                        field.widget.close();
                    }
                    self.prev_field();
                    return true;
                }
                _ => {
                    let before = self.value(index);
                    if let Some(field) = self.field_mut(index) {
                        field.widget.handle_key(key);
                    }
                    self.emit_change(index, before);
                    return true;
                }
            }
        }

        match key.code {
            KeyCode::Tab => {
                self.next_field();
                return true;
            }
            KeyCode::BackTab => {
                self.prev_field();
                return true;
            }
            KeyCode::Up => {
                self.prev_field();
                return true;
            }
            KeyCode::Down => {
                self.next_field();
                return true;
            }
            KeyCode::Esc => return false,
            _ => {}
        }

        let text_field = self
            .field(index)
            .is_some_and(|field| field.widget.is_text());
        if key.code == KeyCode::Char('q') && !text_field {
            return false;
        }

        let before = self.value(index);
        let handled = self
            .field_mut(index)
            .is_some_and(|field| field.widget.handle_key(key));
        self.emit_change(index, before);
        if handled && text_field {
            self.reset_blink();
        }
        handled
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) {
        if let Some(index) = self.open_field_index() {
            let before = self.value(index);
            if let Some(field) = self.field_mut(index) {
                let _ = field.widget.handle_mouse(&mouse);
            }
            self.emit_change(index, before);
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.prev_field();
                return;
            }
            MouseEventKind::ScrollDown => {
                self.next_field();
                return;
            }
            MouseEventKind::Down(MouseButton::Left) => {}
            _ => return,
        }

        let point = (mouse.column, mouse.row);
        let Some((index, _)) = self
            .row_areas
            .iter()
            .find(|(_, area)| rect_contains(*area, point))
        else {
            return;
        };
        let index = *index;
        self.focus_field(index);
        let widget_hit = self
            .widget_areas
            .iter()
            .any(|(candidate, area)| *candidate == index && rect_contains(*area, point));
        if !widget_hit {
            return;
        }
        let before = self.value(index);
        if let Some(field) = self.field_mut(index) {
            match &mut field.widget {
                FormWidget::Dropdown(dropdown) => {
                    let _ = dropdown.handle_mouse(&mouse);
                }
                FormWidget::Checkbox(checkbox) => checkbox.toggle(),
                FormWidget::Radio(radio) => radio.select_next(),
                FormWidget::Text(_) => {}
            }
        }
        self.emit_change(index, before);
    }

    fn focused_index(&self) -> Option<usize> {
        (self.focus != usize::MAX && self.focus < self.total_fields()).then_some(self.focus)
    }

    fn render(&mut self, area: Rect, frame: &mut Frame) {
        self.row_areas.clear();
        self.widget_areas.clear();
        let focused_section = self.focused_section();
        let description_x = area.x + self.label_width + self.gap + self.widget_width + self.gap;
        let description_width = area.right().saturating_sub(description_x + 1);
        let mut y = area.y;
        let mut field_index = 0usize;

        for (section_index, section) in self.sections.iter_mut().enumerate() {
            if y >= area.bottom() {
                break;
            }
            render_form_divider(
                frame,
                area,
                y,
                &section.title,
                focused_section == Some(section_index),
            );
            y += 1;
            for field in &mut section.fields {
                let lines = wrap_form_text(&field.description, description_width);
                let row_height = lines.len().max(1) as u16;
                if y.saturating_add(row_height) > area.bottom() {
                    break;
                }
                let focused = field_index == self.focus;
                let row_area = Rect::new(area.x, y, area.width, row_height);
                if focused {
                    fill_rect(
                        frame.buffer_mut(),
                        row_area,
                        Style::default().bg(Color::Indexed(235)),
                    );
                }

                let label_pad = self
                    .label_width
                    .saturating_sub(UnicodeWidthStr::width(field.label.as_str()) as u16);
                let label_style = if focused {
                    Style::default()
                        .fg(Color::Indexed(231))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Indexed(244))
                };
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(field.label.clone(), label_style))),
                    Rect::new(
                        area.x + label_pad,
                        y,
                        self.label_width.saturating_sub(label_pad),
                        1,
                    ),
                );

                let widget_area = Rect::new(
                    area.x + self.label_width + self.gap,
                    y,
                    self.widget_width,
                    1,
                );
                field.widget.render(widget_area, frame.buffer_mut());
                self.widget_areas.push((field_index, widget_area));

                let description_style = if focused {
                    Style::default().fg(Color::Indexed(231))
                } else {
                    Style::default().fg(if field_index.is_multiple_of(2) {
                        Color::Indexed(244)
                    } else {
                        Color::Indexed(238)
                    })
                };
                for (line_index, line) in lines.iter().enumerate() {
                    frame.buffer_mut().set_line(
                        description_x,
                        y + line_index as u16,
                        &Line::from(Span::styled(line.clone(), description_style)),
                        description_width,
                    );
                }

                self.row_areas.push((field_index, row_area));
                y += row_height;
                field_index += 1;
            }
        }

        for section in &mut self.sections {
            for field in &mut section.fields {
                field.widget.render_popup(frame.buffer_mut());
            }
        }
    }
}

fn form_text_input(value: &str) -> WidgetTextInput {
    let mut input = WidgetTextInput::new();
    input.prompt = "> ".to_string();
    input.set_width(28);
    input.set_value(value);
    let mut styles = input.styles().clone();
    styles.focused.text = Style::default().fg(Color::Indexed(252));
    styles.blurred.text = Style::default().fg(Color::Indexed(244));
    styles.focused.placeholder = Style::default().fg(Color::Indexed(240));
    styles.blurred.placeholder = Style::default().fg(Color::Indexed(240));
    styles.focused.prompt = Style::default().fg(Color::Indexed(219));
    styles.blurred.prompt = Style::default().fg(Color::Indexed(240));
    styles.cursor.style = Style::default().fg(Color::Indexed(219));
    input.set_styles(styles);
    input
}

fn flower_items() -> Vec<DefaultListItem> {
    vec![
        DefaultListItem::new("Rose", "Soft layered petals with a classic scent"),
        DefaultListItem::new("Tulip", "Spring bulbs in bright, saturated colours"),
        DefaultListItem::new("Lavender", "Fragrant purple spikes for sunny borders"),
        DefaultListItem::new("Sunflower", "Tall stems that follow the light"),
        DefaultListItem::new("Orchid", "Elegant tropical blooms on slow stems"),
        DefaultListItem::new("Daisy", "Cheerful white petals around a golden eye"),
        DefaultListItem::new("Peony", "Full ruffled flowers in late spring"),
        DefaultListItem::new("Hydrangea", "Clusters of tiny florets that shift colour"),
        DefaultListItem::new("Jasmine", "Small white flowers with a heavy perfume"),
        DefaultListItem::new("Lotus", "Still water, wide leaves and quiet colour"),
        DefaultListItem::new("Poppy", "Paper-thin petals above fine green stems"),
        DefaultListItem::new("Iris", "Sword leaves and folded violet petals"),
        DefaultListItem::new("Camellia", "Glossy leaves and winter blooms"),
        DefaultListItem::new("Bluebell", "Woodland bells in early spring"),
    ]
}

fn build_form() -> (Form, Rc<RefCell<String>>) {
    let status = Rc::new(RefCell::new("focus: name".to_string()));
    let mut form = Form::new(12, 30, 2);

    let focus_status = Rc::clone(&status);
    form.on_focus(move |label| {
        *focus_status.borrow_mut() = format!("focus: {label}");
    });
    let change_status = Rc::clone(&status);
    form.on_change(move |label, value| {
        *change_status.borrow_mut() = format!("change: {label} = {value}");
    });

    form.section("Identity")
        .text(
            "name",
            "Human readable identifier used in\nthe showcase form.",
            "morning-glow",
        )
        .text(
            "version",
            "Semantic version of this form.\nBumped by release automation.",
            "1.4.0",
        )
        .section("Appearance")
        .radio(
            "style",
            "Overall visual treatment for the\nform preview.",
            ["soft", "vivid"],
        )
        .dropdown(
            "flower",
            "Pick a favourite bloom from the\ncatalogue. Paging keeps the list short.",
            flower_items(),
        )
        .checkbox(
            "optional",
            "Optional flourish shown in the\npreview without changing layout.",
            "add sparkle",
            true,
        )
        .section("Notes")
        .text(
            "notes",
            "Free-form note for the operator.\nThis text is not sent anywhere.",
            "keep the petals fresh",
        );

    form.focus_first();
    (form, status)
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = ShowcaseApp::default();
    loop {
        app.tick();
        terminal.draw(|frame| render_app(frame, &mut app))?;
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    if app.handle_widgets_key(key) {
                        continue;
                    }
                    if matches!(key.code, KeyCode::Char('q') | KeyCode::Esc) {
                        break;
                    }
                    app.handle_global_key(key);
                }
                Event::Mouse(mouse) => {
                    let area: Rect = terminal.size()?.into();
                    app.handle_mouse(mouse, area);
                }
                _ => {}
            }
        }
    }
    Ok(())
}

struct ShowcaseApp {
    tab: usize,
    table_model: TableModel,
    layer_cursor_x: u16,
    layer_cursor_y: u16,
    widgets_focus: usize,
    widgets_control_focus: usize,
    widgets_input: WidgetTextInput,
    widgets_textarea: WidgetTextArea,
    widgets_list: WidgetList<DefaultListItem, WidgetListDelegate>,
    widgets_picker: WidgetFilePicker,
    widgets_picker_root: PathBuf,
    widgets_progress: WidgetProgress,
    widgets_check_plain: WidgetCheckbox,
    widgets_check_inline: WidgetCheckbox,
    widgets_check_description: WidgetCheckbox,
    widgets_check_both: WidgetCheckbox,
    widgets_radio_horizontal: WidgetRadioGroup,
    widgets_radio_vertical: WidgetRadioGroup,
    widgets2_focus: usize,
    widgets2: [WidgetDropdown; 5],
    form: Form,
    form_hooks: Rc<RefCell<String>>,
}

impl Default for ShowcaseApp {
    fn default() -> Self {
        let mut widgets_input = WidgetTextInput::new();
        widgets_input.focus();
        widgets_input.placeholder = "type, delete, move, suggest".to_string();
        widgets_input.show_suggestions = true;
        widgets_input.set_suggestions(&[
            "ratatui".to_string(),
            "ratatui-glamour".to_string(),
            "bubble widget port".to_string(),
        ]);

        let mut widgets_textarea = WidgetTextArea::new();
        widgets_textarea.placeholder = "Tell me a story.".to_string();
        widgets_textarea.set_height(8);
        widgets_textarea.set_width(28);
        widgets_textarea.prompt = "│ ".to_string();
        widgets_textarea.set_value("Walkin' fast, faces pass\nAnd I'm homebound\nStarin' blankly ahead\nJust makin' my way\nMakin' a way through");
        let mut textarea_styles = widgets_textarea.styles().clone();
        textarea_styles.focused.text = Style::default()
            .fg(Color::Indexed(252))
            .bg(Color::Indexed(234));
        textarea_styles.focused.cursor_line = Style::default()
            .fg(Color::Indexed(252))
            .bg(Color::Indexed(236));
        textarea_styles.focused.line_number = Style::default()
            .fg(Color::Indexed(239))
            .bg(Color::Indexed(234));
        textarea_styles.focused.cursor_line_number = Style::default()
            .fg(Color::Indexed(241))
            .bg(Color::Indexed(236));
        textarea_styles.focused.prompt = Style::default()
            .fg(Color::Indexed(248))
            .bg(Color::Indexed(234));
        textarea_styles.focused.placeholder = Style::default()
            .fg(Color::Indexed(245))
            .bg(Color::Indexed(234));
        textarea_styles.focused.end_of_buffer = Style::default()
            .fg(Color::Indexed(236))
            .bg(Color::Indexed(234));
        textarea_styles.blurred.text = Style::default()
            .fg(Color::Indexed(248))
            .bg(Color::Indexed(234));
        textarea_styles.blurred.cursor_line = Style::default()
            .fg(Color::Indexed(248))
            .bg(Color::Indexed(235));
        textarea_styles.blurred.line_number = Style::default()
            .fg(Color::Indexed(238))
            .bg(Color::Indexed(234));
        textarea_styles.blurred.cursor_line_number = Style::default()
            .fg(Color::Indexed(240))
            .bg(Color::Indexed(235));
        textarea_styles.blurred.prompt = Style::default()
            .fg(Color::Indexed(244))
            .bg(Color::Indexed(234));
        textarea_styles.blurred.placeholder = Style::default()
            .fg(Color::Indexed(243))
            .bg(Color::Indexed(234));
        textarea_styles.blurred.end_of_buffer = Style::default()
            .fg(Color::Indexed(235))
            .bg(Color::Indexed(234));
        textarea_styles.cursor.style = Style::default()
            .fg(Color::Indexed(255))
            .bg(Color::Indexed(252));
        widgets_textarea.set_styles(textarea_styles);

        let mut widgets_list = WidgetList::new(
            vec![
                DefaultListItem {
                    title: "Nightly build".into(),
                    description: "musl module matrix is green again".into(),
                    filter_value: "nightly build musl module matrix".into(),
                },
                DefaultListItem {
                    title: "Docs pass".into(),
                    description: "widgets README synced with examples".into(),
                    filter_value: "docs pass widgets readme examples".into(),
                },
                DefaultListItem {
                    title: "Palette review".into(),
                    description: "dark defaults, marmalade accents, calmer chrome".into(),
                    filter_value: "palette review dark defaults accents".into(),
                },
                DefaultListItem {
                    title: "Release prep".into(),
                    description: "one last cargo check before tagging".into(),
                    filter_value: "release prep cargo check tagging".into(),
                },
            ],
            WidgetListDelegate::new(),
            26,
            12,
        );
        widgets_list.title = "Queue".into();
        widgets_list.item_name_singular = "entry".into();
        widgets_list.item_name_plural = "entries".into();

        let widgets_picker_root = home_directory();
        let mut widgets_picker = WidgetFilePicker::new();
        widgets_picker.current_directory = widgets_picker_root.clone();
        widgets_picker.dir_allowed = true;
        widgets_picker.styles.cursor = Style::default().fg(Color::Rgb(90, 86, 224));
        widgets_picker.styles.directory = Style::default().fg(Color::Rgb(112, 92, 255));
        widgets_picker.styles.file = Style::default().fg(Color::Indexed(252));
        widgets_picker.styles.permission = Style::default().fg(Color::Indexed(241));
        widgets_picker.styles.file_size = Style::default().fg(Color::Indexed(241));
        widgets_picker.styles.selected = Style::default()
            .fg(Color::Indexed(213))
            .add_modifier(Modifier::BOLD);
        widgets_picker.styles.disabled_selected = Style::default().fg(Color::Indexed(247));
        let _ = widgets_picker.read_dir();

        let mut widgets_progress = WidgetProgress::new([]);
        widgets_progress.set_width(34);
        widgets_progress.full = '▌';
        widgets_progress.empty = '░';
        widgets_progress.empty_style = Style::default().fg(Color::Rgb(42, 42, 42));
        widgets_progress.percentage_style = Style::default().fg(Color::Indexed(250));
        widgets_progress.set_colors(vec![Color::Rgb(90, 86, 224), Color::Rgb(238, 111, 248)]);
        widgets_progress.set_percent(0.64);
        let _ = widgets_progress.update(widgets_progress.frame_msg());

        let label_color = Color::Indexed(252);
        let description_color = Color::Indexed(241);
        let checked_color = Color::Rgb(90, 86, 224);
        let unchecked_color = Color::Indexed(99);

        let mut widgets_check_plain = WidgetCheckbox::new("");
        widgets_check_plain.style = Style::default().fg(label_color);
        widgets_check_plain.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_check_plain.unchecked_style = Style::default().fg(unchecked_color);

        let mut widgets_check_inline = WidgetCheckbox::new("label");
        widgets_check_inline.checked = true;
        widgets_check_inline.style = Style::default().fg(label_color);
        widgets_check_inline.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_check_inline.unchecked_style = Style::default().fg(unchecked_color);

        let mut widgets_check_description = WidgetCheckbox::new("").with_description("Description");
        widgets_check_description.style = Style::default().fg(label_color);
        widgets_check_description.description_style = Style::default().fg(description_color);
        widgets_check_description.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_check_description.unchecked_style = Style::default().fg(unchecked_color);

        let mut widgets_check_both = WidgetCheckbox::new("label").with_description("Description");
        widgets_check_both.checked = true;
        widgets_check_both.style = Style::default().fg(label_color);
        widgets_check_both.description_style = Style::default().fg(description_color);
        widgets_check_both.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_check_both.unchecked_style = Style::default().fg(unchecked_color);

        let mut widgets_radio_horizontal = WidgetRadioGroup::new(["one", "two", "three"]);
        widgets_radio_horizontal.option_gap = 2;
        widgets_radio_horizontal.style = Style::default().fg(label_color);
        widgets_radio_horizontal.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_radio_horizontal.unchecked_style = Style::default().fg(unchecked_color);

        let mut widgets_radio_vertical = WidgetRadioGroup::new(["one", "two", "three"]);
        widgets_radio_vertical.orientation = RadioOrientation::Vertical;
        widgets_radio_vertical.selected = 1;
        widgets_radio_vertical.style = Style::default().fg(label_color);
        widgets_radio_vertical.checked_style = Style::default()
            .fg(checked_color)
            .add_modifier(Modifier::BOLD);
        widgets_radio_vertical.unchecked_style = Style::default().fg(unchecked_color);

        let dropdown_items = list_default_items();
        let mut widgets2 = [
            WidgetDropdown::new("default", dropdown_items.clone()),
            WidgetDropdown::new("no paginator", dropdown_items.clone()),
            WidgetDropdown::new("no help", dropdown_items.clone()),
            WidgetDropdown::new("page size 2", dropdown_items.clone()),
            WidgetDropdown::new("width 15", dropdown_items),
        ];
        for dropdown in &mut widgets2 {
            dropdown.width = Some(15);
            dropdown.label_width = Some(13);
            dropdown.label_gap = 0;
            dropdown.label_align = DropdownLabelAlign::Right;
            dropdown.label_suffix = ":".to_string();
            dropdown.styles.label = Style::default().fg(description_color);
            dropdown.select(1);
        }
        widgets2[1].show_paginator = false;
        widgets2[1].select(5);
        widgets2[2].show_help = false;
        widgets2[2].select(10);
        widgets2[3].page_size = 2;
        widgets2[3].select(12);
        widgets2[4].select(6);

        let (form, form_hooks) = build_form();

        let mut table_model = TableModel::new(
            vec![
                TableColumn::new("Rank", 6),
                TableColumn::new("City", 14),
                TableColumn::new("Country", 12),
                TableColumn::new("Population", 14),
            ],
            vec![
                vec![
                    "6".into(),
                    "Mexico City".into(),
                    "Mexico".into(),
                    "22,085,140".into(),
                ],
                vec![
                    "7".into(),
                    "Cairo".into(),
                    "Egypt".into(),
                    "21,750,020".into(),
                ],
                vec![
                    "8".into(),
                    "Beijing".into(),
                    "China".into(),
                    "21,333,332".into(),
                ],
                vec![
                    "9".into(),
                    "Mumbai".into(),
                    "India".into(),
                    "20,961,472".into(),
                ],
                vec![
                    "10".into(),
                    "Osaka".into(),
                    "Japan".into(),
                    "19,059,856".into(),
                ],
                vec![
                    "11".into(),
                    "Chongqing".into(),
                    "China".into(),
                    "16,874,740".into(),
                ],
                vec![
                    "12".into(),
                    "Karachi".into(),
                    "Pakistan".into(),
                    "16,839,950".into(),
                ],
                vec![
                    "13".into(),
                    "Istanbul".into(),
                    "Turkey".into(),
                    "15,636,243".into(),
                ],
                vec![
                    "14".into(),
                    "Buenos Aires".into(),
                    "Argentina".into(),
                    "15,490,415".into(),
                ],
                vec![
                    "15".into(),
                    "Kolkata".into(),
                    "India".into(),
                    "15,133,888".into(),
                ],
                vec![
                    "16".into(),
                    "Lagos".into(),
                    "Nigeria".into(),
                    "15,109,072".into(),
                ],
                vec![
                    "17".into(),
                    "Kinshasa".into(),
                    "DR Congo".into(),
                    "14,970,000".into(),
                ],
                vec![
                    "18".into(),
                    "Manila".into(),
                    "Philippines".into(),
                    "14,406,059".into(),
                ],
                vec![
                    "19".into(),
                    "Tianjin".into(),
                    "China".into(),
                    "14,011,828".into(),
                ],
                vec![
                    "20".into(),
                    "Guangzhou".into(),
                    "China".into(),
                    "13,964,637".into(),
                ],
                vec![
                    "21".into(),
                    "Rio de Janeiro".into(),
                    "Brazil".into(),
                    "13,728,000".into(),
                ],
                vec![
                    "22".into(),
                    "Lahore".into(),
                    "Pakistan".into(),
                    "13,541,764".into(),
                ],
                vec![
                    "23".into(),
                    "Bangalore".into(),
                    "India".into(),
                    "13,193,035".into(),
                ],
                vec![
                    "24".into(),
                    "Shenzhen".into(),
                    "China".into(),
                    "12,831,330".into(),
                ],
                vec![
                    "25".into(),
                    "Moscow".into(),
                    "Russia".into(),
                    "12,640,818".into(),
                ],
            ],
        );
        table_model.styles = TableModelStyles {
            header: Style::default()
                .fg(Color::Indexed(252))
                .add_modifier(Modifier::BOLD),
            cell: Style::default().fg(Color::Indexed(250)),
            selected: Style::default()
                .fg(Color::Indexed(231))
                .bg(Color::Indexed(93)),
        };
        table_model.focus();
        table_model.set_cursor(6);

        Self {
            tab: 0,
            table_model,
            layer_cursor_x: 0,
            layer_cursor_y: 0,
            widgets_focus: 0,
            widgets_control_focus: 0,
            widgets_input,
            widgets_textarea,
            widgets_list,
            widgets_picker,
            widgets_picker_root,
            widgets_progress,
            widgets_check_plain,
            widgets_check_inline,
            widgets_check_description,
            widgets_check_both,
            widgets_radio_horizontal,
            widgets_radio_vertical,
            widgets2_focus: 0,
            widgets2,
            form,
            form_hooks,
        }
    }
}

impl ShowcaseApp {
    fn set_widgets_focus(&mut self, focus: usize) {
        self.widgets_focus = focus % 5;
        if self.widgets_focus == 0 {
            self.widgets_input.focus();
            self.widgets_textarea.blur();
        } else if self.widgets_focus == 1 {
            self.widgets_input.blur();
            self.widgets_textarea.focus();
        } else {
            self.widgets_input.blur();
            self.widgets_textarea.blur();
        }
    }

    fn next_widget_focus(&mut self) {
        self.set_widgets_focus(self.widgets_focus + 1);
    }

    fn prev_widget_focus(&mut self) {
        self.set_widgets_focus((self.widgets_focus + 4) % 5);
    }

    fn next_widgets2_focus(&mut self) {
        self.widgets2_focus = (self.widgets2_focus + 1) % self.widgets2.len();
    }

    fn prev_widgets2_focus(&mut self) {
        self.widgets2_focus = (self.widgets2_focus + self.widgets2.len() - 1) % self.widgets2.len();
    }

    fn handle_widgets2_key(&mut self, key: KeyEvent) -> bool {
        if self.tab != 7 {
            return false;
        }
        if key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return false;
        }

        if self.widgets2[self.widgets2_focus].is_open() {
            match key.code {
                KeyCode::Tab => {
                    self.widgets2[self.widgets2_focus].close();
                    self.next_widgets2_focus();
                    return true;
                }
                KeyCode::BackTab => {
                    self.widgets2[self.widgets2_focus].close();
                    self.prev_widgets2_focus();
                    return true;
                }
                KeyCode::Char('q') => return true,
                _ => {
                    let _ = self.widgets2[self.widgets2_focus].handle_key(&key);
                    return true;
                }
            }
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            KeyCode::Tab => {
                self.next_widgets2_focus();
                return true;
            }
            KeyCode::BackTab => {
                self.prev_widgets2_focus();
                return true;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev_widgets2_focus();
                return true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_widgets2_focus();
                return true;
            }
            _ => {}
        }

        let _ = self.widgets2[self.widgets2_focus].handle_key(&key);
        true
    }

    fn handle_widgets2_mouse(&mut self, mouse: MouseEvent) {
        if let Some(index) = self.widgets2.iter().position(WidgetDropdown::is_open) {
            let _ = self.widgets2[index].handle_mouse(&mouse);
            return;
        }
        let point = (mouse.column, mouse.row);
        let hit = self.widgets2.iter().position(|dropdown| {
            dropdown
                .area()
                .is_some_and(|area| rect_contains(area, point))
        });
        if let Some(index) = hit {
            self.widgets2_focus = index;
            let _ = self.widgets2[index].handle_mouse(&mouse);
        }
    }

    fn handle_forms_key(&mut self, key: KeyEvent) -> bool {
        if self.tab != 8 {
            return false;
        }
        self.form.handle_key(&key)
    }

    fn handle_forms_mouse(&mut self, mouse: MouseEvent) {
        if self.tab == 8 {
            self.form.handle_mouse(mouse);
        }
    }

    fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_TITLES.len();
    }

    fn prev_tab(&mut self) {
        self.tab = (self.tab + TAB_TITLES.len() - 1) % TAB_TITLES.len();
    }

    fn left(&mut self) {
        if self.tab == 4 {
            self.layer_cursor_x = self.layer_cursor_x.saturating_sub(1);
        }
    }

    fn right(&mut self) {
        if self.tab == 4 {
            self.layer_cursor_x = self.layer_cursor_x.saturating_add(1);
        }
    }

    fn handle_global_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::SHIFT) {
            match key.code {
                KeyCode::Left => self.prev_tab(),
                KeyCode::Right => self.next_tab(),
                _ => {}
            }
            return;
        }

        if self.tab == 3 {
            self.table_model.handle_key(&key);
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.up(),
            KeyCode::Down | KeyCode::Char('j') => self.down(),
            KeyCode::Left => self.left(),
            KeyCode::Right => self.right(),
            _ => {}
        }
    }

    fn up(&mut self) {
        if self.tab == 3 {
            self.table_model.move_up(1);
        } else if self.tab == 4 {
            self.layer_cursor_y = self.layer_cursor_y.saturating_sub(1);
        }
    }

    fn down(&mut self) {
        if self.tab == 3 {
            self.table_model.move_down(1);
        } else if self.tab == 4 {
            self.layer_cursor_y = self.layer_cursor_y.saturating_add(1);
        }
    }

    fn handle_controls_key(&mut self, key: KeyEvent) {
        const CONTROL_COUNT: usize = 6;
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.widgets_control_focus =
                    (self.widgets_control_focus + CONTROL_COUNT - 1) % CONTROL_COUNT;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.widgets_control_focus = (self.widgets_control_focus + 1) % CONTROL_COUNT;
            }
            KeyCode::Left => match self.widgets_control_focus {
                4 => self.widgets_radio_horizontal.select_previous(),
                5 => self.widgets_radio_vertical.select_previous(),
                _ => {}
            },
            KeyCode::Right => match self.widgets_control_focus {
                4 => self.widgets_radio_horizontal.select_next(),
                5 => self.widgets_radio_vertical.select_next(),
                _ => {}
            },
            KeyCode::Char(' ') | KeyCode::Enter => match self.widgets_control_focus {
                0 => self.widgets_check_plain.toggle(),
                1 => self.widgets_check_inline.toggle(),
                2 => self.widgets_check_description.toggle(),
                3 => self.widgets_check_both.toggle(),
                4 => self.widgets_radio_horizontal.select_next(),
                5 => self.widgets_radio_vertical.select_next(),
                _ => {}
            },
            _ => {}
        }
    }

    fn handle_widgets_key(&mut self, key: KeyEvent) -> bool {
        if self.tab == 8 {
            return self.handle_forms_key(key);
        }
        if self.tab == 7 {
            return self.handle_widgets2_key(key);
        }
        if self.tab != 6 {
            return false;
        }

        if key.modifiers.contains(KeyModifiers::SHIFT)
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return false;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return false,
            _ => {}
        }

        match key.code {
            KeyCode::Tab => {
                self.next_widget_focus();
                return true;
            }
            KeyCode::BackTab => {
                self.prev_widget_focus();
                return true;
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.widgets_progress.incr_percent(0.05);
                let _ = self
                    .widgets_progress
                    .update(self.widgets_progress.frame_msg());
                return true;
            }
            KeyCode::Char('-') => {
                self.widgets_progress.decr_percent(0.05);
                let _ = self
                    .widgets_progress
                    .update(self.widgets_progress.frame_msg());
                return true;
            }
            _ => {}
        }

        match self.widgets_focus {
            0 => self.widgets_input.handle_key(&key),
            1 => self.widgets_textarea.handle_key(&key),
            2 => self.widgets_list.handle_key(&key),
            3 => {
                let _ = self.widgets_picker.handle_key(&key);
            }
            4 => self.handle_controls_key(key),
            _ => {}
        }
        true
    }

    fn tick(&mut self) {
        if self.tab == 6 {
            let _ = self
                .widgets_progress
                .update(self.widgets_progress.frame_msg());
        }
        if self.tab == 8 {
            self.form.tick();
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        if self.tab == 8 {
            self.handle_forms_mouse(mouse);
            return;
        }
        if self.tab == 7 {
            self.handle_widgets2_mouse(mouse);
            return;
        }
        if self.tab != 6 {
            return;
        }

        let Some(body_area) = widgets_demo_body_area(area) else {
            return;
        };
        let Some(textarea_area) = widgets_textarea_content_area(body_area) else {
            return;
        };
        let point = (mouse.column, mouse.row);
        if !rect_contains(textarea_area, point) {
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollDown => self.widgets_textarea.viewport.scroll_down(3),
            MouseEventKind::ScrollUp => self.widgets_textarea.viewport.scroll_up(3),
            _ => {}
        }
    }
}

fn render_app(frame: &mut Frame, app: &mut ShowcaseApp) {
    let area = frame.area();

    if app.tab == 5 {
        render_layout_demo(area, frame);
        return;
    }

    paint_background(frame.buffer_mut(), area);

    let outer = Block::default()
        .title(Line::from(vec![
            Span::styled(
                "⚡ ",
                Style::default()
                    .fg(Color::Indexed(219))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "ratatui-glamour",
                Style::default()
                    .fg(Color::Indexed(255))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "  interactive showcase",
                Style::default().fg(Color::Indexed(189)),
            ),
        ]))
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(141)));
    let inner = outer.inner(area);
    outer.render(area, frame.buffer_mut());

    let [tabs_area, body_area, status_area]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner)
        .as_ref()
        .try_into()
        .unwrap();

    let titles: Vec<Line> = TAB_TITLES
        .iter()
        .map(|title| Line::from(format!(" {title} ")))
        .collect();
    Tabs::new(titles)
        .select(app.tab)
        .style(Style::default().fg(Color::Indexed(189)))
        .highlight_style(
            Style::default()
                .fg(Color::Indexed(16))
                .bg(Color::Indexed(219))
                .add_modifier(Modifier::BOLD),
        )
        .divider("")
        .render(tabs_area, frame.buffer_mut());

    match app.tab {
        0 => render_gradients(body_area, frame),
        1 => render_borders(body_area, frame),
        2 => render_tree_list(body_area, frame),
        3 => render_table(body_area, frame, app),
        4 => render_layers(body_area, frame, app),
        6 => render_widgets_demo(body_area, frame, app),
        7 => render_widgets2_demo(body_area, frame, app),
        8 => render_forms_demo(body_area, frame, app),
        _ => {}
    }

    let help = match app.tab {
        3 => "Shift+←/→ pages  ↑/↓/pgup/pgdn table  g/G home/end  q quit",
        4 => "Shift+←/→ pages  ←/→/↑/↓ move hit cursor  q quit",
        6 => {
            "Tab widgets  ↑/↓ control  Space toggle  ←/→ select  Shift+←/→ pages  +/- progress  q quit"
        }
        7 => "Tab/↑/↓ field  Enter open/select  Esc close  Shift+←/→ pages  q quit",
        8 => "Tab/↑/↓ field  type to edit  Space toggle  ←/→ select  Shift+←/→ pages  q quit",
        _ => "Shift+←/→ pages  q quit",
    };
    Paragraph::new(help)
        .style(
            Style::default()
                .fg(Color::Indexed(255))
                .bg(Color::Indexed(54))
                .add_modifier(Modifier::BOLD),
        )
        .render(status_area, frame.buffer_mut());
}

fn render_gradients(area: Rect, frame: &mut Frame) {
    let [ramp_area, mesh_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    let ramp = Block::default()
        .title(" 1D ramps ")
        .borders(Borders::ALL)
        .border_set(Border::thick().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(81)));
    let ramp_inner = ramp.inner(ramp_area);
    ramp.render(ramp_area, frame.buffer_mut());
    render_ramps(ramp_inner, frame);

    let mesh = Block::default()
        .title(" 2D gradient mesh ")
        .borders(Borders::ALL)
        .border_set(Border::double().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(213)));
    let mesh_inner = mesh.inner(mesh_area);
    mesh.render(mesh_area, frame.buffer_mut());
    render_gradient_mesh(mesh_inner, frame);
}

fn render_ramps(area: Rect, frame: &mut Frame) {
    let ramps = [
        blend_1d(
            area.width as usize,
            &[
                Color::Rgb(255, 0, 102),
                Color::Rgb(255, 178, 0),
                Color::Rgb(255, 255, 255),
            ],
        ),
        blend_1d(
            area.width as usize,
            &[
                Color::Rgb(0, 194, 255),
                Color::Rgb(87, 255, 180),
                Color::Rgb(234, 255, 128),
            ],
        ),
        blend_1d(
            area.width as usize,
            &[
                Color::Rgb(128, 0, 255),
                Color::Rgb(255, 0, 255),
                Color::Rgb(255, 128, 215),
            ],
        ),
    ];
    for (row, ramp) in ramps.iter().enumerate() {
        if row as u16 >= area.height {
            break;
        }
        for (col, color) in ramp.iter().enumerate().take(area.width as usize) {
            if let Some(cell) = frame
                .buffer_mut()
                .cell_mut((area.x + col as u16, area.y + row as u16))
            {
                cell.set_symbol("█");
                cell.set_fg(*color);
            }
        }
    }
}

fn render_gradient_mesh(area: Rect, frame: &mut Frame) {
    let colors = blend_2d(
        area.width as usize,
        area.height as usize,
        28.0,
        &[
            Color::Rgb(13, 15, 40),
            Color::Rgb(92, 27, 153),
            Color::Rgb(0, 215, 255),
            Color::Rgb(255, 95, 175),
        ],
    );
    for y in 0..area.height {
        for x in 0..area.width {
            let idx = y as usize * area.width as usize + x as usize;
            if let Some(cell) = frame.buffer_mut().cell_mut((area.x + x, area.y + y)) {
                cell.set_symbol(" ");
                cell.set_bg(colors[idx]);
            }
        }
    }
}

fn render_borders(area: Rect, frame: &mut Frame) {
    let panels = [
        ("rounded", Border::rounded(), Color::Indexed(141)),
        ("thick", Border::thick(), Color::Indexed(81)),
        ("double", Border::double(), Color::Indexed(213)),
        ("ascii", Border::ascii(), Color::Indexed(220)),
        ("block", Border::block(), Color::Indexed(198)),
        ("markdown", Border::markdown(), Color::Indexed(118)),
    ];
    let rows = 2u32;
    let cols = 3u32;
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, rows), Constraint::Ratio(1, rows)])
        .split(area);
    for (r, row_area) in vertical.iter().enumerate() {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Ratio(1, cols),
                Constraint::Ratio(1, cols),
                Constraint::Ratio(1, cols),
            ])
            .split(*row_area);
        for (c, cell) in horizontal.iter().enumerate() {
            let idx = r * cols as usize + c;
            let (name, border, color) = panels[idx];
            let block = Block::default()
                .title(format!(" {name} "))
                .borders(Borders::ALL)
                .border_set(border.into_border_set())
                .border_style(Style::default().fg(color));
            let inner = block.inner(*cell);
            block.render(*cell, frame.buffer_mut());
            Paragraph::new(format!(
                "{}\n{}\n{}",
                "⚡ powerline-safe", "🭬 unicode-ready", "◉ table junctions later"
            ))
            .style(Style::default().fg(Color::Indexed(255)))
            .render(inner, frame.buffer_mut());
        }
    }
}

fn render_tree_list(area: Rect, frame: &mut Frame) {
    let [left, right]: [Rect; 2] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(8, 17), Constraint::Ratio(9, 17)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    let list_block = Block::default()
        .title(" Lists ")
        .borders(Borders::ALL)
        .border_set(Border::thick().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(219)));
    let list_inner = list_block.inner(left);
    list_block.render(left, frame.buffer_mut());
    let [shopping_area, backlog_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .split(list_inner)
        .as_ref()
        .try_into()
        .unwrap();

    let shopping = List::new()
        .enumerator(bullet)
        .enumerator_style(
            Style::default()
                .fg(Color::Indexed(219))
                .add_modifier(Modifier::BOLD),
        )
        .item_style(Style::default().fg(Color::Indexed(255)))
        .items([
            "⚗ neon primer",
            "🪞 violet powder",
            "matrix gloss\nwith reflected shimmer",
            "circuit liner",
        ]);
    (&shopping).render(shopping_area, frame.buffer_mut());

    let backlog = List::new()
        .enumerator(arabic)
        .enumerator_style(
            Style::default()
                .fg(Color::Indexed(81))
                .add_modifier(Modifier::BOLD),
        )
        .item_style(Style::default().fg(Color::Indexed(230)))
        .items([
            ListItem::from("port border presets"),
            ListItem::from("port tree widget"),
            ListItem::from(vec![
                ListItem::from("multiline items"),
                ListItem::from("table renderer"),
            ]),
        ]);
    (&backlog).render(backlog_area, frame.buffer_mut());

    let tree_block = Block::default()
        .title(" Tree ")
        .borders(Borders::ALL)
        .border_set(Border::double().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(141)));
    let tree_inner = tree_block.inner(right);
    tree_block.render(right, frame.buffer_mut());

    let tree = Tree::new()
        .root("glam source → ratatui-glamour/")
        .root_style(
            Style::default()
                .fg(Color::Indexed(230))
                .add_modifier(Modifier::BOLD),
        )
        .enumerator(rounded_enumerator)
        .enumerator_style(
            Style::default()
                .fg(Color::Indexed(213))
                .add_modifier(Modifier::BOLD),
        )
        .indenter_style(Style::default().fg(Color::Indexed(99)))
        .item_style_fn(|_, idx| {
            if idx % 2 == 0 {
                Style::default().fg(Color::Indexed(255))
            } else {
                Style::default().fg(Color::Indexed(189))
            }
        })
        .children([
            TreeNode::new("border/mod.rs").child(TreeNode::new("preset mapping + ratatui adapter")),
            TreeNode::new("list/mod.rs").children([
                TreeNode::new("arabic"),
                TreeNode::new("roman"),
                TreeNode::new("unicode bullets ⚡"),
            ]),
            TreeNode::new("tree/mod.rs").children([
                TreeNode::new("multiline values\nwith prefix continuation"),
                TreeNode::new("styles + enumerators"),
            ]),
            TreeNode::new("table/mod.rs").child(TreeNode::new("planner + custom grid renderer")),
        ]);
    (&tree).render(tree_inner, frame.buffer_mut());
}

fn render_table(area: Rect, frame: &mut Frame, app: &mut ShowcaseApp) {
    let block = Block::default()
        .title(" Table ")
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(117)));
    let inner = block.inner(area);
    block.render(area, frame.buffer_mut());

    let card = centered_rect(inner, inner.width.min(84), inner.height.min(20));
    let [title_area, divider_area, body_area]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(card)
        .as_ref()
        .try_into()
        .unwrap();

    frame.render_widget(
        Paragraph::new(Line::styled(
            "Table",
            Style::default()
                .fg(Color::Indexed(255))
                .add_modifier(Modifier::BOLD),
        )),
        title_area,
    );
    draw_horizontal_rule(
        frame.buffer_mut(),
        divider_area.x,
        divider_area.y,
        divider_area.width,
        Color::Indexed(238),
    );

    let table_frame = Rect::new(
        body_area.x + 6,
        body_area.y + 1,
        body_area.width.saturating_sub(12),
        body_area.height.saturating_sub(2),
    );
    fill_rect(
        frame.buffer_mut(),
        body_area,
        Style::default().bg(Color::Indexed(235)),
    );
    draw_table_frame(frame.buffer_mut(), table_frame);

    let table_area = Rect::new(
        table_frame.x + 1,
        table_frame.y + 1,
        table_frame.width.saturating_sub(2),
        table_frame.height.saturating_sub(2),
    );
    let table_content_area = Rect::new(
        table_area.x,
        table_area.y,
        table_area.width.saturating_sub(1),
        table_area.height,
    );
    let table_scrollbar_area = Rect::new(
        table_content_area.right(),
        table_area.y,
        table_area.width.saturating_sub(table_content_area.width),
        table_area.height,
    );
    app.table_model.set_size(
        table_content_area.width as usize,
        table_content_area.height as usize,
    );
    app.table_model
        .render(table_content_area, frame.buffer_mut());
    render_dark_scrollbar(
        frame.buffer_mut(),
        table_scrollbar_area,
        app.table_model.viewport.y_offset(),
        app.table_model.height().saturating_sub(1),
        app.table_model.rows().len(),
    );
}

fn render_layers(area: Rect, frame: &mut Frame, app: &ShowcaseApp) {
    let [scene_area, info_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(10, 17), Constraint::Ratio(7, 17)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    let scene_block = Block::default()
        .title(" Compositor ")
        .borders(Borders::ALL)
        .border_set(Border::double().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(213)));
    let scene_inner = scene_block.inner(scene_area);
    scene_block.render(scene_area, frame.buffer_mut());
    fill_rect(
        frame.buffer_mut(),
        scene_inner,
        Style::default().bg(Color::Indexed(234)),
    );

    let compositor = demo_compositor(scene_inner);
    (&compositor).render(scene_inner, frame.buffer_mut());

    let rel_x = app.layer_cursor_x.min(scene_inner.width.saturating_sub(1));
    let rel_y = app.layer_cursor_y.min(scene_inner.height.saturating_sub(1));
    let cursor_x = scene_inner.x + rel_x;
    let cursor_y = scene_inner.y + rel_y;
    if let Some(cell) = frame.buffer_mut().cell_mut((cursor_x, cursor_y)) {
        cell.set_symbol("◎");
        cell.set_fg(Color::Indexed(226));
        cell.set_bg(Color::Indexed(52));
    }

    let hit = compositor.hit(rel_x, rel_y);

    let info_block = Block::default()
        .title(" Hit Test ")
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(117)));
    let info_inner = info_block.inner(info_area);
    info_block.render(info_area, frame.buffer_mut());

    let bounds_text = hit
        .bounds
        .map(|b| format!("x={} y={} w={} h={}", b.x, b.y, b.width, b.height))
        .unwrap_or_else(|| "(none)".to_string());
    let content = format!(
        "cursor: {}, {}\nlayer: {}\nbounds: {}\n\nscene graph:\n- field-a\n- field-b\n- pickles\n- melon\n- sriracha\n\nThis page demonstrates the canvas/layer/compositor stack with z-order hit testing.",
        rel_x,
        rel_y,
        if hit.empty() { "(none)" } else { &hit.id },
        bounds_text,
    );
    Paragraph::new(content)
        .style(Style::default().fg(Color::Indexed(255)))
        .render(info_inner, frame.buffer_mut());
}

fn render_layout_demo(area: Rect, frame: &mut Frame) {
    fill_rect(
        frame.buffer_mut(),
        area,
        Style::default().bg(Color::Indexed(234)),
    );
    let page = centered_rect(
        area,
        LAYOUT_WIDTH.min(area.width.saturating_sub(2)),
        LAYOUT_HEIGHT.min(area.height.saturating_sub(2)),
    );
    fill_rect(
        frame.buffer_mut(),
        page,
        Style::default().bg(Color::Indexed(234)),
    );
    let [top, middle, bottom]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(11),
            Constraint::Length(17),
            Constraint::Length(24),
        ])
        .split(page)
        .as_ref()
        .try_into()
        .unwrap();

    render_layout_tabs(top, frame);
    render_layout_middle(middle, frame);
    render_layout_bottom(bottom, frame);
}

fn render_layout_tabs(area: Rect, frame: &mut Frame) {
    let titles = ["Glamour", "Blush", "Eye Shadow", "Mascara", "Foundation"];
    render_classic_tabs_row(
        frame.buffer_mut(),
        Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 3),
        &titles,
        0,
        Style::default()
            .fg(Color::Indexed(99))
            .bg(Color::Indexed(234)),
        Style::default()
            .fg(Color::Indexed(255))
            .bg(Color::Indexed(234)),
        Style::default()
            .fg(Color::Indexed(255))
            .bg(Color::Indexed(234)),
    );

    let title_stack_x = area.x + 1;
    let title_stack_y = area.y + 5;
    let stack_colors = blend_1d(5, &[Color::Indexed(205), Color::Indexed(63)]);
    for (idx, color) in stack_colors.iter().enumerate() {
        let rect = Rect::new(
            title_stack_x + idx as u16 * 2,
            title_stack_y + idx as u16,
            11,
            1,
        );
        fill_rect(frame.buffer_mut(), rect, Style::default().bg(*color));
        frame.buffer_mut().set_line(
            rect.x,
            rect.y,
            &Line::from(vec![Span::styled(
                " Glamour ",
                Style::default()
                    .fg(Color::Indexed(231))
                    .bg(*color)
                    .add_modifier(Modifier::ITALIC),
            )]),
            rect.width,
        );
    }

    let desc_x = title_stack_x + 18;
    let desc_y = title_stack_y + 1;
    frame.buffer_mut().set_line(
        desc_x,
        desc_y,
        &Line::from(vec![Span::styled(
            "Style Definitions for Nice Terminal Layouts",
            Style::default().fg(Color::Indexed(255)),
        )]),
        area.width.saturating_sub(desc_x - area.x),
    );
    draw_horizontal_rule(
        frame.buffer_mut(),
        desc_x,
        desc_y + 1,
        area.right().saturating_sub(3) - desc_x,
        Color::Indexed(238),
    );
    frame.buffer_mut().set_line(
        desc_x,
        desc_y + 2,
        &Line::from(vec![
            Span::styled("From Charm", Style::default().fg(Color::Indexed(250))),
            Span::styled(" • ", Style::default().fg(Color::Indexed(238))),
            Span::styled(
                "https://github.com/tinythings/ratatui-glamour",
                Style::default().fg(Color::Indexed(48)),
            ),
        ]),
        area.width.saturating_sub(desc_x - area.x),
    );
}

fn render_layout_middle(area: Rect, frame: &mut Frame) {
    let [dialog_area, columns_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(9), Constraint::Length(8)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    let popup_region = Rect::new(
        dialog_area.x,
        dialog_area.y,
        dialog_area.width,
        9.min(dialog_area.height),
    );
    let dialog = place_with_pattern(
        frame.buffer_mut(),
        popup_region,
        52,
        6,
        "猫咪",
        Style::default()
            .fg(Color::Indexed(236))
            .bg(Color::Indexed(234)),
    );

    let inner = render_gradient_rounded_panel(
        frame.buffer_mut(),
        dialog,
        Style::default().bg(Color::Indexed(234)),
        &[
            Color::Indexed(205),
            Color::Indexed(99),
            Color::Indexed(51),
            Color::Indexed(99),
            Color::Indexed(205),
        ],
    );
    let headline = gradient_text_line(
        "Are you sure you want to eat marmalade?",
        &[
            Color::Indexed(229),
            Color::Indexed(221),
            Color::Indexed(216),
            Color::Indexed(210),
            Color::Indexed(204),
        ],
        Color::Indexed(234),
    );
    let content_w = 50u16.min(inner.width);
    let content_x = inner.x + inner.width.saturating_sub(content_w) / 2;
    frame.buffer_mut().set_line(
        content_x,
        inner.y + 1,
        &center_line(headline, content_w as usize, Color::Indexed(234)),
        content_w,
    );

    let yes_w = 9u16;
    let maybe_w = 11u16;
    let gap = 2u16;
    let buttons_w = yes_w + gap + maybe_w;
    let buttons_x = inner.x + inner.width.saturating_sub(buttons_w) / 2;
    draw_button(
        frame.buffer_mut(),
        Rect::new(buttons_x, inner.y + 3, yes_w, 1),
        "Yes",
        Color::Indexed(205),
        Color::Indexed(231),
        true,
    );
    draw_button(
        frame.buffer_mut(),
        Rect::new(buttons_x + yes_w + gap, inner.y + 3, maybe_w, 1),
        "Maybe",
        Color::Indexed(246),
        Color::Indexed(231),
        false,
    );

    let [left, center, right]: [Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .split(columns_area)
        .as_ref()
        .try_into()
        .unwrap();
    render_layout_list(
        frame,
        left,
        "Citrus Fruits to Try",
        &["Grapefruit", "Yuzu", "Citron", "Kumquat", "Pomelo"],
        &[0, 1],
    );
    render_layout_list(
        frame,
        center,
        "Actual Glamour Vendors",
        &["Glossier", "Claire's Boutique", "Nyx", "Mac", "Milk"],
        &[2, 4],
    );
    render_swatch_grid(frame, right);
}

fn render_layout_bottom(area: Rect, frame: &mut Frame) {
    let [body, status]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();
    let [c1, c2, c3]: [Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
            Constraint::Ratio(1, 3),
        ])
        .spacing(2)
        .split(body)
        .as_ref()
        .try_into()
        .unwrap();
    render_paragraph_card(
        frame,
        c1,
        "The Romans learned from the Greeks that quinces slowly cooked with honey would 'set' when cool. The Apicius gives a recipe for preserving whole quinces, stems and leaves attached, in a bath of honey diluted with defrutum.",
    );
    render_paragraph_card(
        frame,
        c2,
        "Medieval quince preserves, which went by the French name cotignac, produced in a clear version and a fruit pulp version, began to lose their medieval seasoning of spices in the 16th century.",
    );
    render_paragraph_card(
        frame,
        c3,
        "In 1524, Henry VIII, King of England, received a 'box of marmalade' from Mr. Hull of Exeter. This was probably marmelada, a solid quince paste from Portugal.",
    );
    let badge = Rect::new(
        c2.right().saturating_sub(7),
        c2.bottom().saturating_sub(3),
        28,
        2,
    );
    fill_rect(
        frame.buffer_mut(),
        badge,
        Style::default().bg(Color::Indexed(204)),
    );
    frame.buffer_mut().set_line(
        badge.x + 3,
        badge.y,
        &Line::from(vec![Span::styled(
            "Now with Compositing!",
            Style::default()
                .fg(Color::Indexed(231))
                .bg(Color::Indexed(204))
                .add_modifier(Modifier::ITALIC),
        )]),
        badge.width - 6,
    );
    render_status_bar(frame, status);
}

fn render_widgets_demo(area: Rect, frame: &mut Frame, app: &mut ShowcaseApp) {
    let [left_area, right_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(7, 16), Constraint::Ratio(9, 16)])
        .spacing(1)
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    let [progress_area, input_area, controls_area, list_area]: [Rect; 4] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(11),
            Constraint::Min(0),
        ])
        .spacing(0)
        .split(left_area)
        .as_ref()
        .try_into()
        .unwrap();

    let [textarea_area, picker_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .spacing(0)
        .split(right_area)
        .as_ref()
        .try_into()
        .unwrap();

    render_demo_block(frame, progress_area, "Progress", false);
    app.widgets_progress
        .render(inner_rect(progress_area), frame.buffer_mut());

    render_demo_block(frame, input_area, "TextInput", app.widgets_focus == 0);
    frame.render_widget(
        Paragraph::new(app.widgets_input.view()),
        inner_rect(input_area),
    );

    render_controls_panel(frame, controls_area, app);

    render_list_panel(frame, list_area, app);

    render_textarea_panel(frame, textarea_area, app);

    render_filepicker_panel(frame, picker_area, app);
}

fn render_widgets2_demo(area: Rect, frame: &mut Frame, app: &mut ShowcaseApp) {
    let open = app.widgets2.iter().any(WidgetDropdown::is_open);
    render_demo_block(frame, area, "Dropdown", open);

    let inner = inner_rect(area);
    let row_x = inner.x + 2;
    let row_width = inner.width.saturating_sub(4);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Bubble list-default inside a relative popup",
            Style::default().fg(Color::Indexed(244)),
        ))),
        Rect::new(row_x, inner.y, row_width, 1),
    );

    let mut y = inner.y + 2;
    for (index, dropdown) in app.widgets2.iter_mut().enumerate() {
        let focused = index == app.widgets2_focus;
        dropdown.styles.label = if focused {
            Style::default()
                .fg(Color::Indexed(219))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Indexed(244))
        };
        dropdown.render_field(Rect::new(row_x, y, row_width, 1), frame.buffer_mut());
        y += 2;
    }

    let selected = app.widgets2[app.widgets2_focus].value();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("selected ", Style::default().fg(Color::Indexed(240))),
            Span::styled(
                format!(" {selected} "),
                Style::default()
                    .fg(Color::Indexed(231))
                    .bg(Color::Indexed(237)),
            ),
        ])),
        Rect::new(row_x, inner.bottom().saturating_sub(2), row_width, 1),
    );
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Enter opens the list · ↑/↓ move · Enter selects · Esc closes",
            Style::default().fg(Color::Indexed(241)),
        ))),
        Rect::new(row_x, inner.bottom().saturating_sub(1), row_width, 1),
    );

    for dropdown in &mut app.widgets2 {
        dropdown.render_popup(frame.buffer_mut());
    }
}

fn render_forms_demo(area: Rect, frame: &mut Frame, app: &mut ShowcaseApp) {
    let block = Block::default()
        .title(" Form ")
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(99)))
        .style(Style::default().bg(Color::Indexed(234)));
    let inner = block.inner(area);
    block.render(area, frame.buffer_mut());

    app.form.render(inner, frame);

    let signal = app.form_hooks.borrow().clone();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("signal  ", Style::default().fg(Color::Indexed(238))),
            Span::styled(signal, Style::default().fg(Color::Indexed(244))),
        ])),
        Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
    );
}

fn render_form_divider(frame: &mut Frame, area: Rect, y: u16, title: &str, active: bool) {
    let neon = Color::Rgb(0, 240, 255);
    let marker_style = if active {
        Style::default().fg(neon).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(141))
    };
    let title_style = if active {
        Style::default().fg(neon).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Indexed(244))
    };
    let rule_style = Style::default().fg(if active {
        Color::Indexed(244)
    } else {
        Color::Indexed(238)
    });
    let title_width = UnicodeWidthStr::width(title) as u16;
    let used = 3u16.saturating_add(title_width);
    let rule_width = area.width.saturating_sub(used.saturating_add(1));

    let line = Line::from(vec![
        Span::styled("◆", marker_style),
        Span::raw("  "),
        Span::styled(title.to_string(), title_style),
        Span::raw(" "),
        Span::styled("─".repeat(rule_width as usize), rule_style),
    ]);
    frame.buffer_mut().set_line(area.x, y, &line, area.width);
}

fn wrap_form_text(text: &str, width: u16) -> Vec<String> {
    let max_width = width.max(1) as usize;
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0usize;
        for word in paragraph.split_whitespace() {
            let word_width = UnicodeWidthStr::width(word);
            if current.is_empty() {
                current.push_str(word);
                current_width = word_width;
            } else if current_width + 1 + word_width <= max_width {
                current.push(' ');
                current.push_str(word);
                current_width += 1 + word_width;
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_width = word_width;
            }
        }
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn render_controls_panel(frame: &mut Frame, area: Rect, app: &ShowcaseApp) {
    render_demo_block(frame, area, "Checkboxes + Radios", app.widgets_focus == 4);
    let inner = inner_rect(area);
    let label_width = 11u16;
    let value_x = inner.x + label_width + 2;
    let value_width = inner.right().saturating_sub(value_x + 1);
    let mut y = inner.y;
    let draw_row = |frame: &mut Frame, y: u16, label: &str, line: Line<'static>, active: bool| {
        let style = if active {
            Style::default()
                .fg(Color::Indexed(219))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Indexed(241))
        };
        frame.render_widget(
            Paragraph::new(Span::styled(label, style)),
            Rect::new(inner.x, y, label_width, 1),
        );
        frame.render_widget(Paragraph::new(line), Rect::new(value_x, y, value_width, 1));
    };
    let focused = app.widgets_focus == 4;

    draw_row(
        frame,
        y,
        "plain",
        app.widgets_check_plain.view(),
        focused && app.widgets_control_focus == 0,
    );
    y += 1;
    draw_row(
        frame,
        y,
        "inline",
        app.widgets_check_inline.view(),
        focused && app.widgets_control_focus == 1,
    );
    y += 1;
    draw_row(
        frame,
        y,
        "description",
        app.widgets_check_description.view(),
        focused && app.widgets_control_focus == 2,
    );
    y += 1;
    draw_row(
        frame,
        y,
        "both",
        app.widgets_check_both.view(),
        focused && app.widgets_control_focus == 3,
    );
    y += 2;
    draw_row(
        frame,
        y,
        "horizontal",
        app.widgets_radio_horizontal.view(),
        focused && app.widgets_control_focus == 4,
    );
    y += 1;
    for (index, line) in app.widgets_radio_vertical.lines().iter().enumerate() {
        draw_row(
            frame,
            y + index as u16,
            if index == 0 { "vertical" } else { "" },
            line.clone(),
            focused && app.widgets_control_focus == 5 && index == 0,
        );
    }
}

fn render_demo_block(frame: &mut Frame, area: Rect, title: &str, focused: bool) {
    let border_color = if focused {
        Color::Indexed(219)
    } else {
        Color::Indexed(99)
    };
    Block::default()
        .title(format!(" {title} "))
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(border_color))
        .render(area, frame.buffer_mut());
}

fn render_textarea_panel(frame: &mut Frame, area: Rect, app: &mut ShowcaseApp) {
    let title_style = if app.widgets_focus == 1 {
        Style::default()
            .fg(Color::Indexed(219))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Indexed(255))
            .add_modifier(Modifier::BOLD)
    };
    let [title_area, body_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    frame.render_widget(
        Paragraph::new(Line::styled("Text Area", title_style)),
        title_area,
    );

    let inner = render_dark_panel(frame, body_area, app.widgets_focus == 1);

    let padded = Rect::new(
        inner.x + 2,
        inner.y + 1,
        inner.width.saturating_sub(4),
        inner.height.saturating_sub(2),
    );
    let content_area = Rect::new(
        padded.x,
        padded.y,
        padded.width.saturating_sub(1),
        padded.height,
    );
    let scrollbar_area = Rect::new(
        content_area.right(),
        padded.y,
        padded.width.saturating_sub(content_area.width),
        padded.height,
    );
    app.widgets_textarea.set_width(content_area.width as usize);
    app.widgets_textarea
        .set_height(content_area.height as usize);
    app.widgets_textarea
        .render(content_area, frame.buffer_mut());
    render_dark_scrollbar(
        frame.buffer_mut(),
        scrollbar_area,
        app.widgets_textarea.viewport.y_offset(),
        app.widgets_textarea.viewport.height(),
        app.widgets_textarea.viewport.total_line_count(),
    );
}

fn render_list_panel(frame: &mut Frame, area: Rect, app: &mut ShowcaseApp) {
    let title_style = if app.widgets_focus == 2 {
        Style::default()
            .fg(Color::Indexed(219))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Indexed(255))
            .add_modifier(Modifier::BOLD)
    };
    let [title_area, body_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    frame.render_widget(
        Paragraph::new(Line::styled("List", title_style)),
        title_area,
    );

    let inner = render_dark_panel(frame, body_area, app.widgets_focus == 2);
    let padded = Rect::new(
        inner.x + 1,
        inner.y + 1,
        inner.width.saturating_sub(2),
        inner.height.saturating_sub(2),
    );
    app.widgets_list
        .set_size(padded.width as usize, padded.height as usize);
    app.widgets_list.render(padded, frame.buffer_mut());
}

fn render_filepicker_panel(frame: &mut Frame, area: Rect, app: &mut ShowcaseApp) {
    let title_style = if app.widgets_focus == 3 {
        Style::default()
            .fg(Color::Indexed(219))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Indexed(255))
            .add_modifier(Modifier::BOLD)
    };
    let [title_area, body_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(area)
        .as_ref()
        .try_into()
        .unwrap();

    frame.render_widget(
        Paragraph::new(Line::styled("File Picker", title_style)),
        title_area,
    );

    let inner = render_dark_panel(frame, body_area, app.widgets_focus == 3);

    let [path_area, selected_area, list_area]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .margin(2)
        .split(inner)
        .as_ref()
        .try_into()
        .unwrap();
    app.widgets_picker.set_height(list_area.height as usize);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{} ", app.widgets_picker.cursor),
                Style::default().fg(Color::Rgb(90, 86, 224)),
            ),
            Span::styled(
                display_home_relative_path(
                    &app.widgets_picker_root,
                    &app.widgets_picker.current_directory,
                ),
                Style::default().fg(Color::Indexed(252)),
            ),
        ]))
        .style(Style::default().bg(Color::Indexed(234))),
        path_area,
    );

    let selected_label = if app.widgets_picker.path.as_os_str().is_empty() {
        Line::from(vec![Span::styled(
            "Pick a file:",
            Style::default().fg(Color::Indexed(252)),
        )])
    } else {
        Line::from(vec![
            Span::styled("Selected file: ", Style::default().fg(Color::Indexed(252))),
            Span::styled(
                display_home_relative_path(&app.widgets_picker_root, &app.widgets_picker.path),
                Style::default()
                    .fg(Color::Indexed(213))
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    };
    frame.render_widget(
        Paragraph::new(selected_label).style(Style::default().bg(Color::Indexed(234))),
        selected_area,
    );

    frame.render_widget(
        Paragraph::new(app.widgets_picker.view()).style(Style::default().bg(Color::Indexed(234))),
        list_area,
    );
}

fn widgets_textarea_content_area(area: Rect) -> Option<Rect> {
    let [_, right_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(7, 16), Constraint::Ratio(9, 16)])
        .spacing(1)
        .split(area)
        .as_ref()
        .try_into()
        .ok()?;
    let [textarea_area, _]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)])
        .spacing(1)
        .split(right_area)
        .as_ref()
        .try_into()
        .ok()?;
    let [_, body_area]: [Rect; 2] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(0)])
        .split(textarea_area)
        .as_ref()
        .try_into()
        .ok()?;
    let inner = Rect::new(
        body_area.x + 1,
        body_area.y + 1,
        body_area.width.saturating_sub(2),
        body_area.height.saturating_sub(2),
    );
    let padded = Rect::new(
        inner.x + 2,
        inner.y + 1,
        inner.width.saturating_sub(4),
        inner.height.saturating_sub(2),
    );
    Some(Rect::new(
        padded.x,
        padded.y,
        padded.width.saturating_sub(1),
        padded.height,
    ))
}

fn widgets_demo_body_area(area: Rect) -> Option<Rect> {
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );
    let [_, body_area, _]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner)
        .as_ref()
        .try_into()
        .ok()?;
    Some(body_area)
}

fn rect_contains(area: Rect, point: (u16, u16)) -> bool {
    point.0 >= area.x && point.0 < area.right() && point.1 >= area.y && point.1 < area.bottom()
}

fn render_dark_scrollbar(
    buf: &mut ratatui::buffer::Buffer,
    area: Rect,
    offset: usize,
    visible: usize,
    total: usize,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let track_style = Style::default()
        .fg(Color::Indexed(236))
        .bg(Color::Indexed(234));
    let thumb_style = Style::default()
        .fg(Color::Indexed(241))
        .bg(Color::Indexed(234));
    for y in 0..area.height {
        buf.set_string(area.x, area.y + y, "│", track_style);
    }

    if total <= visible || visible == 0 {
        return;
    }

    let thumb_height = ((area.height as usize * visible) / total)
        .max(1)
        .min(area.height as usize);
    let max_offset = total.saturating_sub(visible).max(1);
    let thumb_travel = area.height as usize - thumb_height;
    let thumb_top = (offset.min(max_offset) * thumb_travel) / max_offset;
    for y in 0..thumb_height {
        buf.set_string(area.x, area.y + (thumb_top + y) as u16, "█", thumb_style);
    }
}

fn render_dark_panel(frame: &mut Frame, area: Rect, focused: bool) -> Rect {
    let fill = Style::default().bg(Color::Indexed(234));
    if focused {
        return render_gradient_rounded_panel(
            frame.buffer_mut(),
            area,
            fill,
            &[
                Color::Indexed(205),
                Color::Indexed(99),
                Color::Indexed(51),
                Color::Indexed(99),
                Color::Indexed(205),
            ],
        );
    }

    let panel = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Indexed(238)))
        .style(fill);
    let inner = panel.inner(area);
    panel.render(area, frame.buffer_mut());
    inner
}

fn display_home_relative_path(root: &Path, path: &Path) -> String {
    if path == root {
        return "~".to_string();
    }
    match path.strip_prefix(root) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => path.display().to_string(),
    }
}

fn home_directory() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn inner_rect(area: Rect) -> Rect {
    Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn draw_horizontal_rule(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    width: u16,
    color: Color,
) {
    for dx in 0..width {
        buf.set_string(x + dx, y, "─", Style::default().fg(color));
    }
}

fn draw_table_frame(buf: &mut ratatui::buffer::Buffer, area: Rect) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let style = Style::default()
        .fg(Color::Indexed(239))
        .bg(Color::Indexed(235));
    for x in area.x + 1..area.right().saturating_sub(1) {
        buf.set_string(x, area.y, "─", style);
        buf.set_string(x, area.bottom().saturating_sub(1), "─", style);
    }
    for y in area.y + 1..area.bottom().saturating_sub(1) {
        buf.set_string(area.x, y, "│", style);
        buf.set_string(area.right().saturating_sub(1), y, "│", style);
    }
    buf.set_string(area.x, area.y, "┌", style);
    buf.set_string(area.right().saturating_sub(1), area.y, "┐", style);
    buf.set_string(area.x, area.bottom().saturating_sub(1), "└", style);
    buf.set_string(
        area.right().saturating_sub(1),
        area.bottom().saturating_sub(1),
        "┘",
        style,
    );
}

fn fill_rect(buf: &mut ratatui::buffer::Buffer, area: Rect, style: Style) {
    for y in 0..area.height {
        for x in 0..area.width {
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_symbol(" ");
                cell.set_style(style);
            }
        }
    }
}

fn draw_button(
    buf: &mut ratatui::buffer::Buffer,
    area: Rect,
    label: &str,
    bg: Color,
    fg: Color,
    underline: bool,
) {
    let style = if underline {
        Style::default()
            .bg(bg)
            .fg(fg)
            .add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default().bg(bg).fg(fg)
    };
    fill_rect(buf, area, Style::default().bg(bg));
    buf.set_line(
        area.x,
        area.y,
        &Line::from(vec![Span::styled(
            centered(label, area.width as usize),
            style,
        )]),
        area.width,
    );
}

fn gradient_text_line(text: &str, stops: &[Color], bg: Color) -> Line<'static> {
    let colors = blend_1d(text.chars().count().max(1), stops);
    Line::from(
        text.chars()
            .enumerate()
            .map(|(idx, ch)| Span::styled(ch.to_string(), Style::default().fg(colors[idx]).bg(bg)))
            .collect::<Vec<_>>(),
    )
}

fn center_line(line: Line<'static>, width: usize, bg: Color) -> Line<'static> {
    let used = line.width();
    if used >= width {
        return line;
    }
    let left = (width - used) / 2;
    let right = width - used - left;
    let mut spans = Vec::new();
    if left > 0 {
        spans.push(Span::styled(" ".repeat(left), Style::default().bg(bg)));
    }
    spans.extend(line.spans);
    if right > 0 {
        spans.push(Span::styled(" ".repeat(right), Style::default().bg(bg)));
    }
    Line::from(spans)
}

fn render_layout_list(frame: &mut Frame, area: Rect, title: &str, items: &[&str], done: &[usize]) {
    frame.buffer_mut().set_line(
        area.x,
        area.y,
        &Line::from(vec![Span::styled(
            title,
            Style::default().fg(Color::Indexed(255)),
        )]),
        area.width,
    );
    draw_horizontal_rule(
        frame.buffer_mut(),
        area.x,
        area.y + 1,
        area.width.saturating_sub(3),
        Color::Indexed(238),
    );
    for (idx, item) in items.iter().enumerate() {
        let y = area.y + 2 + idx as u16;
        let mark = if done.contains(&idx) { "✓" } else { " " };
        let item_style = if done.contains(&idx) {
            Style::default()
                .fg(Color::Indexed(240))
                .add_modifier(Modifier::CROSSED_OUT)
        } else {
            Style::default().fg(Color::Indexed(255))
        };
        frame.buffer_mut().set_line(
            area.x,
            y,
            &Line::from(vec![
                Span::styled(format!("{mark} "), Style::default().fg(Color::Indexed(48))),
                Span::styled((*item).to_string(), item_style),
            ]),
            area.width,
        );
    }
    draw_vertical_rule(
        frame.buffer_mut(),
        area.right().saturating_sub(1),
        area.y,
        area.height.saturating_sub(1),
        Color::Indexed(238),
    );
}

fn render_swatch_grid(frame: &mut Frame, area: Rect) {
    let grid = Rect::new(
        area.x + 2,
        area.y,
        area.width.saturating_sub(4),
        area.height.saturating_sub(1),
    );
    let colors = corner_blend_2d(
        grid.width as usize,
        grid.height as usize,
        Color::Indexed(205),
        Color::Indexed(228),
        Color::Indexed(63),
        Color::Indexed(51),
    );
    for y in 0..grid.height {
        for x in 0..grid.width {
            let idx = y as usize * grid.width as usize + x as usize;
            if let Some(cell) = frame.buffer_mut().cell_mut((grid.x + x, grid.y + y)) {
                cell.set_symbol(" ");
                cell.set_bg(colors[idx]);
            }
        }
    }
}

fn corner_blend_2d(
    width: usize,
    height: usize,
    top_left: Color,
    top_right: Color,
    bottom_left: Color,
    bottom_right: Color,
) -> Vec<Color> {
    let mut out = Vec::with_capacity(width.saturating_mul(height));
    if width == 0 || height == 0 {
        return out;
    }

    for y in 0..height {
        let ty = if height > 1 {
            y as f32 / (height - 1) as f32
        } else {
            0.0
        };
        let left = lerp_showcase_color(top_left, bottom_left, ty);
        let right = lerp_showcase_color(top_right, bottom_right, ty);
        for x in 0..width {
            let tx = if width > 1 {
                x as f32 / (width - 1) as f32
            } else {
                0.0
            };
            out.push(lerp_showcase_color(left, right, tx));
        }
    }

    out
}

fn lerp_showcase_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => {
            Color::Rgb(lerp_u8(fr, tr, t), lerp_u8(fg, tg, t), lerp_u8(fb, tb, t))
        }
        _ => {
            let (fr, fg, fb) = ratatui_glamour::color::color_to_rgb(from);
            let (tr, tg, tb) = ratatui_glamour::color::color_to_rgb(to);
            Color::Rgb(lerp_u8(fr, tr, t), lerp_u8(fg, tg, t), lerp_u8(fb, tb, t))
        }
    }
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t.clamp(0.0, 1.0)).round() as u8
}

fn render_paragraph_card(frame: &mut Frame, area: Rect, text: &str) {
    fill_rect(
        frame.buffer_mut(),
        area,
        Style::default().bg(Color::Indexed(99)),
    );
    let inner = Rect::new(
        area.x + 2,
        area.y + 1,
        area.width.saturating_sub(4),
        area.height.saturating_sub(2),
    );
    let wrapped = wrap_text_lines(text, inner.width as usize);
    for (idx, line) in wrapped.into_iter().take(inner.height as usize).enumerate() {
        frame.buffer_mut().set_line(
            inner.x,
            inner.y + idx as u16,
            &Line::from(vec![Span::styled(
                centered(&line, inner.width as usize),
                Style::default()
                    .fg(Color::Indexed(231))
                    .bg(Color::Indexed(99)),
            )]),
            inner.width,
        );
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    let left = Rect::new(area.x, area.y, 8, area.height);
    let center = Rect::new(
        area.x + 8,
        area.y,
        area.width.saturating_sub(28),
        area.height,
    );
    let right_a = Rect::new(area.right().saturating_sub(20), area.y, 8, area.height);
    let right_b = Rect::new(area.right().saturating_sub(12), area.y, 12, area.height);

    fill_rect(
        frame.buffer_mut(),
        left,
        Style::default().bg(Color::Indexed(204)),
    );
    fill_rect(
        frame.buffer_mut(),
        center,
        Style::default().bg(Color::Indexed(236)),
    );
    fill_rect(
        frame.buffer_mut(),
        right_a,
        Style::default().bg(Color::Indexed(99)),
    );
    fill_rect(
        frame.buffer_mut(),
        right_b,
        Style::default().bg(Color::Indexed(63)),
    );

    frame.buffer_mut().set_line(
        left.x + 1,
        left.y,
        &Line::from(vec![Span::styled(
            "STATUS",
            Style::default()
                .fg(Color::Indexed(231))
                .bg(Color::Indexed(204))
                .add_modifier(Modifier::BOLD),
        )]),
        left.width - 1,
    );
    frame.buffer_mut().set_line(
        center.x + 1,
        center.y,
        &Line::from(vec![Span::styled(
            "Ravishingly Dark!",
            Style::default()
                .fg(Color::Indexed(223))
                .bg(Color::Indexed(236)),
        )]),
        center.width - 2,
    );
    frame.buffer_mut().set_line(
        right_a.x + 1,
        right_a.y,
        &Line::from(vec![Span::styled(
            "UTF-8",
            Style::default()
                .fg(Color::Indexed(231))
                .bg(Color::Indexed(99)),
        )]),
        right_a.width - 1,
    );
    frame.buffer_mut().set_line(
        right_b.x + 1,
        right_b.y,
        &Line::from(vec![Span::styled(
            "⚙ Fish Cake",
            Style::default()
                .fg(Color::Indexed(231))
                .bg(Color::Indexed(63)),
        )]),
        right_b.width - 1,
    );
}

fn draw_vertical_rule(
    buf: &mut ratatui::buffer::Buffer,
    x: u16,
    y: u16,
    height: u16,
    color: Color,
) {
    for dy in 0..height {
        buf.set_string(x, y + dy, "│", Style::default().fg(color));
    }
}

fn wrap_text_lines(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if unicode_width::UnicodeWidthStr::width(candidate.as_str()) > width && !current.is_empty()
        {
            out.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

fn demo_compositor(area: Rect) -> Compositor {
    let field = Layer::from_lines(slash_field_lines(
        area.width.min(43),
        area.height.min(17),
        Color::Rgb(54, 54, 64),
    ))
    .id("field-a")
    .x(5)
    .y(2)
    .z(0);

    let pickles = Layer::from_lines(card_lines(
        "Pickles",
        Color::Indexed(255),
        Color::Indexed(234),
        &[
            Color::Indexed(205),
            Color::Indexed(99),
            Color::Indexed(51),
            Color::Indexed(99),
            Color::Indexed(205),
        ],
    ))
    .id("pickles")
    .x(4)
    .y(2)
    .z(1);

    let melon = Layer::from_lines(card_lines(
        "Bitter\nMelon",
        Color::Indexed(255),
        Color::Indexed(234),
        &[
            Color::Indexed(205),
            Color::Indexed(99),
            Color::Indexed(51),
            Color::Indexed(99),
            Color::Indexed(205),
        ],
    ))
    .id("melon")
    .x(22)
    .y(1)
    .z(0);

    let sriracha = Layer::from_lines(card_lines(
        "Sriracha",
        Color::Indexed(255),
        Color::Indexed(234),
        &[
            Color::Indexed(205),
            Color::Indexed(99),
            Color::Indexed(51),
            Color::Indexed(99),
            Color::Indexed(205),
        ],
    ))
    .id("sriracha")
    .x(11)
    .y(7)
    .z(0);

    let scene = field.add_layers([pickles, melon, sriracha]);
    Compositor::new([scene])
}

fn slash_field_lines(width: u16, height: u16, color: Color) -> Vec<Line<'static>> {
    let mut lines = Vec::with_capacity(height as usize);
    let style = Style::default().fg(color);
    for _ in 0..height {
        lines.push(Line::from(vec![Span::styled(
            "/".repeat(width as usize),
            style,
        )]));
    }
    lines
}

fn card_lines(title: &str, fg: Color, bg: Color, border_stops: &[Color]) -> Vec<Line<'static>> {
    let width = 16usize;
    let height = 9usize;
    let text_style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
    let mut lines =
        gradient_rounded_panel_lines(width, height, Style::default().bg(bg), border_stops);

    let title_lines: Vec<&str> = title.split('\n').collect();
    let start_row = 3 + (1usize.saturating_sub(title_lines.len().saturating_sub(1)));
    for (offset, line) in title_lines.iter().enumerate() {
        let text = centered(line, width - 2);
        let spans = &mut lines[start_row + offset].spans;
        for (idx, ch) in text.chars().enumerate() {
            spans[idx + 1] = Span::styled(ch.to_string(), text_style);
        }
    }

    lines
}

fn centered(text: &str, width: usize) -> String {
    let used = unicode_width::UnicodeWidthStr::width(text);
    if used >= width {
        return text.chars().take(width).collect();
    }
    let left = (width - used) / 2;
    let right = width - used - left;
    format!("{}{}{}", " ".repeat(left), text, " ".repeat(right))
}

fn paint_background(buf: &mut ratatui::buffer::Buffer, area: Rect) {
    let colors = blend_2d(
        area.width as usize,
        area.height as usize,
        24.0,
        &[
            Color::Rgb(12, 10, 32),
            Color::Rgb(32, 16, 64),
            Color::Rgb(10, 65, 91),
        ],
    );
    for y in 0..area.height {
        for x in 0..area.width {
            let idx = y as usize * area.width as usize + x as usize;
            if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) {
                cell.set_symbol(" ");
                cell.set_bg(colors[idx]);
            }
        }
    }
}
