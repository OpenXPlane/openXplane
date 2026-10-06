//! Screen-space interface drawn over the scene: a bitmap font (Roboto Mono, see assets/font), filled
//! rectangles and rotated lines, built on the CPU as textured triangles in normalised device coordinates,
//! and the flight instruments made from them. No dependency beyond the renderer's quad pipeline.
use bytemuck::{Pod, Zeroable};
use openxplane::flight::{Controls, Telemetry};

pub const CELL_W: f32 = 12.0;
pub const CELL_H: f32 = 26.0;
const COLS: usize = 16;
const ATLAS_W: f32 = 192.0;
const ATLAS_H: f32 = 156.0;
/// Capacity of the vertex buffer, in vertices.
pub const MAX_VERTICES: usize = 24576;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct HudVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

pub type Color = [f32; 4];
pub const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
pub const PANEL: Color = [0.02, 0.04, 0.07, 0.55];
pub const GREEN: Color = [0.35, 0.9, 0.5, 1.0];
pub const AMBER: Color = [1.0, 0.75, 0.2, 1.0];
pub const RED: Color = [1.0, 0.3, 0.25, 1.0];
pub const SKY: Color = [0.35, 0.62, 0.95, 0.9];
pub const GROUND: Color = [0.55, 0.38, 0.2, 0.9];

pub struct Hud {
    pub vertices: Vec<HudVertex>,
    width: f32,
    height: f32,
}

impl Hud {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            vertices: Vec::with_capacity(4096),
            width: width as f32,
            height: height as f32,
        }
    }

    pub fn size(&self) -> (f32, f32) {
        (self.width, self.height)
    }

    /// Pixel position (origin top left, y down) to normalised device coordinates.
    pub fn ndc(&self, x: f32, y: f32) -> [f32; 2] {
        [x / self.width * 2.0 - 1.0, 1.0 - y / self.height * 2.0]
    }

    /// Texture rectangle (u0, v0, u1, v1) of a character; unknown characters draw as a space.
    pub fn glyph_uv(c: char) -> [f32; 4] {
        let code = c as u32;
        let i = if (32..=127).contains(&code) {
            (code - 32) as usize
        } else {
            0
        };
        let (col, row) = ((i % COLS) as f32, (i / COLS) as f32);
        [
            col * CELL_W / ATLAS_W,
            row * CELL_H / ATLAS_H,
            (col + 1.0) * CELL_W / ATLAS_W,
            (row + 1.0) * CELL_H / ATLAS_H,
        ]
    }

    /// A quad given by four pixel corners in order, with a texture rectangle.
    pub fn quad(&mut self, p: [[f32; 2]; 4], uv: [f32; 4], color: Color) {
        if self.vertices.len() + 6 > MAX_VERTICES {
            return;
        }
        let corner = |i: usize| {
            let (u, v) = match i {
                0 => (uv[0], uv[1]),
                1 => (uv[2], uv[1]),
                2 => (uv[2], uv[3]),
                _ => (uv[0], uv[3]),
            };
            HudVertex {
                pos: self.ndc(p[i][0], p[i][1]),
                uv: [u, v],
                color,
            }
        };
        let v: Vec<HudVertex> = (0..4).map(corner).collect();
        self.vertices.extend([v[0], v[1], v[2], v[0], v[2], v[3]]);
    }

    /// A filled rectangle (the atlas's solid cell is sampled at its centre).
    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let c = Self::glyph_uv('\u{7f}');
        let (u, v) = ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0);
        self.quad(
            [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
            [u, v, u, v],
            color,
        );
    }

    /// A line of the given thickness between two pixel points.
    pub fn line(&mut self, a: [f32; 2], b: [f32; 2], thickness: f32, color: Color) {
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let len = dx.hypot(dy);
        if len < 1e-3 {
            return;
        }
        let (nx, ny) = (-dy / len * thickness * 0.5, dx / len * thickness * 0.5);
        let c = Self::glyph_uv('\u{7f}');
        let (u, v) = ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0);
        self.quad(
            [
                [a[0] + nx, a[1] + ny],
                [b[0] + nx, b[1] + ny],
                [b[0] - nx, b[1] - ny],
                [a[0] - nx, a[1] - ny],
            ],
            [u, v, u, v],
            color,
        );
    }

    pub fn text_width(text: &str, scale: f32) -> f32 {
        text.chars().count() as f32 * CELL_W * scale
    }

    /// Draws text with its top left corner at (x, y); returns the width drawn.
    pub fn text(&mut self, x: f32, y: f32, scale: f32, color: Color, text: &str) -> f32 {
        let mut pen = x;
        for c in text.chars() {
            if c != ' ' {
                let (w, h) = (CELL_W * scale, CELL_H * scale);
                self.quad(
                    [[pen, y], [pen + w, y], [pen + w, y + h], [pen, y + h]],
                    Self::glyph_uv(c),
                    color,
                );
            }
            pen += CELL_W * scale;
        }
        pen - x
    }

    pub fn text_centered(&mut self, cx: f32, y: f32, scale: f32, color: Color, text: &str) {
        let w = Self::text_width(text, scale);
        self.text(cx - w * 0.5, y, scale, color, text);
    }
}

/// The lines of the key help overlay (`H`).
pub const HELP: &[&str] = &[
    "KEYS (X-PLANE DEFAULTS)",
    "F1 / F2 / F3    throttle down / up / full",
    "1 / 2           flaps up / down",
    "B               brakes (hold)    V  max brakes",
    "[ ]  5 6 7  8 9 0   pitch / rudder / aileron trim",
    "P               pause            W  default view",
    "Q E R F  = -    move the camera",
    "",
    "OPENXPLANE KEYS",
    "Up / Down       pitch    Left / Right  roll",
    "Z / X           rudder   Tab  arrows as view keys",
    "H               this help     Delete  reset",
    "Mouse drag      look around      Esc  quit",
];

pub struct FlightHud<'a> {
    pub telemetry: &'a Telemetry,
    pub controls: &'a Controls,
    pub paused: bool,
    pub help: bool,
    pub note: Option<&'a str>,
}

/// Draws the flight instruments: the speed, altitude and heading panel, the attitude indicator, the
/// throttle bar, trims, warnings and the optional help overlay.
pub fn draw_flight(hud: &mut Hud, f: &FlightHud) {
    let (w, h) = hud.size();
    let s = (h / 800.0).clamp(0.7, 1.6);
    let t = f.telemetry;

    // speed, altitude, vertical speed, heading
    let (px, pw, ph) = (16.0 * s, 270.0 * s, 132.0 * s);
    let py = h - ph - 16.0 * s;
    hud.rect(px, py, pw, ph, PANEL);
    let rows = [
        ("IAS", format!("{:>5.0} kt", t.airspeed_kt)),
        ("ALT", format!("{:>5.0} ft", t.altitude_ft)),
        ("VS ", format!("{:>+5.0} fpm", t.vertical_speed_fpm)),
        ("HDG", format!("{:>5.0} deg", t.heading_deg)),
    ];
    for (i, (label, value)) in rows.iter().enumerate() {
        let y = py + (8.0 + 30.0 * i as f32) * s;
        hud.text(px + 10.0 * s, y, 1.0 * s, [0.7, 0.8, 0.95, 1.0], label);
        hud.text(px + 60.0 * s, y, 1.0 * s, WHITE, value);
    }

    // attitude indicator
    let (cx, cy, r) = (w * 0.5, h - 112.0 * s, 92.0 * s);
    hud.rect(cx - r, cy - r, 2.0 * r, 2.0 * r, PANEL);
    let roll = t.roll_deg.to_radians();
    let (right, down) = ([roll.cos(), -roll.sin()], [roll.sin(), roll.cos()]);
    let ppd = 2.6 * s;
    let at = |along: f32, drop: f32| {
        [
            cx + right[0] * along + down[0] * drop,
            cy + right[1] * along + down[1] * drop,
        ]
    };
    let horizon = t.pitch_deg * ppd;
    // sky and ground as two thin wedges next to the horizon line, then the pitch ladder
    hud.line(
        at(-r * 0.9, horizon + 5.0 * s),
        at(r * 0.9, horizon + 5.0 * s),
        10.0 * s,
        GROUND,
    );
    hud.line(
        at(-r * 0.9, horizon - 5.0 * s),
        at(r * 0.9, horizon - 5.0 * s),
        10.0 * s,
        SKY,
    );
    hud.line(
        at(-r * 0.95, horizon),
        at(r * 0.95, horizon),
        2.5 * s,
        WHITE,
    );
    for mark in [-20, -10, 10, 20] {
        let drop = horizon - mark as f32 * ppd;
        if drop.abs() < r * 0.85 {
            hud.line(
                at(-r * 0.3, drop),
                at(r * 0.3, drop),
                2.0 * s,
                [1.0, 1.0, 1.0, 0.7],
            );
        }
    }
    // fixed aircraft symbol
    hud.line([cx - 44.0 * s, cy], [cx - 12.0 * s, cy], 4.0 * s, AMBER);
    hud.line([cx + 12.0 * s, cy], [cx + 44.0 * s, cy], 4.0 * s, AMBER);
    hud.rect(cx - 3.0 * s, cy - 3.0 * s, 6.0 * s, 6.0 * s, AMBER);
    hud.text_centered(
        cx,
        cy - r + 4.0 * s,
        0.8 * s,
        WHITE,
        &format!("P{:+.0} B{:+.0}", t.pitch_deg, t.roll_deg),
    );

    // throttle bar, flaps and trims
    let (bx, bw, bh) = (w - 74.0 * s, 28.0 * s, 150.0 * s);
    let by = h - bh - 56.0 * s;
    hud.rect(
        bx - 6.0 * s,
        by - 28.0 * s,
        bw + 52.0 * s,
        bh + 84.0 * s,
        PANEL,
    );
    hud.rect(bx, by, bw, bh, [0.1, 0.12, 0.16, 0.9]);
    let filled = bh * t.throttle.clamp(0.0, 1.0);
    hud.rect(bx, by + bh - filled, bw, filled, GREEN);
    hud.text(bx - 2.0 * s, by - 24.0 * s, 0.75 * s, WHITE, "THR");
    hud.text(
        bx - 2.0 * s,
        by + bh + 4.0 * s,
        0.75 * s,
        WHITE,
        &format!("{:>3.0}%", t.throttle * 100.0),
    );
    hud.text(
        bx - 2.0 * s,
        by + bh + 24.0 * s,
        0.75 * s,
        [0.8, 0.9, 1.0, 1.0],
        &format!("FL{:>3.0}", f.controls.flaps * 100.0),
    );
    hud.text(
        bx + bw + 6.0 * s,
        by,
        0.7 * s,
        [0.8, 0.9, 1.0, 1.0],
        &format!("T{:+.0}", f.controls.elevator_trim * 100.0),
    );

    // warnings and notes
    if t.stalled_elements > 0 {
        hud.text_centered(w * 0.5, 40.0 * s, 1.8 * s, RED, "STALL");
    }
    if t.on_ground {
        hud.text_centered(
            w * 0.5,
            14.0 * s,
            0.8 * s,
            [0.8, 0.9, 1.0, 1.0],
            "ON GROUND",
        );
    }
    if f.paused {
        hud.text_centered(w * 0.5, 90.0 * s, 1.4 * s, AMBER, "PAUSED");
    }
    if let Some(note) = f.note {
        hud.text_centered(w * 0.5, h * 0.3, 0.9 * s, AMBER, note);
    }
    if f.help {
        let lines = HELP.len() as f32;
        let (hw, hh) = (
            Hud::text_width("[ ]  5 6 7  8 9 0   pitch / rudder / aileron trim", 0.8 * s)
                + 40.0 * s,
            lines * 24.0 * s + 30.0 * s,
        );
        let (x0, y0) = (w * 0.5 - hw * 0.5, h * 0.5 - hh * 0.5 - 40.0 * s);
        hud.rect(x0, y0, hw, hh, [0.02, 0.04, 0.07, 0.82]);
        for (i, line) in HELP.iter().enumerate() {
            let heading = line.chars().all(|c| !c.is_lowercase()) && !line.is_empty();
            hud.text(
                x0 + 20.0 * s,
                y0 + 16.0 * s + 24.0 * s * i as f32,
                0.8 * s,
                if heading { AMBER } else { WHITE },
                line,
            );
        }
    } else {
        hud.text(
            w - 150.0 * s,
            10.0 * s,
            0.7 * s,
            [0.8, 0.9, 1.0, 0.7],
            "H: help",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_map_to_normalised_device_coordinates() {
        let hud = Hud::new(200, 100);
        assert_eq!(hud.ndc(0.0, 0.0), [-1.0, 1.0]);
        assert_eq!(hud.ndc(200.0, 100.0), [1.0, -1.0]);
        assert_eq!(hud.ndc(100.0, 50.0), [0.0, 0.0]);
    }

    #[test]
    fn glyphs_sit_in_their_atlas_cells() {
        let a = Hud::glyph_uv('A');
        // 'A' is 33 characters after the space: column 1, row 2
        assert_eq!(a[0], 12.0 / 192.0);
        assert_eq!(a[1], 2.0 * 26.0 / 156.0);
        assert_eq!(Hud::glyph_uv(' ')[0], 0.0);
        assert_eq!(Hud::glyph_uv('é'), Hud::glyph_uv(' '));
        let solid = Hud::glyph_uv('\u{7f}');
        assert_eq!((solid[2], solid[3]), (1.0, 1.0));
    }

    #[test]
    fn text_makes_a_quad_per_visible_character() {
        let mut hud = Hud::new(800, 600);
        let width = hud.text(10.0, 10.0, 1.0, WHITE, "IAS 87");
        assert_eq!(width, 6.0 * CELL_W);
        assert_eq!(hud.vertices.len(), 5 * 6, "the space draws nothing");
        assert!(
            hud.vertices
                .iter()
                .all(|v| v.pos[0] >= -1.0 && v.pos[0] <= 1.0)
        );
    }

    #[test]
    fn rectangles_and_lines_cover_the_expected_area() {
        let mut hud = Hud::new(100, 100);
        hud.rect(0.0, 0.0, 100.0, 100.0, PANEL);
        let xs: Vec<f32> = hud.vertices.iter().map(|v| v.pos[0]).collect();
        assert_eq!(
            (
                xs.iter().cloned().fold(f32::MAX, f32::min),
                xs.iter().cloned().fold(f32::MIN, f32::max)
            ),
            (-1.0, 1.0)
        );
        let mut line = Hud::new(100, 100);
        line.line([10.0, 50.0], [90.0, 50.0], 4.0, WHITE);
        let ys: Vec<f32> = line.vertices.iter().map(|v| v.pos[1]).collect();
        let spread = ys.iter().cloned().fold(f32::MIN, f32::max)
            - ys.iter().cloned().fold(f32::MAX, f32::min);
        assert!(
            (spread - 4.0 / 100.0 * 2.0).abs() < 1e-5,
            "a 4 px line is 0.08 tall in NDC: {spread}"
        );
        let mut none = Hud::new(100, 100);
        none.line([5.0, 5.0], [5.0, 5.0], 3.0, WHITE);
        assert!(none.vertices.is_empty());
    }

    #[test]
    fn buffer_capacity_is_never_exceeded() {
        let mut hud = Hud::new(100, 100);
        for _ in 0..(MAX_VERTICES / 6 + 50) {
            hud.rect(0.0, 0.0, 1.0, 1.0, WHITE);
        }
        assert!(hud.vertices.len() <= MAX_VERTICES);
    }

    #[test]
    fn the_flight_hud_fits_the_buffer_and_shows_warnings() {
        let telemetry = Telemetry {
            airspeed_kt: 87.0,
            altitude_ft: 1234.0,
            vertical_speed_fpm: 650.0,
            pitch_deg: 7.0,
            roll_deg: -12.0,
            heading_deg: 160.0,
            alpha_deg: 4.0,
            throttle: 1.0,
            on_ground: false,
            stalled_elements: 3,
        };
        let controls = Controls::default();
        let mut calm = Hud::new(1280, 800);
        draw_flight(
            &mut calm,
            &FlightHud {
                telemetry: &telemetry,
                controls: &controls,
                paused: false,
                help: false,
                note: None,
            },
        );
        let mut busy = Hud::new(1280, 800);
        draw_flight(
            &mut busy,
            &FlightHud {
                telemetry: &telemetry,
                controls: &controls,
                paused: true,
                help: true,
                note: Some("reset"),
            },
        );
        assert!(busy.vertices.len() > calm.vertices.len());
        assert!(
            busy.vertices.len() < MAX_VERTICES,
            "{}",
            busy.vertices.len()
        );
        assert!(
            calm.vertices.iter().any(|v| v.color == RED),
            "the stall warning is red"
        );
    }
}
