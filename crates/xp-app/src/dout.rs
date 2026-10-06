//! The data-output text lines of the reference build (`engine/dout/dout_lines.cpp`), the rows X-Plane can
//! draw over the scene ("frame rate" and the others of its Data Output settings).
//!
//! What is taken from the reference build (`tools/extract_dout.py`, `assets/dout/lines.tsv`): the 173 line
//! label strings. A label is 8 cells of 12 characters; `_` stands for a space; the first six characters of a
//! cell are its upper label row and the last six its lower row; a cell whose label is `_____-_____` is empty.
//! The frame-rate line has a second form without the `vblnk sync` cell, which the reference build uses when
//! that value is not available.
//!
//! What is estimated from a photograph of the reference build's frame-rate line, not from its drawing code
//! (`0x1416cc3a0`, not ported): the three-row layout (upper label, value, lower label), the 8-character cell
//! pitch, the teal text colour, and the value format (six characters, as many decimals as fit). Values are
//! supplied by the caller; the font is this project's, not the reference build's bitmap font.
use crate::hud::{Color, Hud};

#[allow(dead_code)]
const LINES: &str = include_str!("../assets/dout/lines.tsv");
/// The frame-rate line without the `vblnk sync` cell (`0x142721090` variant in the reference build).
pub const FRAME_RATE_NO_VSYNC: &str = "f-act  /sec f-sim  /sec _____-_____ frame  time___cpu  time  gpu  time___grnd ratio  flit ratio";
/// Cells per line and characters per cell label.
pub const CELLS: usize = 8;
const CELL_CHARS: usize = 12;
/// Teal of the reference build's data-output text, from the photograph.
pub const TEXT: Color = [0.12, 0.9, 0.68, 1.0];

/// The label string of line `index`.
#[allow(dead_code)]
pub fn label(index: usize) -> Option<String> {
    LINES.lines().find_map(|l| {
        let (i, label) = l.split_once('\t')?;
        (i.parse::<usize>().ok()? == index).then(|| label.to_string())
    })
}

/// Number of lines in the table.
#[allow(dead_code)]
pub fn line_count() -> usize {
    LINES.lines().count()
}

/// The upper and lower label rows (six characters each) of the eight cells of a label string; `None` for an
/// empty cell.
pub fn cell_labels(label: &str) -> [Option<(String, String)>; CELLS] {
    let chars: Vec<char> = label
        .chars()
        .map(|c| if c == '_' { ' ' } else { c })
        .collect();
    let raw: Vec<char> = label.chars().collect();
    std::array::from_fn(|i| {
        let cell = raw.get(i * CELL_CHARS..((i + 1) * CELL_CHARS).min(raw.len()))?;
        if cell.iter().collect::<String>() == "_____-_____ "
            || cell.iter().collect::<String>() == "_____-_____"
        {
            return None;
        }
        let shown = &chars[i * CELL_CHARS..((i + 1) * CELL_CHARS).min(chars.len())];
        let top: String = shown.iter().take(6).collect();
        let bottom: String = shown.iter().skip(6).take(6).collect();
        Some((top, bottom))
    })
}

/// A value in six characters with as many decimals as fit (10.462, 0.0956, 1.0000).
pub fn format_value(v: f32) -> String {
    let int_digits = format!("{:.0}", v.abs().trunc()).len();
    let sign = usize::from(v < 0.0);
    let decimals = 5usize.saturating_sub(int_digits + sign);
    let s = format!("{v:.decimals$}");
    format!("{s:>6}")
}

/// Draws one line with its top left corner at (x, y): upper labels, values, lower labels. `values[i]` is
/// `None` for a cell without a value. Returns the height used.
pub fn draw_line(
    hud: &mut Hud,
    x: f32,
    y: f32,
    scale: f32,
    label: &str,
    values: &[Option<f32>; CELLS],
) -> f32 {
    let pitch = 8.0 * crate::hud::CELL_W * scale;
    let row = crate::hud::CELL_H * scale * 0.95;
    for (i, cell) in cell_labels(label).iter().enumerate() {
        let Some((top, bottom)) = cell else { continue };
        let cx = x + i as f32 * pitch;
        hud.text(cx, y, scale, TEXT, top);
        if let Some(v) = values[i] {
            hud.text(cx, y + row, scale, TEXT, &format_value(v));
        }
        hud.text(cx, y + 2.0 * row, scale, TEXT, bottom);
    }
    3.0 * row
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_has_every_line_and_the_frame_rate_cells() {
        assert_eq!(line_count(), 173);
        let l = label(0).unwrap();
        assert_eq!(l.chars().count(), 95);
        let cells = cell_labels(&l);
        assert_eq!(cells[0], Some(("f-act ".into(), " /sec ".into())));
        assert_eq!(cells[2], Some(("frame ".into(), " time ".into())));
        assert_eq!(cells[5], Some(("vblnk ".into(), " sync ".into())));
        let none = cell_labels(FRAME_RATE_NO_VSYNC);
        assert!(none[2].is_none());
        assert_eq!(none[3], Some(("frame ".into(), " time ".into())));
        assert_eq!(none[7], Some((" flit ".into(), "ratio".into())));
    }

    #[test]
    fn values_use_six_characters() {
        assert_eq!(format_value(10.462), "10.462");
        assert_eq!(format_value(19.9), "19.900");
        assert_eq!(format_value(0.0956), "0.0956");
        assert_eq!(format_value(1.0), "1.0000");
        assert_eq!(format_value(123.456), "123.46");
        assert_eq!(format_value(-1.5), "-1.500");
    }

    #[test]
    fn a_line_stays_within_the_buffer() {
        let mut hud = Hud::new(1280, 720);
        let values = [
            Some(10.462),
            Some(19.9),
            None,
            Some(0.0956),
            Some(0.0698),
            Some(0.0956),
            Some(1.0),
            Some(1.0),
        ];
        let h = draw_line(&mut hud, 10.0, 10.0, 1.0, FRAME_RATE_NO_VSYNC, &values);
        assert!(h > 0.0);
        assert!(hud.vertices.len() < crate::hud::MAX_VERTICES);
        assert!(!hud.vertices.is_empty());
    }
}
