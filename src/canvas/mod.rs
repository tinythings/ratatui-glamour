use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use unicode_width::UnicodeWidthStr;

use crate::ansi;

pub trait Drawable {
    fn draw(&self, canvas: &mut Canvas, area: Rect);
}

#[derive(Clone)]
pub struct Canvas {
    area: Rect,
    buf: Buffer,
}

impl Canvas {
    pub fn new(width: u16, height: u16) -> Self {
        let area = Rect::new(0, 0, width, height);
        Self { area, buf: Buffer::empty(area) }
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.area = Rect::new(0, 0, width, height);
        self.buf = Buffer::empty(self.area);
    }

    pub fn clear(&mut self) {
        self.buf = Buffer::empty(self.area);
    }

    pub fn width(&self) -> u16 {
        self.area.width
    }

    pub fn height(&self) -> u16 {
        self.area.height
    }

    pub fn area(&self) -> Rect {
        self.area
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buf
    }

    pub fn buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buf
    }

    pub fn cell(&self, x: u16, y: u16) -> Option<&ratatui::buffer::Cell> {
        self.buf.cell((x, y))
    }

    pub fn cell_mut(&mut self, x: u16, y: u16) -> Option<&mut ratatui::buffer::Cell> {
        self.buf.cell_mut((x, y))
    }

    pub fn compose<D: Drawable>(&mut self, drawable: &D) -> &mut Self {
        drawable.draw(self, self.area);
        self
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for y in 0..self.area.height {
            let mut line = String::new();
            for x in 0..self.area.width {
                let symbol = self.buf.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ");
                line.push_str(symbol);
            }
            while line.ends_with(' ') {
                line.pop();
            }
            out.push_str(&line);
            if y + 1 < self.area.height {
                out.push('\n');
            }
        }
        out
    }

    pub fn render_ansi(&self) -> String {
        ansi::render_buffer(&self.buf, self.area)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Layer {
    id: String,
    content: Vec<Line<'static>>,
    x: i32,
    y: i32,
    z: i32,
    layers: Vec<Layer>,
}

impl Layer {
    pub fn new(content: impl Into<String>, layers: impl IntoIterator<Item = Layer>) -> Self {
        Self::from_text(content).add_layers(layers)
    }

    pub fn from_text(content: impl Into<String>) -> Self {
        let text: String = content.into();
        let lines = if text.is_empty() { vec![Line::default()] } else { text.split('\n').map(|line| Line::raw(line.to_string())).collect() };
        Self { id: String::new(), content: lines, x: 0, y: 0, z: 0, layers: Vec::new() }
    }

    pub fn from_lines(content: Vec<Line<'static>>) -> Self {
        Self { id: String::new(), content, x: 0, y: 0, z: 0, layers: Vec::new() }
    }

    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.content = self.content.into_iter().map(|line| patch_line_style(line, style)).collect();
        self
    }

    pub fn x(mut self, x: i32) -> Self {
        self.x = x;
        self
    }

    pub fn y(mut self, y: i32) -> Self {
        self.y = y;
        self
    }

    pub fn z(mut self, z: i32) -> Self {
        self.z = z;
        self
    }

    pub fn add_layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn add_layers(mut self, layers: impl IntoIterator<Item = Layer>) -> Self {
        self.layers.extend(layers);
        self
    }

    pub fn content(&self) -> &[Line<'static>] {
        &self.content
    }

    pub fn id_ref(&self) -> &str {
        &self.id
    }

    pub fn children(&self) -> &[Layer] {
        &self.layers
    }

    pub fn x_pos(&self) -> i32 { self.x }
    pub fn y_pos(&self) -> i32 { self.y }
    pub fn z_pos(&self) -> i32 { self.z }

    pub fn width(&self) -> u16 {
        let self_width = self.content.iter().map(line_width).max().unwrap_or(0) as i32;
        let mut max_right = self_width;
        for child in &self.layers {
            max_right = max_right.max(child.x + child.width() as i32);
        }
        max_right.max(0) as u16
    }

    pub fn height(&self) -> u16 {
        let self_height = self.content.len() as i32;
        let mut max_bottom = self_height;
        for child in &self.layers {
            max_bottom = max_bottom.max(child.y + child.height() as i32);
        }
        max_bottom.max(0) as u16
    }

    pub fn bounds_with_offset(&self, parent_x: i32, parent_y: i32) -> Rect {
        let abs_x = self.x + parent_x;
        let abs_y = self.y + parent_y;
        let mut left = abs_x;
        let mut top = abs_y;
        let mut right = abs_x + self.width() as i32;
        let mut bottom = abs_y + self.height() as i32;
        for child in &self.layers {
            let cb = child.bounds_with_offset(abs_x, abs_y);
            left = left.min(cb.x as i32);
            top = top.min(cb.y as i32);
            right = right.max(cb.right() as i32);
            bottom = bottom.max(cb.bottom() as i32);
        }
        Rect::new(left.max(0) as u16, top.max(0) as u16, right.saturating_sub(left).max(0) as u16, bottom.saturating_sub(top).max(0) as u16)
    }

    pub fn get_layer(&self, id: &str) -> Option<&Layer> {
        if id.is_empty() {
            return None;
        }
        if self.id == id {
            return Some(self);
        }
        self.layers.iter().find_map(|child| child.get_layer(id))
    }

    pub fn max_z(&self) -> i32 {
        self.layers.iter().fold(self.z, |acc, child| acc.max(child.max_z()))
    }
}

impl Drawable for Layer {
    fn draw(&self, canvas: &mut Canvas, area: Rect) {
        draw_layer_content(self, canvas.buffer_mut(), area, 0, 0);
    }
}

#[derive(Clone, Debug, Default)]
pub struct LayerHit {
    pub id: String,
    pub bounds: Option<Rect>,
}

impl LayerHit {
    pub fn empty(&self) -> bool {
        self.id.is_empty()
    }
}

#[derive(Clone, Debug)]
struct CompositeLayer {
    layer: Layer,
    abs_x: i32,
    abs_y: i32,
    bounds: Rect,
}

#[derive(Clone, Debug, Default)]
pub struct Compositor {
    root: Layer,
    layers: Vec<CompositeLayer>,
    bounds: Rect,
}

impl Compositor {
    pub fn new(layers: impl IntoIterator<Item = Layer>) -> Self {
        let root = Layer::from_text("").add_layers(layers);
        let mut this = Self { root, layers: Vec::new(), bounds: Rect::default() };
        this.refresh();
        this
    }

    pub fn add_layers(&mut self, layers: impl IntoIterator<Item = Layer>) {
        self.root.layers.extend(layers);
        self.refresh();
    }

    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn get_layer(&self, id: &str) -> Option<&Layer> {
        self.root.get_layer(id)
    }

    pub fn refresh(&mut self) {
        self.layers.clear();
        flatten_recursive(&self.root, 0, 0, &mut self.layers);
        self.layers.sort_by_key(|layer| layer.layer.z);
        self.bounds = self.layers.iter().fold(Rect::default(), |acc, layer| union_rect(acc, layer.bounds));
    }

    pub fn hit(&self, x: u16, y: u16) -> LayerHit {
        let point = Position::new(x, y);
        for layer in self.layers.iter().rev() {
            if !layer.layer.id.is_empty() && rect_contains(layer.bounds, point) {
                return LayerHit { id: layer.layer.id.clone(), bounds: Some(layer.bounds) };
            }
        }
        LayerHit::default()
    }

    pub fn render_text(&self) -> String {
        let mut canvas = Canvas::new(self.bounds.width.max(1), self.bounds.height.max(1));
        canvas.compose(self);
        canvas.render_text()
    }

    pub fn render_ansi(&self) -> String {
        let mut canvas = Canvas::new(self.bounds.width.max(1), self.bounds.height.max(1));
        canvas.compose(self);
        canvas.render_ansi()
    }
}

impl Drawable for Compositor {
    fn draw(&self, canvas: &mut Canvas, area: Rect) {
        for layer in &self.layers {
            if rects_overlap(layer.bounds, area) {
                draw_layer_content(&layer.layer, canvas.buffer_mut(), area, layer.abs_x, layer.abs_y);
            }
        }
    }
}

impl Widget for &Compositor {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for layer in &self.layers {
            if rects_overlap(layer.bounds, area) {
                draw_layer_content(&layer.layer, buf, area, layer.abs_x + area.x as i32, layer.abs_y + area.y as i32);
            }
        }
    }
}

fn flatten_recursive(layer: &Layer, parent_x: i32, parent_y: i32, out: &mut Vec<CompositeLayer>) {
    let abs_x = layer.x + parent_x;
    let abs_y = layer.y + parent_y;
    let bounds = Rect::new(abs_x.max(0) as u16, abs_y.max(0) as u16, layer.width(), layer.height());
    out.push(CompositeLayer { layer: layer.clone(), abs_x, abs_y, bounds });
    for child in &layer.layers {
        flatten_recursive(child, abs_x, abs_y, out);
    }
}

fn draw_layer_content(layer: &Layer, buf: &mut Buffer, clip: Rect, abs_x: i32, abs_y: i32) {
    for (row_idx, line) in layer.content.iter().enumerate() {
        let y = abs_y + row_idx as i32;
        if y < clip.y as i32 || y >= clip.bottom() as i32 {
            continue;
        }
        if abs_x >= clip.right() as i32 {
            continue;
        }
        let x = abs_x.max(clip.x as i32) as u16;
        if x >= clip.right() {
            continue;
        }
        buf.set_line(x, y as u16, line, clip.right().saturating_sub(x));
    }
}

fn patch_line_style(mut line: Line<'static>, style: Style) -> Line<'static> {
    line.spans = line.spans.into_iter().map(|span| patch_span_style(span, style)).collect();
    line
}

fn patch_span_style(mut span: Span<'static>, style: Style) -> Span<'static> {
    span.style = span.style.patch(style);
    span
}

fn line_width(line: &Line<'static>) -> usize {
    line.spans.iter().map(|span| UnicodeWidthStr::width(span.content.as_ref())).sum()
}

fn union_rect(a: Rect, b: Rect) -> Rect {
    if a.width == 0 && a.height == 0 {
        return b;
    }
    if b.width == 0 && b.height == 0 {
        return a;
    }
    let left = a.x.min(b.x);
    let top = a.y.min(b.y);
    let right = a.right().max(b.right());
    let bottom = a.bottom().max(b.bottom());
    Rect::new(left, top, right - left, bottom - top)
}

fn rect_contains(rect: Rect, point: Position) -> bool {
    point.x >= rect.x && point.x < rect.right() && point.y >= rect.y && point.y < rect.bottom()
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y
}
