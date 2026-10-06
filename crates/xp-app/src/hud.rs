//! Screen-space interface drawn over the scene: a bitmap font (Roboto Mono, see assets/font), filled
//! rectangles and rotated lines, built on the CPU as textured triangles in normalised device coordinates,
//! . No dependency beyond the renderer's quad pipeline.
use bytemuck::{Pod, Zeroable};

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

pub struct Hud {
    pub vertices: Vec<HudVertex>,
    width: f32,
    height: f32,
}

// Rectangles, lines and centred text are the drawing primitives for the interface to come; only text is
// used so far.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Color = [1.0, 1.0, 1.0, 1.0];
    const PANEL: Color = [0.02, 0.04, 0.07, 0.55];

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
}
