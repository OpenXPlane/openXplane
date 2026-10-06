//! The data-output text lines of the reference build (`engine/dout/dout_lines.cpp`), the rows X-Plane can
//! draw over the scene ("frame rate" and the others of its Data Output settings).
//!
//! What is taken from the reference build (`tools/extract_dout.py`, `assets/dout/lines.tsv`): the 173 line
//! label strings. A label is 8 cells of 12 characters; `_` stands for a space; the first six characters of a
//! cell are its upper label row and the last six its lower row; a cell whose label is `_____-_____` is empty.
//! A `-` inside a label is drawn as a space (the photograph shows no dashes; this is an assumption).
//! The frame-rate line has a second form without the `vblnk sync` cell, which the reference build uses when
//! that value is not available.
//!
//! What is estimated from a photograph of the reference build's frame-rate line, not from its drawing code
//! (`0x1416cc3a0`, not ported): the three-row layout (upper label, value, lower label), the 8-character cell
//! pitch, the teal text colour, and the value format (six characters, as many decimals as fit). Values are
//! supplied by the caller; the font is this project's, not the reference build's bitmap font.
use crate::hud::{Color, Hud};
use openxplane::flight::{Controls, State, Telemetry};

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
        .map(|c| if matches!(c, '_' | '-') { ' ' } else { c })
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

/// The values the viewer can supply to a data-output line.
pub struct Sample<'a> {
    pub telemetry: &'a Telemetry,
    pub controls: &'a Controls,
    pub state: &'a State,
    /// The measured frame timings (line 0), when the caller has them.
    pub frame: Option<[Option<f32>; CELLS]>,
}

const KT_PER_MS: f32 = 1.943_844_5;
const MPH_PER_MS: f32 = 2.236_936_3;

/// Speed of sound in m/s of the standard atmosphere at the altitude in metres.
fn speed_of_sound(altitude_m: f32) -> f32 {
    let t = 288.15 - 0.0065 * altitude_m.min(11_000.0);
    340.294 * (t / 288.15).sqrt()
}

/// The cell values of line `index` from the flight model, or `None` for a line this build cannot fill. The
/// values come from the approximate flight model (research/FLIGHT_MODEL.md); cells without a source are
/// empty.
pub fn line_values(index: usize, s: &Sample) -> Option<[Option<f32>; CELLS]> {
    let (t, c, st) = (s.telemetry, s.controls, s.state);
    let tas = st.velocity.length();
    let ground = (st.velocity.x * st.velocity.x + st.velocity.z * st.velocity.z).sqrt();
    let heading = |x: f32, z: f32| (x.atan2(-z).to_degrees() + 360.0) % 360.0;
    let mut v = [None; CELLS];
    match index {
        0 => return s.frame,
        3 => {
            v[0] = Some(t.airspeed_kt);
            v[1] = Some(t.airspeed_kt);
            v[2] = Some(tas * KT_PER_MS);
            v[3] = Some(ground * KT_PER_MS);
            v[5] = Some(t.airspeed_kt * MPH_PER_MS / KT_PER_MS);
            v[6] = Some(tas * MPH_PER_MS);
            v[7] = Some(ground * MPH_PER_MS);
        }
        4 => {
            v[0] = Some(tas / speed_of_sound(t.altitude_ft * 0.3048));
            v[2] = Some(t.vertical_speed_fpm);
        }
        8 => {
            v[0] = Some(c.elevator);
            v[1] = Some(c.aileron);
            v[2] = Some(c.rudder);
        }
        13 => {
            v[0] = Some(c.elevator_trim);
            v[1] = Some(c.aileron_trim);
            v[2] = Some(c.rudder_trim);
            v[3] = Some(c.flaps);
            v[4] = Some(c.flaps);
        }
        17 => {
            v[0] = Some(t.pitch_deg);
            v[1] = Some(t.roll_deg);
            v[2] = Some(t.heading_deg);
        }
        18 => {
            let body = st.orientation.inverse() * st.velocity;
            v[0] = Some(t.alpha_deg);
            v[1] = Some(body.x.atan2(-body.z).to_degrees());
            v[2] = Some(heading(st.velocity.x, st.velocity.z));
            v[3] = Some(st.velocity.y.atan2(ground).to_degrees());
        }
        20 => {
            v[2] = Some(t.altitude_ft);
            v[5] = Some(t.altitude_ft);
        }
        21 => {
            v[0] = Some(st.position.x);
            v[1] = Some(st.position.y);
            v[2] = Some(st.position.z);
            v[3] = Some(st.velocity.x);
            v[4] = Some(st.velocity.y);
            v[5] = Some(st.velocity.z);
        }
        25 => v[0] = Some(c.throttle),
        _ => return None,
    }
    Some(v)
}

/// The lines selected by `OPENXPLANE_DATA_OUTPUT` (a comma-separated list of line indexes, like the Data
/// Output checkboxes of the reference build); `OPENXPLANE_FRAME_RATE` adds line 0.
pub fn selected_lines() -> Vec<usize> {
    let mut lines: Vec<usize> = std::env::var("OPENXPLANE_DATA_OUTPUT")
        .unwrap_or_default()
        .split(',')
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    if std::env::var_os("OPENXPLANE_FRAME_RATE").is_some() {
        lines.push(0);
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

/// Draws the selected lines one under the other in index order; lines without values are skipped. The
/// frame-rate line uses its label without the vertical-blank cell.
pub fn draw_lines(hud: &mut Hud, lines: &[usize], sample: &Sample, scale: f32) {
    let (x, mut y) = (16.0 * scale, 16.0 * scale);
    for &index in lines {
        let Some(values) = line_values(index, sample) else {
            continue;
        };
        let label = if index == 0 {
            Some(FRAME_RATE_NO_VSYNC.to_string())
        } else {
            label(index)
        };
        if let Some(label) = label {
            y += draw_line(hud, x, y, scale, &label, &values) + 6.0 * scale;
        }
    }
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
    fn supported_lines_fill_cells_that_have_a_label() {
        let telemetry = Telemetry {
            airspeed_kt: 80.0,
            altitude_ft: 1000.0,
            vertical_speed_fpm: 500.0,
            pitch_deg: 5.0,
            roll_deg: -2.0,
            heading_deg: 90.0,
            alpha_deg: 4.0,
            throttle: 1.0,
            on_ground: false,
            stalled_elements: 0,
        };
        let controls = Controls::default();
        let state = State {
            position: glam::Vec3::new(1.0, 2.0, 3.0),
            velocity: glam::Vec3::new(40.0, 0.0, 0.0),
            orientation: glam::Quat::IDENTITY,
            omega: glam::Vec3::ZERO,
            time: 0.0,
        };
        let sample = Sample {
            telemetry: &telemetry,
            controls: &controls,
            state: &state,
            frame: None,
        };
        for index in [3, 4, 8, 13, 17, 18, 20, 21, 25] {
            let values = line_values(index, &sample).unwrap();
            let cells = cell_labels(&label(index).unwrap());
            for (value, cell) in values.iter().zip(&cells) {
                assert!(value.is_none() || cell.is_some(), "line {index}");
            }
            assert!(values.iter().any(Option::is_some));
        }
        let speeds = line_values(3, &sample).unwrap();
        assert!((speeds[3].unwrap() - 40.0 * KT_PER_MS).abs() < 1e-3);
        assert!(line_values(100, &sample).is_none());
        assert!(line_values(0, &sample).is_none());
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
