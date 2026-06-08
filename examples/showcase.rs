use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
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
    surface::{centered_rect, gradient_rounded_panel_lines, place_with_pattern, render_classic_tabs_row, render_gradient_rounded_panel},
    table::{HEADER_ROW, Table},
    tree::{Tree, TreeNode, rounded_enumerator},
};

const TAB_TITLES: [&str; 6] = ["Gradients", "Borders", "Tree + List", "Table", "Layers", "Layout"];
const LAYOUT_WIDTH: u16 = 96;
const LAYOUT_HEIGHT: u16 = 52;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = ShowcaseApp::default();
    loop {
        terminal.draw(|frame| render_app(frame, &app))?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Tab | KeyCode::Char('l') => app.next_tab(),
                KeyCode::BackTab | KeyCode::Char('h') => app.prev_tab(),
                KeyCode::Up | KeyCode::Char('k') => app.up(),
                KeyCode::Down | KeyCode::Char('j') => app.down(),
                KeyCode::Left => app.left(),
                KeyCode::Right => app.right(),
                KeyCode::Char('w') => app.table_wrap = !app.table_wrap,
                _ => {}
            }
        }
    }
    Ok(())
}

#[derive(Default)]
struct ShowcaseApp {
    tab: usize,
    table_offset: usize,
    table_wrap: bool,
    layer_cursor_x: u16,
    layer_cursor_y: u16,
}

impl ShowcaseApp {
    fn next_tab(&mut self) {
        self.tab = (self.tab + 1) % TAB_TITLES.len();
    }

    fn prev_tab(&mut self) {
        self.tab = (self.tab + TAB_TITLES.len() - 1) % TAB_TITLES.len();
    }

    fn left(&mut self) {
        if self.tab == 4 {
            self.layer_cursor_x = self.layer_cursor_x.saturating_sub(1);
        } else {
            self.prev_tab();
        }
    }

    fn right(&mut self) {
        if self.tab == 4 {
            self.layer_cursor_x = self.layer_cursor_x.saturating_add(1);
        } else {
            self.next_tab();
        }
    }

    fn up(&mut self) {
        if self.tab == 3 {
            self.table_offset = self.table_offset.saturating_sub(1);
        } else if self.tab == 4 {
            self.layer_cursor_y = self.layer_cursor_y.saturating_sub(1);
        }
    }

    fn down(&mut self) {
        if self.tab == 3 {
            self.table_offset = self.table_offset.saturating_add(1);
        } else if self.tab == 4 {
            self.layer_cursor_y = self.layer_cursor_y.saturating_add(1);
        }
    }
}

fn render_app(frame: &mut Frame, app: &ShowcaseApp) {
    let area = frame.area();

    if app.tab == 5 {
        render_layout_demo(area, frame);
        return;
    }

    paint_background(frame.buffer_mut(), area);

    let outer = Block::default()
        .title(Line::from(vec![
            Span::styled("⚡ ", Style::default().fg(Color::Indexed(219)).add_modifier(Modifier::BOLD)),
            Span::styled("ratatui-glamour", Style::default().fg(Color::Indexed(255)).add_modifier(Modifier::BOLD)),
            Span::styled("  interactive showcase", Style::default().fg(Color::Indexed(189))),
        ]))
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(141)));
    let inner = outer.inner(area);
    outer.render(area, frame.buffer_mut());

    let [tabs_area, body_area, status_area]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(2)])
        .split(inner)
        .as_ref()
        .try_into()
        .unwrap();

    let titles: Vec<Line> = TAB_TITLES.iter().map(|title| Line::from(format!(" {title} "))).collect();
    Tabs::new(titles)
        .select(app.tab)
        .style(Style::default().fg(Color::Indexed(189)))
        .highlight_style(Style::default().fg(Color::Indexed(16)).bg(Color::Indexed(219)).add_modifier(Modifier::BOLD))
        .divider("")
        .render(tabs_area, frame.buffer_mut());

    match app.tab {
        0 => render_gradients(body_area, frame),
        1 => render_borders(body_area, frame),
        2 => render_tree_list(body_area, frame),
        3 => render_table(body_area, frame, app),
        4 => render_layers(body_area, frame, app),
        _ => {}
    }

    let help = match app.tab {
        3 => "Tab/Shift-Tab tabs  ↑/↓ table scroll  w wrap  q quit",
        4 => "Tab/Shift-Tab tabs  ←/→/↑/↓ move hit cursor  q quit",
        _ => "Tab/Shift-Tab tabs  q quit",
    };
    Paragraph::new(help)
        .style(Style::default().fg(Color::Indexed(255)).bg(Color::Indexed(54)).add_modifier(Modifier::BOLD))
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
        blend_1d(area.width as usize, &[Color::Rgb(255, 0, 102), Color::Rgb(255, 178, 0), Color::Rgb(255, 255, 255)]),
        blend_1d(area.width as usize, &[Color::Rgb(0, 194, 255), Color::Rgb(87, 255, 180), Color::Rgb(234, 255, 128)]),
        blend_1d(area.width as usize, &[Color::Rgb(128, 0, 255), Color::Rgb(255, 0, 255), Color::Rgb(255, 128, 215)]),
    ];
    for (row, ramp) in ramps.iter().enumerate() {
        if row as u16 >= area.height {
            break;
        }
        for (col, color) in ramp.iter().enumerate().take(area.width as usize) {
            if let Some(cell) = frame.buffer_mut().cell_mut((area.x + col as u16, area.y + row as u16)) {
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
        &[Color::Rgb(13, 15, 40), Color::Rgb(92, 27, 153), Color::Rgb(0, 215, 255), Color::Rgb(255, 95, 175)],
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
            .constraints([Constraint::Ratio(1, cols), Constraint::Ratio(1, cols), Constraint::Ratio(1, cols)])
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
            Paragraph::new(format!("{}\n{}\n{}", "⚡ powerline-safe", "🭬 unicode-ready", "◉ table junctions later"))
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
        .enumerator_style(Style::default().fg(Color::Indexed(219)).add_modifier(Modifier::BOLD))
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
        .enumerator_style(Style::default().fg(Color::Indexed(81)).add_modifier(Modifier::BOLD))
        .item_style(Style::default().fg(Color::Indexed(230)))
        .items([
            ListItem::from("port border presets"),
            ListItem::from("port tree widget"),
            ListItem::from(vec![ListItem::from("multiline items"), ListItem::from("table renderer")]),
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
        .root_style(Style::default().fg(Color::Indexed(230)).add_modifier(Modifier::BOLD))
        .enumerator(rounded_enumerator)
        .enumerator_style(Style::default().fg(Color::Indexed(213)).add_modifier(Modifier::BOLD))
        .indenter_style(Style::default().fg(Color::Indexed(99)))
        .item_style_fn(|_, idx| if idx % 2 == 0 { Style::default().fg(Color::Indexed(255)) } else { Style::default().fg(Color::Indexed(189)) })
        .children([
            TreeNode::new("border/mod.rs").child(TreeNode::new("preset mapping + ratatui adapter")),
            TreeNode::new("list/mod.rs").children([TreeNode::new("arabic"), TreeNode::new("roman"), TreeNode::new("unicode bullets ⚡")]),
            TreeNode::new("tree/mod.rs").children([TreeNode::new("multiline values\nwith prefix continuation"), TreeNode::new("styles + enumerators")]),
            TreeNode::new("table/mod.rs").child(TreeNode::new("planner + custom grid renderer")),
        ]);
    (&tree).render(tree_inner, frame.buffer_mut());
}

fn render_table(area: Rect, frame: &mut Frame, app: &ShowcaseApp) {
    let block = Block::default()
        .title(" Table ")
        .borders(Borders::ALL)
        .border_set(Border::rounded().into_border_set())
        .border_style(Style::default().fg(Color::Indexed(117)));
    let inner = block.inner(area);
    block.render(area, frame.buffer_mut());

    let table = Table::new()
        .headers(["Module", "Status", "Notes"])
        .rows([
            ["border/mod.rs", "done", "Preset catalog plus adapter into ratatui border sets."],
            ["color.rs", "done", "1D and 2D blend helpers for candy-ramp gradients."],
            ["tree/mod.rs", "done", "Multiline values now continue under the correct branch prefix."],
            ["list/mod.rs", "done", "Arabic, roman, bullets, nested items, Unicode-safe output."],
            ["ansi.rs", "done", "ANSI exporter preserves fg/bg and wide icon widths for demos."],
            ["table/mod.rs", "live", "Width planning, median-based shrink, overflow row, custom border grid renderer."],
            ["showcase.rs", "live", "Interactive gallery instead of dumping a buffer and dying instantly."],
            ["next", "pending", "Expose more style/padding semantics once higher-level glam widgets settle."],
        ])
        .border(Border::double())
        .border_style(Style::default().fg(Color::Indexed(213)))
        .style_fn(|row, col| match row {
            HEADER_ROW => Style::default().fg(Color::Indexed(16)).bg(Color::Indexed(219)).add_modifier(Modifier::BOLD),
            _ if col == 1 => {
                let tone = match row {
                    0..=4 => Color::Indexed(118),
                    5..=6 => Color::Indexed(81),
                    _ => Color::Indexed(220),
                };
                Style::default().fg(tone).add_modifier(Modifier::BOLD)
            }
            _ => Style::default().fg(Color::Indexed(255)),
        })
        .y_offset(app.table_offset)
        .wrap(app.table_wrap);
    (&table).render(inner, frame.buffer_mut());
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
    fill_rect(frame.buffer_mut(), scene_inner, Style::default().bg(Color::Indexed(234)));

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
    fill_rect(frame.buffer_mut(), area, Style::default().bg(Color::Indexed(234)));
    let page = centered_rect(area, LAYOUT_WIDTH.min(area.width.saturating_sub(2)), LAYOUT_HEIGHT.min(area.height.saturating_sub(2)));
    fill_rect(frame.buffer_mut(), page, Style::default().bg(Color::Indexed(234)));
    let [top, middle, bottom]: [Rect; 3] = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(11), Constraint::Length(17), Constraint::Length(24)])
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
        Style::default().fg(Color::Indexed(99)).bg(Color::Indexed(234)),
        Style::default().fg(Color::Indexed(255)).bg(Color::Indexed(234)),
        Style::default().fg(Color::Indexed(255)).bg(Color::Indexed(234)),
    );

    let title_stack_x = area.x + 1;
    let title_stack_y = area.y + 5;
    let stack_colors = blend_1d(5, &[Color::Indexed(205), Color::Indexed(63)]);
    for (idx, color) in stack_colors.iter().enumerate() {
        let rect = Rect::new(title_stack_x + idx as u16 * 2, title_stack_y + idx as u16, 11, 1);
        fill_rect(frame.buffer_mut(), rect, Style::default().bg(*color));
        frame.buffer_mut().set_line(
            rect.x,
            rect.y,
            &Line::from(vec![Span::styled(" Glamour ", Style::default().fg(Color::Indexed(231)).bg(*color).add_modifier(Modifier::ITALIC))]),
            rect.width,
        );
    }

    let desc_x = title_stack_x + 18;
    let desc_y = title_stack_y + 1;
    frame.buffer_mut().set_line(
        desc_x,
        desc_y,
        &Line::from(vec![Span::styled("Style Definitions for Nice Terminal Layouts", Style::default().fg(Color::Indexed(255)))]),
        area.width.saturating_sub(desc_x - area.x),
    );
    draw_horizontal_rule(frame.buffer_mut(), desc_x, desc_y + 1, area.right().saturating_sub(3) - desc_x, Color::Indexed(238));
    frame.buffer_mut().set_line(
        desc_x,
        desc_y + 2,
        &Line::from(vec![
            Span::styled("From Charm", Style::default().fg(Color::Indexed(250))),
            Span::styled(" • ", Style::default().fg(Color::Indexed(238))),
            Span::styled("https://github.com/tinythings/ratatui-glamour", Style::default().fg(Color::Indexed(48))),
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

    let popup_region = Rect::new(dialog_area.x, dialog_area.y, dialog_area.width, 9.min(dialog_area.height));
    let dialog = place_with_pattern(
        frame.buffer_mut(),
        popup_region,
        52,
        6,
        "猫咪",
        Style::default().fg(Color::Indexed(236)).bg(Color::Indexed(234)),
    );

    let inner = render_gradient_rounded_panel(
        frame.buffer_mut(),
        dialog,
        Style::default().bg(Color::Indexed(234)),
        &[Color::Indexed(205), Color::Indexed(99), Color::Indexed(51), Color::Indexed(99), Color::Indexed(205)],
    );
    let headline = gradient_text_line(
        "Are you sure you want to eat marmalade?",
        &[Color::Indexed(229), Color::Indexed(221), Color::Indexed(216), Color::Indexed(210), Color::Indexed(204)],
        Color::Indexed(234),
    );
    let content_w = 50u16.min(inner.width);
    let content_x = inner.x + inner.width.saturating_sub(content_w) / 2;
    frame.buffer_mut().set_line(content_x, inner.y + 1, &center_line(headline, content_w as usize, Color::Indexed(234)), content_w);

    let yes_w = 9u16;
    let maybe_w = 11u16;
    let gap = 2u16;
    let buttons_w = yes_w + gap + maybe_w;
    let buttons_x = inner.x + inner.width.saturating_sub(buttons_w) / 2;
    draw_button(frame.buffer_mut(), Rect::new(buttons_x, inner.y + 3, yes_w, 1), "Yes", Color::Indexed(205), Color::Indexed(231), true);
    draw_button(frame.buffer_mut(), Rect::new(buttons_x + yes_w + gap, inner.y + 3, maybe_w, 1), "Maybe", Color::Indexed(246), Color::Indexed(231), false);

    let [left, center, right]: [Rect; 3] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .split(columns_area)
        .as_ref()
        .try_into()
        .unwrap();
    render_layout_list(frame, left, "Citrus Fruits to Try", &["Grapefruit", "Yuzu", "Citron", "Kumquat", "Pomelo"], &[0, 1]);
    render_layout_list(frame, center, "Actual Glamour Vendors", &["Glossier", "Claire's Boutique", "Nyx", "Mac", "Milk"], &[2, 4]);
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
        .constraints([Constraint::Ratio(1, 3), Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)])
        .spacing(2)
        .split(body)
        .as_ref()
        .try_into()
        .unwrap();
    render_paragraph_card(frame, c1, "The Romans learned from the Greeks that quinces slowly cooked with honey would 'set' when cool. The Apicius gives a recipe for preserving whole quinces, stems and leaves attached, in a bath of honey diluted with defrutum.");
    render_paragraph_card(frame, c2, "Medieval quince preserves, which went by the French name cotignac, produced in a clear version and a fruit pulp version, began to lose their medieval seasoning of spices in the 16th century.");
    render_paragraph_card(frame, c3, "In 1524, Henry VIII, King of England, received a 'box of marmalade' from Mr. Hull of Exeter. This was probably marmelada, a solid quince paste from Portugal.");
    let badge = Rect::new(c2.right().saturating_sub(7), c2.bottom().saturating_sub(3), 28, 2);
    fill_rect(frame.buffer_mut(), badge, Style::default().bg(Color::Indexed(204)));
    frame.buffer_mut().set_line(
        badge.x + 3,
        badge.y,
        &Line::from(vec![Span::styled("Now with Compositing!", Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(204)).add_modifier(Modifier::ITALIC))]),
        badge.width - 6,
    );
    render_status_bar(frame, status);
}

fn draw_tab(buf: &mut ratatui::buffer::Buffer, area: Rect, title: &str, active: bool) {
    let color = Color::Indexed(99);
    let border_style = Style::default().fg(color).bg(Color::Indexed(234));
    buf.set_string(area.x, area.y, "╭", border_style);
    for dx in 1..area.width - 1 { buf.set_string(area.x + dx, area.y, "─", border_style); }
    buf.set_string(area.right() - 1, area.y, "╮", border_style);
    buf.set_string(area.x, area.y + 1, "│", border_style);
    buf.set_string(area.right() - 1, area.y + 1, "│", border_style);
    let centered_title = centered(title, area.width as usize - 2);
    if active {
        let label = format!(" {} ", title);
        let label_w = unicode_width::UnicodeWidthStr::width(label.as_str()) as u16;
        let label_x = area.x + 1 + ((area.width.saturating_sub(2)).saturating_sub(label_w)) / 2;
        buf.set_line(area.x + 1, area.y + 1, &Line::from(vec![Span::styled(centered_title, Style::default().fg(Color::Indexed(255)).bg(Color::Indexed(234)))]), area.width - 2);
        buf.set_line(
            label_x,
            area.y + 1,
            &Line::from(vec![Span::styled(label, Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(205)).add_modifier(Modifier::ITALIC))]),
            label_w,
        );
    } else {
        buf.set_line(area.x + 1, area.y + 1, &Line::from(vec![Span::styled(centered_title, Style::default().fg(Color::Indexed(255)).bg(Color::Indexed(234)))]), area.width - 2);
    }
    if active {
        buf.set_string(area.x, area.y + 2, "┘", border_style);
        for dx in 1..area.width - 1 { buf.set_string(area.x + dx, area.y + 2, " ", Style::default().bg(Color::Indexed(234))); }
        buf.set_string(area.right() - 1, area.y + 2, "└", border_style);
    } else {
        buf.set_string(area.x, area.y + 2, "┴", border_style);
        for dx in 1..area.width - 1 { buf.set_string(area.x + dx, area.y + 2, "─", border_style); }
        buf.set_string(area.right() - 1, area.y + 2, "┴", border_style);
    }
}

fn draw_horizontal_rule(buf: &mut ratatui::buffer::Buffer, x: u16, y: u16, width: u16, color: Color) {
    for dx in 0..width { buf.set_string(x + dx, y, "─", Style::default().fg(color)); }
}

fn fill_rect(buf: &mut ratatui::buffer::Buffer, area: Rect, style: Style) {
    for y in 0..area.height { for x in 0..area.width { if let Some(cell) = buf.cell_mut((area.x + x, area.y + y)) { cell.set_symbol(" "); cell.set_style(style); } } }
}

fn draw_button(buf: &mut ratatui::buffer::Buffer, area: Rect, label: &str, bg: Color, fg: Color, underline: bool) {
    let style = if underline { Style::default().bg(bg).fg(fg).add_modifier(Modifier::UNDERLINED) } else { Style::default().bg(bg).fg(fg) };
    fill_rect(buf, area, Style::default().bg(bg));
    buf.set_line(area.x, area.y, &Line::from(vec![Span::styled(centered(label, area.width as usize), style)]), area.width);
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
    frame.buffer_mut().set_line(area.x, area.y, &Line::from(vec![Span::styled(title, Style::default().fg(Color::Indexed(255)))]), area.width);
    draw_horizontal_rule(frame.buffer_mut(), area.x, area.y + 1, area.width.saturating_sub(3), Color::Indexed(238));
    for (idx, item) in items.iter().enumerate() {
        let y = area.y + 2 + idx as u16;
        let mark = if done.contains(&idx) { "✓" } else { " " };
        let item_style = if done.contains(&idx) { Style::default().fg(Color::Indexed(240)).add_modifier(Modifier::CROSSED_OUT) } else { Style::default().fg(Color::Indexed(255)) };
        frame.buffer_mut().set_line(
            area.x,
            y,
            &Line::from(vec![Span::styled(format!("{mark} "), Style::default().fg(Color::Indexed(48))), Span::styled((*item).to_string(), item_style)]),
            area.width,
        );
    }
    draw_vertical_rule(frame.buffer_mut(), area.right().saturating_sub(1), area.y, area.height.saturating_sub(1), Color::Indexed(238));
}

fn render_swatch_grid(frame: &mut Frame, area: Rect) {
    let grid = Rect::new(area.x + 2, area.y, area.width.saturating_sub(4), area.height.saturating_sub(1));
    let colors = corner_blend_2d(
        grid.width as usize,
        grid.height as usize,
        Color::Indexed(205),
        Color::Indexed(228),
        Color::Indexed(63),
        Color::Indexed(51),
    );
    for y in 0..grid.height { for x in 0..grid.width { let idx = y as usize * grid.width as usize + x as usize; if let Some(cell) = frame.buffer_mut().cell_mut((grid.x + x, grid.y + y)) { cell.set_symbol(" "); cell.set_bg(colors[idx]); } } }
}

fn corner_blend_2d(width: usize, height: usize, top_left: Color, top_right: Color, bottom_left: Color, bottom_right: Color) -> Vec<Color> {
    let mut out = Vec::with_capacity(width.saturating_mul(height));
    if width == 0 || height == 0 {
        return out;
    }

    for y in 0..height {
        let ty = if height > 1 { y as f32 / (height - 1) as f32 } else { 0.0 };
        let left = lerp_showcase_color(top_left, bottom_left, ty);
        let right = lerp_showcase_color(top_right, bottom_right, ty);
        for x in 0..width {
            let tx = if width > 1 { x as f32 / (width - 1) as f32 } else { 0.0 };
            out.push(lerp_showcase_color(left, right, tx));
        }
    }

    out
}

fn lerp_showcase_color(from: Color, to: Color, t: f32) -> Color {
    match (from, to) {
        (Color::Rgb(fr, fg, fb), Color::Rgb(tr, tg, tb)) => Color::Rgb(
            lerp_u8(fr, tr, t),
            lerp_u8(fg, tg, t),
            lerp_u8(fb, tb, t),
        ),
        _ => {
            let (fr, fg, fb) = ratatui_glamour::color::color_to_rgb(from);
            let (tr, tg, tb) = ratatui_glamour::color::color_to_rgb(to);
            Color::Rgb(
                lerp_u8(fr, tr, t),
                lerp_u8(fg, tg, t),
                lerp_u8(fb, tb, t),
            )
        }
    }
}

fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t.clamp(0.0, 1.0)).round() as u8
}

fn render_paragraph_card(frame: &mut Frame, area: Rect, text: &str) {
    fill_rect(frame.buffer_mut(), area, Style::default().bg(Color::Indexed(99)));
    let inner = Rect::new(area.x + 2, area.y + 1, area.width.saturating_sub(4), area.height.saturating_sub(2));
    let wrapped = wrap_text_lines(text, inner.width as usize);
    for (idx, line) in wrapped.into_iter().take(inner.height as usize).enumerate() {
        frame.buffer_mut().set_line(inner.x, inner.y + idx as u16, &Line::from(vec![Span::styled(centered(&line, inner.width as usize), Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(99)))]), inner.width);
    }
}

fn render_status_bar(frame: &mut Frame, area: Rect) {
    let left = Rect::new(area.x, area.y, 8, area.height);
    let center = Rect::new(area.x + 8, area.y, area.width.saturating_sub(28), area.height);
    let right_a = Rect::new(area.right().saturating_sub(20), area.y, 8, area.height);
    let right_b = Rect::new(area.right().saturating_sub(12), area.y, 12, area.height);

    fill_rect(frame.buffer_mut(), left, Style::default().bg(Color::Indexed(204)));
    fill_rect(frame.buffer_mut(), center, Style::default().bg(Color::Indexed(236)));
    fill_rect(frame.buffer_mut(), right_a, Style::default().bg(Color::Indexed(99)));
    fill_rect(frame.buffer_mut(), right_b, Style::default().bg(Color::Indexed(63)));

    frame.buffer_mut().set_line(left.x + 1, left.y, &Line::from(vec![Span::styled("STATUS", Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(204)).add_modifier(Modifier::BOLD))]), left.width - 1);
    frame.buffer_mut().set_line(center.x + 1, center.y, &Line::from(vec![Span::styled("Ravishingly Dark!", Style::default().fg(Color::Indexed(223)).bg(Color::Indexed(236)))]), center.width - 2);
    frame.buffer_mut().set_line(right_a.x + 1, right_a.y, &Line::from(vec![Span::styled("UTF-8", Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(99)))]), right_a.width - 1);
    frame.buffer_mut().set_line(right_b.x + 1, right_b.y, &Line::from(vec![Span::styled("⚙ Fish Cake", Style::default().fg(Color::Indexed(231)).bg(Color::Indexed(63)))]), right_b.width - 1);
}

fn draw_vertical_rule(buf: &mut ratatui::buffer::Buffer, x: u16, y: u16, height: u16, color: Color) {
    for dy in 0..height { buf.set_string(x, y + dy, "│", Style::default().fg(color)); }
}

fn wrap_text_lines(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        if unicode_width::UnicodeWidthStr::width(candidate.as_str()) > width && !current.is_empty() {
            out.push(current);
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() { out.push(current); }
    out
}

fn demo_compositor(area: Rect) -> Compositor {
    let field = Layer::from_lines(slash_field_lines(area.width.min(43), area.height.min(17), Color::Rgb(54, 54, 64)))
        .id("field-a")
        .x(5)
        .y(2)
        .z(0);

    let pickles = Layer::from_lines(card_lines(
        "Pickles",
        Color::Indexed(255),
        Color::Indexed(234),
        &[Color::Indexed(205), Color::Indexed(99), Color::Indexed(51), Color::Indexed(99), Color::Indexed(205)],
    ))
    .id("pickles")
    .x(4)
    .y(2)
    .z(1);

    let melon = Layer::from_lines(card_lines(
        "Bitter\nMelon",
        Color::Indexed(255),
        Color::Indexed(234),
        &[Color::Indexed(205), Color::Indexed(99), Color::Indexed(51), Color::Indexed(99), Color::Indexed(205)],
    ))
    .id("melon")
    .x(22)
    .y(1)
    .z(0);

    let sriracha = Layer::from_lines(card_lines(
        "Sriracha",
        Color::Indexed(255),
        Color::Indexed(234),
        &[Color::Indexed(205), Color::Indexed(99), Color::Indexed(51), Color::Indexed(99), Color::Indexed(205)],
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
        lines.push(Line::from(vec![Span::styled("/".repeat(width as usize), style)]));
    }
    lines
}

fn card_lines(title: &str, fg: Color, bg: Color, border_stops: &[Color]) -> Vec<Line<'static>> {
    let width = 16usize;
    let height = 9usize;
    let text_style = Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD);
    let mut lines = gradient_rounded_panel_lines(width, height, Style::default().bg(bg), border_stops);

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
        &[Color::Rgb(12, 10, 32), Color::Rgb(32, 16, 64), Color::Rgb(10, 65, 91)],
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
