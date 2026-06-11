# ratatui-glamour

`ratatui-glamour` is a Rust terminal UI toolkit inspired by [Lip Gloss](https://github.com/charmbracelet/lipgloss), built on top of [`ratatui`](https://github.com/ratatui/ratatui).

It focuses on the parts that make Lip Gloss fun to use in a TUI: expressive borders, gradients, layered composition, tree and list rendering, and opinionated text-mode surfaces that look good with very little code.

This crate is not a 1:1 port of Lip Gloss. It is a Rust-native, `ratatui`-friendly take on the same style of terminal rendering.

## Features

- Border presets including rounded, thick, double, ASCII, markdown, and block variants
- Gradient helpers for 1D ramps and 2D color meshes
- Tree and list widgets with custom enumerators and styling
- A table widget with wrapping, scrolling, and per-cell styling hooks
- Layered composition with hit-testing and ANSI/text export
- Surface helpers for centered panels, patterned fills, tabs, and rounded gradient panels
- Bubble-style stateful widgets for keys, help, spinners, paginators, viewports, inputs, and lists

## Screenshots

### Layout
<img width="1141" height="1020" alt="Screenshot from 2026-06-08 20-15-20" src="https://github.com/user-attachments/assets/152417de-ec6c-4667-ae77-0f5ebc46ebdd" />

### Compositor
<img width="1203" height="1042" alt="Screenshot from 2026-06-08 20-16-26" src="https://github.com/user-attachments/assets/9d9ea7da-4b41-44b2-ad75-b32e3597f445" />

## Installation

```toml
[dependencies]
ratatui = "0.30"
ratatui-glamour = "0.1"
```

## Quick Example

```rust
use std::io;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders},
};
use ratatui_glamour::{
    border::Border,
    list::{List, bullet},
    table::Table,
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = terminal.draw(render);
    ratatui::restore();
    result.map(|_| ())
}

fn render(frame: &mut Frame) {
    let area = frame.area();

    let list = List::new()
        .enumerator(bullet)
        .enumerator_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .item_style(Style::default().fg(Color::White))
        .items([
            "Rounded borders",
            "Gradient helpers",
            "Layer composition",
            "Tree and table widgets",
        ]);

    let table = Table::new()
        .headers(["Widget", "Status"])
        .rows([
            ["List", "ready"],
            ["Tree", "ready"],
            ["Table", "ready"],
            ["Canvas", "ready"],
        ])
        .border(Border::rounded())
        .border_style(Style::default().fg(Color::Magenta))
        .style_fn(|row, _| {
            if row == -1 {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            }
        });

    frame.render_widget(
        Block::default()
            .title(" ratatui-glamour ")
            .borders(Borders::ALL)
            .border_set(Border::double().into_border_set())
            .border_style(Style::default().fg(Color::Blue)),
        area,
    );

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .margin(1)
        .split(area);

    frame.render_widget(&list, inner[0]);
    frame.render_widget(&table, inner[1]);
}
```

## Modules

- `border`: reusable border presets and conversion into `ratatui` border sets
- `color`: RGB interpolation and terminal-friendly gradient generation
- `canvas`: off-screen drawing, compositing, layering, hit-testing, ANSI export
- `list`: list rendering built on the tree renderer with bullet, roman, arabic, and alphabet enumerators
- `rule`: gradient title rules and slash-fill separators
- `surface`: rounded panels, centered layouts, text patterns, and classic tabs
- `table`: a text table widget with wrapping, sizing, borders, and style callbacks
- `tree`: styled tree rendering with custom branch and indent behavior
- `widgets`: Bubble-style stateful widgets ported for ratatui, including key maps, help views, spinners, paginators, viewports, text inputs, text areas, and lists

## Showcase

Run the interactive demo:

```bash
cargo run --example showcase
```

Run the widget sampler:

```bash
cargo run --example widgets
```

The showcase includes:

- gradient ramps and 2D meshes
- border variants
- list and tree rendering
- table layout behavior
- layered composition and hit-testing
- surface and layout helpers

## Design Goals

- Feel pleasant and expressive for terminal styling work
- Stay close to `ratatui` instead of wrapping it away
- Prefer composable primitives over heavy framework abstractions
- Make good-looking terminal output easy to build from plain Rust

## Status

The crate is early-stage, but the public modules already cover a useful set of styling and rendering primitives. Expect the API to evolve as the Lip Gloss-inspired surface area grows.

## License

MIT
