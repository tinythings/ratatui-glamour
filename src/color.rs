use ratatui::style::Color;

/// Convert any ratatui Color to (R, G, B) components.
pub fn color_to_rgb(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::Indexed(n) => indexed_to_rgb(n),
        _ => (255, 255, 255),
    }
}

/// Linearly interpolate between two Colors in RGB space.
/// Output is always `Color::Rgb` for true-color gradient fidelity.
pub fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let (fr, fg, fb) = color_to_rgb(from);
    let (tr, tg, tb) = color_to_rgb(to);
    Color::Rgb(lerp_u8(fr, tr, t), lerp_u8(fg, tg, t), lerp_u8(fb, tb, t))
}

pub fn lerp_u8(from: u8, to: u8, t: f32) -> u8 {
    (from as f32 + (to as f32 - from as f32) * t).round() as u8
}

pub fn blend_1d(steps: usize, stops: &[Color]) -> Vec<Color> {
    if steps == 0 || stops.is_empty() {
        return Vec::new();
    }
    if stops.len() == 1 {
        return vec![stops[0]; steps];
    }
    if steps <= stops.len() {
        return stops[..steps].to_vec();
    }

    let segments = stops.len() - 1;
    let mut out = Vec::with_capacity(steps);
    for step in 0..steps {
        let pos = if steps > 1 {
            step as f32 / (steps - 1) as f32
        } else {
            0.0
        };
        let scaled = pos * segments as f32;
        let idx = scaled.floor() as usize;
        let from = stops[idx.min(segments - 1)];
        let to = stops[(idx + 1).min(segments)];
        let local_t = scaled - idx.min(segments - 1) as f32;
        out.push(if idx >= segments {
            *stops.last().unwrap()
        } else {
            lerp_color(from, to, local_t)
        });
    }
    out
}

pub fn blend_2d(width: usize, height: usize, angle_degrees: f32, stops: &[Color]) -> Vec<Color> {
    if width == 0 || height == 0 || stops.is_empty() {
        return Vec::new();
    }
    if stops.len() == 1 {
        return vec![stops[0]; width * height];
    }

    let angle = angle_degrees.rem_euclid(360.0).to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    let center_x = (width.saturating_sub(1)) as f32 / 2.0;
    let center_y = (height.saturating_sub(1)) as f32 / 2.0;
    let diagonal = ((width * width + height * height) as f32).sqrt().max(1.0);
    let gradient = blend_1d(width.max(height), stops);
    let max_idx = gradient.len().saturating_sub(1) as f32;
    let mut out = Vec::with_capacity(width * height);

    for y in 0..height {
        let dy = y as f32 - center_y;
        for x in 0..width {
            let dx = x as f32 - center_x;
            let rotated = dx * cos - dy * sin;
            let pos = ((rotated + diagonal / 2.0) / diagonal).clamp(0.0, 1.0);
            let idx = (pos * max_idx).round() as usize;
            out.push(gradient[idx.min(gradient.len().saturating_sub(1))]);
        }
    }

    out
}

/// Convert an xterm-256 indexed color to 24-bit RGB.
pub fn indexed_to_rgb(index: u8) -> (u8, u8, u8) {
    match index {
        0 => (0x00, 0x00, 0x00),
        1 => (0x80, 0x00, 0x00),
        2 => (0x00, 0x80, 0x00),
        3 => (0x80, 0x80, 0x00),
        4 => (0x00, 0x00, 0x80),
        5 => (0x80, 0x00, 0x80),
        6 => (0x00, 0x80, 0x80),
        7 => (0xc0, 0xc0, 0xc0),
        8 => (0x80, 0x80, 0x80),
        9 => (0xff, 0x00, 0x00),
        10 => (0x00, 0xff, 0x00),
        11 => (0xff, 0xff, 0x00),
        12 => (0x00, 0x00, 0xff),
        13 => (0xff, 0x00, 0xff),
        14 => (0x00, 0xff, 0xff),
        15 => (0xff, 0xff, 0xff),
        16..=231 => {
            let cube = index - 16;
            let r = cube / 36;
            let g = (cube / 6) % 6;
            let b = cube % 6;
            const STEPS: [u8; 6] = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
            (STEPS[r as usize], STEPS[g as usize], STEPS[b as usize])
        }
        232..=255 => {
            let gray = 8 + (index - 232) * 10;
            (gray, gray, gray)
        }
    }
}
