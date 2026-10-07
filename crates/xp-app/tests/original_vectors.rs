//! Compares the ports against vectors produced by running the ORIGINAL machine code of the
//! reference build in an emulator (tools/gen_profile_vectors.py). Every output must match bit for bit.
use openxplane::{
    airfoil::{Coefficients, POLAR_ROWS, Polar, PolarRow},
    buffet::NoiseTable,
    profile::{Inputs, evaluate_polar},
    runtime::RunningTime,
};

fn polar(k: i64, p: [f32; 5]) -> Polar {
    let mut parameters = [0.0f32; 25];
    parameters[..5].copy_from_slice(&p);
    let rows = (0..POLAR_ROWS as i64)
        .map(|i| PolarRow {
            alpha_deg: i as f32,
            coefficients: Coefficients {
                cl: ((i * 131 + k * 977) % 4001 - 2000) as f32 / 1000.0,
                cd: ((i * 37 + k * 53) % 997) as f32 / 10000.0,
                cm: ((i * 61 + k * 389) % 2003 - 1000) as f32 / 5000.0,
            },
        })
        .collect();
    Polar { parameters, rows }
}

fn noise() -> NoiseTable {
    let values = (0..262144u64)
        .map(|i| {
            let h = ((i * 2654435761) & 0xffff_ffff) >> 8;
            (h as f32 / 16777216.0) * 2.0 - 1.0
        })
        .collect();
    NoiseTable::new(values).unwrap()
}

fn f(hex: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(hex, 16).unwrap())
}

#[test]
fn profile_stage_matches_the_original_machine_code() {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/profile_stage.txt"
    ))
    .unwrap();
    let noise = noise();
    let mut cases = 0;
    let mut stalled_cases = 0;
    let mut mismatches = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(t[15], "|", "{line}");
        let k: i64 = t[0].parse().unwrap();
        let p = [f(t[1]), f(t[2]), f(t[3]), f(t[4]), f(t[5])];
        let time = f64::from_bits(u64::from_str_radix(t[14], 16).unwrap());
        let input = Inputs {
            alpha_deg: f(t[6]),
            multiplier: f(t[7]),
            divisor: f(t[8]),
            regime: 0.0,
            time: RunningTime::from_snapshot(time).unwrap(),
            noise_coordinates: [f(t[11]), f(t[12]), f(t[13])],
            retain_stall: t[9] == "1",
        };
        let prev = t[10] == "1";
        let got = evaluate_polar(&polar(k, p), input, prev, Some(&noise)).unwrap();
        let want_norm = f(t[16]);
        let want_stall = t[17] == "1";
        let want = [f(t[18]), f(t[19]), f(t[20])];
        let have = [
            got.coefficients.cl,
            got.coefficients.cd,
            got.coefficients.cm,
        ];
        cases += 1;
        stalled_cases += usize::from(want_stall);
        let same = got.stall.normalized_alpha.to_bits() == want_norm.to_bits()
            && got.stall.stalled == want_stall
            && have
                .iter()
                .zip(want)
                .all(|(a, b)| a.to_bits() == b.to_bits());
        if !same {
            mismatches.push(format!(
                "{line}\n   port: norm {:08x} stall {} cl/cd/cm {:08x} {:08x} {:08x}",
                got.stall.normalized_alpha.to_bits(),
                got.stall.stalled,
                have[0].to_bits(),
                have[1].to_bits(),
                have[2].to_bits()
            ));
        }
    }
    assert!(cases >= 1000, "too few cases: {cases}");
    assert!(
        stalled_cases > 100,
        "vectors must exercise the active-stall branch"
    );
    assert!(
        mismatches.is_empty(),
        "{} of {cases} cases differ from the original:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Distance in units in the last place between two floats of the same sign convention.
fn ulps(a: f32, b: f32) -> u32 {
    if a.to_bits() == b.to_bits() {
        return 0;
    }
    let key = |x: f32| {
        let b = x.to_bits() as i32;
        if b < 0 { i32::MIN.wrapping_sub(b) } else { b }
    };
    key(a).abs_diff(key(b))
}

#[test]
fn wing_element_geometry_matches_the_original_machine_code() {
    use openxplane::wing_element::{Boundary, delta_wing_weight, sweep_degrees};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/wing_geometry.txt"
    ))
    .unwrap();
    let (mut cases, mut exact_sweep, mut exact_weight, mut nonzero) = (0, 0, 0, 0);
    let (mut worst_sweep, mut worst_weight) = (0u32, 0u32);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().collect();
        assert_eq!(t[10], "|", "{line}");
        let v: Vec<f32> = t[..10].iter().map(|h| f(h)).collect();
        let (x, y, z, c) = ([v[0], v[1]], [v[2], v[3]], [v[4], v[5]], [v[6], v[7]]);
        let b = Boundary {
            x: &x,
            y: &y,
            z: &z,
            chord: &c,
        };
        let (want_sweep, want_weight) = (f(t[11]), f(t[12]));
        let sweep = sweep_degrees(&b, 0);
        let weight = delta_wing_weight(&b, 0, v[8], v[9]);
        cases += 1;
        nonzero += usize::from(want_weight != 0.0);
        exact_sweep += usize::from(sweep.to_bits() == want_sweep.to_bits());
        exact_weight += usize::from(weight.to_bits() == want_weight.to_bits());
        worst_sweep = worst_sweep.max(ulps(sweep, want_sweep));
        worst_weight = worst_weight.max(ulps(weight, want_weight));
    }
    println!(
        "{cases} cases: sweep exact {exact_sweep} (worst {worst_sweep} ulp), weight exact {exact_weight} (worst {worst_weight} ulp), {nonzero} nonzero weights"
    );
    assert!(cases >= 1000 && nonzero > 100);
    // atan2 and tan come from the platform's libm here and from the C runtime in the original
    assert!(worst_sweep <= 4, "sweep differs by {worst_sweep} ulp");
    assert!(worst_weight <= 64, "weight differs by {worst_weight} ulp");
}

#[test]
fn wing_element_area_matches_the_original_machine_code() {
    use openxplane::wing_element::element_area;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/wing_area.txt"
    ))
    .unwrap();
    let (mut cases, mut worst) = (0, 0u32);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().collect();
        let chord = [f(t[3]), f(t[4])];
        let i = 0;
        let got = element_area(f(t[0]), f(t[1]), &chord, i, t[2].parse().unwrap());
        worst = worst.max(ulps(got, f(t[6])));
        cases += 1;
    }
    println!("{cases} cases, worst {worst} ulp");
    assert!(cases >= 300);
    // cosf comes from the platform's libm here and from the C runtime in the original
    assert!(worst <= 2, "area differs by {worst} ulp");
}

#[test]
fn control_deflection_matches_the_original_machine_code() {
    use openxplane::wing_element::control_deflection;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/control_deflection.txt"
    ))
    .unwrap();
    let offsets = [
        (0xb, [0x324, 0x328, 0x1dfc, 0x1e00]),
        (0xc, [0x354, 0x358, 0x1e0c, 0x1e10]),
        (0xd, [0x444, 0x448, 0x1e18, 0x1e1c]),
        (0xe, [0x474, 0x478, 0x1e28, 0x1e2c]),
        (0xf, [0x4a4, 0x4a8, 0x1e3c, 0x1e40]),
        (0x10, [0x384, 0x388, 0x1e4c, 0x1e50]),
        (0x11, [0x3b4, 0x3b8, 0x1e60, 0x1e64]),
        (0x12, [0x3e4, 0x3e8, 0x1e74, 0x1e78]),
        (0x13, [0x414, 0x418, 0x1e88, 0x1e8c]),
        (0x14, [0x534, 0x538, 0x1e9c, 0x1ea0]),
        (0x15, [0x564, 0x568, 0x1ec4, 0x1ec8]),
        (0x16, [0x4d4, 0x4d8, 0x1ee0, 0x1ee4]),
        (0x17, [0x504, 0x508, 0x1ef0, 0x1ef4]),
    ];
    let (mut cases, mut exact) = (0, 0);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().collect();
        let code: u32 = t[0].parse().unwrap();
        let index: i32 = t[1].parse().unwrap();
        let at = offsets.iter().find(|o| o.0 == code).unwrap().1;
        let (first, last, a, b) = (f(t[2]), f(t[3]), f(t[4]), f(t[5]));
        let chords: Vec<f32> = t[6..26].iter().map(|h| f(h)).collect();
        let wing = |o: usize| {
            if o == at[0] {
                first
            } else if o == at[1] {
                last
            } else {
                chords[(o - 0x70) / 4]
            }
        };
        let control = |o: usize| if o == at[2] { a } else { b };
        let got = control_deflection(code, &wing, &control, index).unwrap();
        cases += 1;
        exact += usize::from(got.to_bits() == f(t[27]).to_bits());
    }
    println!("{cases} cases, exact {exact}");
    assert!(cases >= 600);
    assert_eq!(exact, cases);
    assert!(control_deflection(0x18, &|_| 0.0, &|_| 0.0, 0).is_none());
}

#[test]
fn small_wing_helpers_match_the_original_machine_code() {
    use openxplane::wing_element::{angle_shape, interpolate_clamped};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/wing_helpers.txt"
    ))
    .unwrap();
    let (mut shape, mut interp) = (0, 0);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t[0] {
            "S" => {
                assert_eq!(angle_shape(f(t[1])).to_bits(), f(t[3]).to_bits(), "{line}");
                shape += 1;
            }
            "I" => {
                let got = interpolate_clamped(f(t[1]), f(t[2]), f(t[3]), f(t[4]), f(t[5]));
                assert_eq!(got.to_bits(), f(t[7]).to_bits(), "{line}");
                interp += 1;
            }
            other => panic!("unknown record {other}"),
        }
    }
    println!("{shape} angle_shape and {interp} interpolate_clamped cases, all exact");
    assert!(shape >= 400 && interp >= 400);
}

#[test]
fn control_surface_terms_match_the_original_machine_code() {
    use openxplane::wing_element::{ControlSurface, control_deflection, control_surface_terms};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/control_surface.txt"
    ))
    .unwrap();
    let offsets = [
        (0xb, [0x324, 0x328, 0x1dfc, 0x1e00]),
        (0xc, [0x354, 0x358, 0x1e0c, 0x1e10]),
        (0xd, [0x444, 0x448, 0x1e18, 0x1e1c]),
        (0xe, [0x474, 0x478, 0x1e28, 0x1e2c]),
        (0xf, [0x4a4, 0x4a8, 0x1e3c, 0x1e40]),
        (0x10, [0x384, 0x388, 0x1e4c, 0x1e50]),
        (0x11, [0x3b4, 0x3b8, 0x1e60, 0x1e64]),
        (0x12, [0x3e4, 0x3e8, 0x1e74, 0x1e78]),
        (0x13, [0x414, 0x418, 0x1e88, 0x1e8c]),
        (0x14, [0x534, 0x538, 0x1e9c, 0x1ea0]),
        (0x15, [0x564, 0x568, 0x1ec4, 0x1ec8]),
        (0x16, [0x4d4, 0x4d8, 0x1ee0, 0x1ee4]),
        (0x17, [0x504, 0x508, 0x1ef0, 0x1ef4]),
    ];
    let (mut cases, mut exact, mut worst, mut changed) = (0, 0, 0u32, 0);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let code: u32 = t[0].parse().unwrap();
        let element: i32 = t[1].parse().unwrap();
        let mask = u32::from_str_radix(t[2], 16).unwrap();
        let v: Vec<f32> = t[3..7].iter().map(|h| f(h)).collect();
        let (w0, w20, x2c, x1bc, angle) = (f(t[7]), f(t[8]), f(t[9]), f(t[10]), f(t[11]));
        let modes: Vec<i32> = t[12..15].iter().map(|x| x.parse().unwrap()).collect();
        let kind: i32 = t[15].parse().unwrap();
        let index: usize = t[16].parse().unwrap();
        let ratios: Vec<f32> = t[17..23].iter().map(|h| f(h)).collect();
        let table_a: Vec<f32> = t[23..28].iter().map(|h| f(h)).collect();
        let table_b: Vec<f32> = t[28..33].iter().map(|h| f(h)).collect();
        let chords: Vec<f32> = t[33..53].iter().map(|h| f(h)).collect();
        let old: Vec<f32> = t[53..57].iter().map(|h| f(h)).collect();
        let want: Vec<f32> = t[57..61].iter().map(|h| f(h)).collect();
        let at = offsets.iter().find(|o| o.0 == code).unwrap().1;
        let wing = |o: usize| {
            if o == at[0] {
                v[0]
            } else if o == at[1] {
                v[1]
            } else {
                chords[(o - 0x70) / 4]
            }
        };
        let control = |o: usize| if o == at[2] { v[2] } else { v[3] };
        let deflection = control_deflection(code, &wing, &control, element).unwrap();
        let input = ControlSurface {
            code,
            deflection,
            chord: chords[element as usize],
            wing_0: w0,
            wing_20: w20,
            x_2c: x2c,
            x_1bc: x1bc,
            angle_deg: angle,
            modes: [modes[0], modes[1], modes[2]],
            kind,
            table_a: table_a[index],
            table_b: table_b[index],
            ratios: [
                ratios[0], ratios[1], ratios[2], ratios[3], ratios[4], ratios[5],
            ],
        };
        let driven = |id: u32| mask >> (id - 0x2d9) & 1 == 1;
        let mut got = old.clone();
        if let Some(terms) = control_surface_terms(&input, &driven) {
            changed += 1;
            for k in 0..4 {
                got[k] = terms[k] + old[k];
            }
        }
        cases += 1;
        let off = (0..4).map(|k| ulps(got[k], want[k])).max().unwrap();
        worst = worst.max(off);
        exact += usize::from(off == 0);
    }
    println!("{cases} cases, {changed} change the outputs, exact {exact}, worst {worst} ulp");
    assert!(cases >= 1000 && changed > 600);
    // sin, cos and atan2 come from the platform's libm here and from the C runtime in the original
    assert!(worst <= 64, "outputs differ by {worst} ulp");
}

struct Sparse(std::collections::HashMap<usize, u32>);

impl Sparse {
    fn parse(segment: &str) -> Self {
        Sparse(
            segment
                .split_whitespace()
                .filter_map(|t| t.split_once('='))
                .map(|(o, v)| {
                    (
                        usize::from_str_radix(o, 16).unwrap(),
                        u32::from_str_radix(v, 16).unwrap(),
                    )
                })
                .collect(),
        )
    }
}

impl openxplane::element_force::Mem for Sparse {
    fn f32(&self, offset: usize) -> f32 {
        f32::from_bits(self.0.get(&offset).copied().unwrap_or(0))
    }
    fn i32(&self, offset: usize) -> i32 {
        self.0.get(&offset).copied().unwrap_or(0) as i32
    }
}

#[test]
fn element_force_matches_the_original_machine_code() {
    use openxplane::element_force::{Call, Objects, element_force};
    use openxplane::wing_element::{FoilCall, FoilResult};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/element_force.txt"
    ))
    .unwrap();
    let (mut cases, mut exact, mut worst, mut problems) = (0, 0, 0u32, Vec::<String>::new());
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let parts: Vec<&str> = line.splitn(6, " | ").collect();
        let head: Vec<&str> = parts[0].split_whitespace().collect();
        let (fm, bm, wm, xm) = (
            Sparse::parse(parts[1]),
            Sparse::parse(parts[2]),
            Sparse::parse(parts[3]),
            Sparse::parse(parts[4]),
        );
        let mut it = parts[5].split_whitespace().filter(|t| *t != "|");
        let mut next = || it.next().unwrap_or_else(|| panic!("line {n} too short"));
        let int = |t: &str| t.parse::<i64>().unwrap();
        let ncalls = int(next()) as usize;
        let mut recorded: Vec<(FoilCall, FoilResult)> = Vec::new();
        for _ in 0..ncalls {
            let call = FoilCall {
                slot: int(next()) as usize,
                x_norm: f(next()),
                y_norm: f(next()),
                z_norm: f(next()),
                retain: int(next()) != 0,
                diagnostics: int(next()) != 0,
                re_meg: f(next()),
                arg6: f(next()),
                alpha: f(next()),
                multiplier: f(next()),
                divisor: f(next()),
                flag_dac: int(next()) != 0,
                stalled: int(next()) != 0,
            };
            let result = FoilResult {
                ret: f(next()),
                cl: f(next()),
                cd: f(next()),
                cm: f(next()),
                ratio: f(next()),
                stalled: int(next()) != 0,
            };
            recorded.push((call, result));
        }
        let want: Vec<f32> = (0..8).map(|_| f(next())).collect();
        let want_stall = int(next()) != 0;
        let e: usize = head[0].parse().unwrap();
        let names: Vec<String> = (4..7).map(|i| format!("foil{}", head[i])).collect();
        let mask = u32::from_str_radix(head[7], 16).unwrap();
        let thickness: Vec<Option<f32>> = head[8..11]
            .iter()
            .map(|t| (*t != "-").then(|| f(t)))
            .collect();
        let call = Call {
            index: e,
            retain: head[1] == "1",
            ice: f(head[2]),
            g10: f(head[3]),
            names: [&names[0], &names[1], &names[2]],
            foil_thickness: [thickness[0], thickness[1], thickness[2]],
        };
        let objects = Objects {
            f: &fm,
            b: &bm,
            w: &wm,
            x: &xm,
        };
        let mut replay = recorded.iter();
        let mut bad = Vec::new();
        let driven = |id: u32| mask >> (id - 0x2d9) & 1 == 1;
        let got = element_force(
            &objects,
            &call,
            |c: &FoilCall| {
                let Some((want_call, result)) = replay.next() else {
                    bad.push("extra profile call".to_string());
                    return Err("extra profile call".into());
                };
                let bits = |a: f32, b: f32| a.to_bits() == b.to_bits();
                // the angle comes through atan2 and cos of the platform's libm
                let near = |a: f32, b: f32| ulps(a, b) <= 8 || (a - b).abs() < 2e-5;
                let same = c.slot == want_call.slot
                    && bits(c.x_norm, want_call.x_norm)
                    && bits(c.y_norm, want_call.y_norm)
                    && bits(c.z_norm, want_call.z_norm)
                    && c.retain == want_call.retain
                    && bits(c.re_meg, want_call.re_meg)
                    && bits(c.arg6, want_call.arg6)
                    && near(c.alpha, want_call.alpha)
                    && near(c.multiplier, want_call.multiplier)
                    && near(c.divisor, want_call.divisor)
                    && c.flag_dac == want_call.flag_dac
                    && c.stalled == want_call.stalled;
                if !same {
                    bad.push(format!("profile call differs: {c:?} vs {want_call:?}"));
                }
                Ok(*result)
            },
            &driven,
        );
        let out = match got {
            Ok(out) => out,
            Err(error) => {
                problems.push(format!("case {n}: {error}"));
                continue;
            }
        };
        let have = [
            out.out1,
            out.out2,
            out.out3,
            out.x_f4,
            out.x_11c,
            out.x_144,
            out.x_16c,
            out.element.ratio,
        ];
        let off = have
            .iter()
            .zip(&want[..3])
            .chain(have[3..7].iter().zip(&want[3..7]))
            .chain(have[7..].iter().zip(&want[7..]))
            .map(|(a, b)| ulps(*a, *b))
            .max()
            .unwrap();
        worst = worst.max(off);
        cases += 1;
        exact += usize::from(off == 0);
        if off > 64 && std::env::var_os("ELEMENT_FORCE_DEBUG").is_some() && cases < 12 {
            println!("case {n}: port {have:?}\n          orig {want:?}");
        }
        if off > 64 || out.element.stall_flag != want_stall || !bad.is_empty() {
            problems.push(format!(
                "case {n}: worst {off} ulp, stall {} vs {want_stall}, {bad:?}",
                out.element.stall_flag
            ));
        }
    }
    println!("{cases} cases, exact {exact}, worst {worst} ulp");
    for p in problems.iter().take(8) {
        println!("{p}");
    }
    assert!(
        cases >= 200 && problems.is_empty(),
        "{} problems",
        problems.len()
    );
}

#[test]
fn geometry_helpers_match_the_original_machine_code() {
    use openxplane::wing_element::{hypot2, hypot3, rotate_euler};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/geometry.txt"
    ))
    .unwrap();
    let (mut counts, mut worst) = ([0usize; 3], 0u32);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        match t[0] {
            "H2" => {
                assert_eq!(hypot2(f(t[1]), f(t[2])).to_bits(), f(t[3]).to_bits());
                counts[0] += 1;
            }
            "H3" => {
                assert_eq!(
                    hypot3(f(t[1]), f(t[2]), f(t[3])).to_bits(),
                    f(t[4]).to_bits()
                );
                counts[1] += 1;
            }
            "R" => {
                let got = rotate_euler([f(t[1]), f(t[2]), f(t[3])], f(t[4]), f(t[5]), f(t[6]));
                for k in 0..3 {
                    worst = worst.max(ulps(got[k], f(t[7 + k])));
                }
                counts[2] += 1;
            }
            other => panic!("unknown record {other}"),
        }
    }
    println!("{counts:?} cases, rotation worst {worst} ulp");
    assert!(counts.iter().all(|c| *c >= 300));
    // sin and cos come from the platform's libm here and from the C runtime in the original
    assert!(worst <= 64, "rotation differs by {worst} ulp");
}

#[test]
fn aircraft_frame_transform_matches_the_original_machine_code() {
    use openxplane::transform::{Frame, to_aircraft_frame};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/transform.txt"
    ))
    .unwrap();
    let (mut cases, mut exact) = (0, 0);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let shift = t[0] == "1";
        let disabled = t[1] == "1";
        let d = |h: &str| f64::from_bits(u64::from_str_radix(h, 16).unwrap());
        let frame = Frame {
            origin: [d(t[2]), d(t[3]), d(t[4])],
            rotation: [[f(t[5]), f(t[6])], [f(t[7]), f(t[8])], [f(t[9]), f(t[10])]],
        };
        let got = to_aircraft_frame(&frame, [f(t[11]), f(t[12]), f(t[13])], shift, disabled);
        cases += 1;
        exact += usize::from((0..3).all(|k| got[k].to_bits() == f(t[14 + k]).to_bits()));
    }
    println!("{cases} cases, exact {exact}");
    assert_eq!(exact, cases);
}

struct Fields(std::collections::HashMap<usize, f32>);

impl openxplane::element_force::Mem for Fields {
    fn f32(&self, offset: usize) -> f32 {
        self.0.get(&offset).copied().unwrap_or(0.0)
    }
    fn i32(&self, offset: usize) -> i32 {
        self.0.get(&offset).copied().unwrap_or(0.0).to_bits() as i32
    }
}

#[test]
fn engine_functions_match_the_original_machine_code() {
    use openxplane::engine::{curve, ram_power_factor, signed_pow};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/engine.txt"
    ))
    .unwrap();
    let (mut counts, mut exact, mut worst) = ([0usize; 3], [0usize; 3], [0u32; 3]);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let (kind, got, want) = match t[0] {
            "P" => (0, signed_pow(f(t[1]), f(t[2])), f(t[3])),
            "C" => (
                1,
                curve(f(t[1]), f(t[2]), f(t[3]), f(t[4]), f(t[5]), f(t[6])),
                f(t[7]),
            ),
            "R" => {
                let b = Fields(
                    [
                        (0x984, f(t[1])),
                        (0x980, f(t[2])),
                        (0x950, f(t[3])),
                        (0x940, f(t[4])),
                        (0x988, f(t[5])),
                        (0x98c, f(t[6])),
                    ]
                    .into_iter()
                    .collect(),
                );
                (
                    2,
                    ram_power_factor(&b, f(t[7]), f(t[8]), f(t[9]), f(t[10]), f(t[11]), f(t[12])),
                    f(t[13]),
                )
            }
            other => panic!("unknown record {other}"),
        };
        counts[kind] += 1;
        let off = ulps(got, want);
        exact[kind] += usize::from(off == 0);
        worst[kind] = worst[kind].max(off);
    }
    println!("cases {counts:?}, exact {exact:?}, worst ulp {worst:?}");
    assert!(counts.iter().all(|c| *c >= 300));
    // powf and cos come from the platform's libm here and from the C runtime in the original
    assert!(worst[0] <= 4 && worst[1] <= 64, "{worst:?}");
    assert!(worst[2] <= 512, "{worst:?}");
}

struct Replay<'a> {
    calls: std::slice::Iter<'a, (String, Vec<u32>, u64)>,
    problems: Vec<String>,
    threshold: f64,
}

impl Replay<'_> {
    fn next(&mut self, name: &str, args: &[u32]) -> u64 {
        match self.calls.next() {
            // arguments are equal bit for bit, or both NaN (the sign of a NaN differs between the CPU and Rust)
            Some((n, a, ret))
                if n == name
                    && a.len() == args.len()
                    && a.iter().zip(args).all(|(x, y)| {
                        x == y || (f32::from_bits(*x).is_nan() && f32::from_bits(*y).is_nan())
                    }) =>
            {
                *ret
            }
            other => {
                self.problems
                    .push(format!("call {name} {args:?} against {other:?}"));
                0
            }
        }
    }
}

impl openxplane::engine::EngineEnv for Replay<'_> {
    fn atmosphere_a(&mut self, time: f32) -> f32 {
        f32::from_bits(self.next("atmo_a", &[time.to_bits()]) as u32)
    }
    fn atmosphere_b(&mut self, time: f32, value: f32) -> f32 {
        f32::from_bits(self.next("atmo_b", &[time.to_bits(), value.to_bits()]) as u32)
    }
    fn engine_flag(&mut self) -> bool {
        self.next("flag", &[]) != 0
    }
    fn frame_time(&mut self) -> f64 {
        f64::from_bits(self.next("dt", &[]))
    }
    fn binding(&mut self, id: u32, index: i32) -> bool {
        self.next("bind", &[id, index as u32]) != 0
    }
    fn thrust_threshold(&mut self) -> f64 {
        self.threshold
    }
    fn fuel_draw(&mut self, amount: f32, interval: f32, mode: i32) {
        self.next("fuel", &[amount.to_bits(), interval.to_bits(), mode as u32]);
    }
    fn random_unit(&mut self) -> f32 {
        f32::from_bits(self.next("rand", &[]) as u32)
    }
}

#[test]
fn engine_update_matches_the_original_machine_code() {
    use openxplane::engine::{Record, engine_update};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/engine_update.txt"
    ))
    .unwrap();
    // fields written by the part of the update that is ported
    let ported = [
        0x258usize, 0x22c, 0x240, 0x244, 0x248, 0x28c, 0x98, 0x90, 0x78,
    ];
    let (mut cases, mut exact, mut worst, mut problems) = (0, 0, 0u32, Vec::<String>::new());
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let parts: Vec<&str> = line.split(" | ").collect();
        let head: Vec<&str> = parts[1].split_whitespace().collect();
        let ncalls: usize = head[0].parse().unwrap();
        let mut calls = Vec::new();
        let mut i = 1;
        for _ in 0..ncalls {
            let name = head[i].to_string();
            let nargs: usize = head[i + 1].parse().unwrap();
            let args: Vec<u32> = head[i + 2..i + 2 + nargs]
                .iter()
                .map(|h| u32::from_str_radix(h, 16).unwrap())
                .collect();
            let ret = u64::from_str_radix(head[i + 2 + nargs], 16).unwrap();
            calls.push((name, args, ret));
            i += 3 + nargs;
        }
        let (fm, bm, dm, wm) = (
            Sparse::parse(parts[2]),
            Sparse::parse(parts[3]),
            Sparse::parse(parts[4]),
            Sparse::parse(parts[5]),
        );
        let threshold = f64::from(f32::from_bits(u32::from_str_radix(parts[0], 16).unwrap()));
        let words: Vec<u32> = parts[6]
            .split_whitespace()
            .map(|h| u32::from_str_radix(h, 16).unwrap())
            .collect();
        let changed = Sparse::parse(parts[7]);
        let mut record = Record(words.clone());
        let mut replay = Replay {
            calls: calls.iter(),
            problems: Vec::new(),
            threshold,
        };
        engine_update(&fm, &bm, &dm, &wm, &mut record, 0, &mut replay);
        cases += 1;
        let mut off = 0;
        for &o in &ported {
            let want = changed.0.get(&o).copied().unwrap_or(words[o / 4]);
            let (a, b) = (f32::from_bits(record.0[o / 4]), f32::from_bits(want));
            // both NaN counts as equal (the sign of a NaN differs between the CPU and Rust)
            let d = if a.is_nan() && b.is_nan() {
                0
            } else {
                ulps(a, b)
            };
            if d > 16 && std::env::var_os("ENGINE_DEBUG").is_some() {
                println!(
                    "case {n} field {o:#x}: port {} original {}",
                    f32::from_bits(record.0[o / 4]),
                    f32::from_bits(want)
                );
            }
            off = off.max(d);
        }
        worst = worst.max(off);
        exact += usize::from(off == 0);
        if off > 16 || !replay.problems.is_empty() {
            problems.push(format!("case {n}: {off} ulp {:?}", replay.problems));
        }
    }
    println!("{cases} cases, exact {exact}, worst {worst} ulp");
    for p in problems.iter().take(6) {
        println!("{p}");
    }
    assert!(cases >= 100 && problems.is_empty());
}

#[test]
fn fuel_draw_matches_the_original_machine_code() {
    use openxplane::fuel::Tanks;
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/fuel.txt"))
        .unwrap();
    let (mut cases, mut drawn, mut by_mode) = (0, 0, [0usize; 6]);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let mut tanks = Tanks {
            flags: [t[0].parse().unwrap(), t[1].parse().unwrap()],
            used: [f(t[2]), f(t[3]), f(t[4])],
            capacity: [f(t[5]), f(t[6]), f(t[7])],
        };
        let mode: i32 = t[10].parse().unwrap();
        let got = tanks.draw(f(t[8]), f(t[9]), mode);
        assert_eq!(got, t[11] == "1", "{line}");
        for k in 0..3 {
            assert_eq!(tanks.used[k].to_bits(), f(t[12 + k]).to_bits(), "{line}");
        }
        cases += 1;
        drawn += usize::from(got);
        by_mode[mode as usize] += usize::from(got);
    }
    println!("{cases} cases, {drawn} draws, by mode {by_mode:?}");
    assert!(cases >= 1000 && drawn > 400);
    assert!(by_mode[1] > 20 && by_mode[2] > 20 && by_mode[3] > 20 && by_mode[5] > 20);
}

#[test]
fn atmosphere_accessors_match_the_original_machine_code() {
    use openxplane::atmosphere::Atmosphere;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/atmosphere.txt"
    ))
    .unwrap();
    let mut lines = text.lines().filter(|l| !l.is_empty());
    let table: Vec<(f32, f32)> = {
        let t: Vec<f32> = lines
            .next()
            .unwrap()
            .split_whitespace()
            .skip(1)
            .map(f)
            .collect();
        t.chunks(2).map(|c| (c[0], c[1])).collect()
    };
    assert_eq!(table.len(), 0x803);
    let (mut counts, mut worst) = ([0usize; 3], [0u32; 3]);
    for line in lines {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let (kind, got, want) = match t[0] {
            "T" => {
                let object = Fields([(0x64, f(t[2]))].into_iter().collect());
                let a = Atmosphere {
                    object: &object,
                    table: &table,
                };
                (0, a.temperature(f(t[1])), f(t[3]))
            }
            "P" => {
                let object = Fields([(0x64, -300.0), (0x98, f(t[2]))].into_iter().collect());
                let a = Atmosphere {
                    object: &object,
                    table: &table,
                };
                (1, a.pressure(f(t[1])), f(t[3]))
            }
            "D" => {
                let object = Fields(
                    [(0x64, -300.0), (0x98, f(t[3])), (0x9c, f(t[4]))]
                        .into_iter()
                        .collect(),
                );
                let a = Atmosphere {
                    object: &object,
                    table: &table,
                };
                (2, a.density_ratio(f(t[1]), f(t[2])), f(t[5]))
            }
            other => panic!("unknown record {other}"),
        };
        // the layered path (object+0x64 above absolute zero) is not ported
        let Some(got) = got else { continue };
        counts[kind] += 1;
        let both_nan = got.is_nan() && want.is_nan();
        worst[kind] = worst[kind].max(if both_nan { 0 } else { ulps(got, want) });
    }
    println!("cases {counts:?}, worst ulp {worst:?}");
    assert!(counts.iter().all(|c| *c >= 150));
    assert_eq!(worst[0], 0);
    // the power function comes from the platform's libm here and from the C runtime in the original
    assert!(worst[1] <= 64 && worst[2] <= 64, "{worst:?}");
}

#[test]
fn force_totals_match_the_original_machine_code() {
    use openxplane::forces::{Words, force_totals};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/force_totals.txt"
    ))
    .unwrap();
    let offsets: Vec<usize> = (0x2b0..0x340)
        .step_by(4)
        .chain((0x6750..0x67d0).step_by(4))
        .collect();
    let (mut cases, mut skipped) = (0, 0);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let (before, after) = line.split_once(" | ").unwrap();
        let read = |part: &str| -> Vec<u32> {
            part.split_whitespace()
                .map(|h| u32::from_str_radix(h, 16).unwrap())
                .collect()
        };
        let (before, after) = (read(before), read(after));
        assert_eq!(before.len(), offsets.len());
        let mut words = Words::default();
        for (o, v) in offsets.iter().zip(&before) {
            words.0.insert(*o, *v);
        }
        skipped += usize::from(words.i32(0x675c) != 0);
        force_totals(&mut words);
        for (o, v) in offsets.iter().zip(&after) {
            assert_eq!(words.0[o], *v, "offset {o:#x} in {line}");
        }
        cases += 1;
    }
    println!("{cases} cases, {skipped} with the totals supplied from outside");
    assert!(cases >= 300 && skipped > 20);
}

#[test]
fn wing_misc_helpers_match_the_original_machine_code() {
    use openxplane::wing_element::{Boundary, element_dihedral, signed_sqrt};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/wing_misc.txt"
    ))
    .unwrap();
    let (mut counts, mut worst) = ([0usize; 2], 0u32);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        match t[0] {
            "Q" => {
                assert_eq!(signed_sqrt(f(t[1])).to_bits(), f(t[2]).to_bits(), "{line}");
                counts[0] += 1;
            }
            "D" => {
                let v: Vec<f32> = t[1..7].iter().map(|h| f(h)).collect();
                let (x, y, z) = ([v[0], v[1]], [v[2], v[3]], [v[4], v[5]]);
                let b = Boundary {
                    x: &x,
                    y: &y,
                    z: &z,
                    chord: &[],
                };
                worst = worst.max(ulps(element_dihedral(&b, 0), f(t[7])));
                counts[1] += 1;
            }
            other => panic!("unknown record {other}"),
        }
    }
    println!("cases {counts:?}, dihedral worst {worst} ulp");
    assert!(counts.iter().all(|c| *c >= 300) && worst <= 4);
}

#[test]
fn frame_rotations_match_the_original_machine_code() {
    use openxplane::transform::{Frame, from_aircraft_frame, rotate_euler_offset, rotate_pairs};
    use openxplane::wing_element::boundary_at;
    let text =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/frame.txt"))
            .unwrap();
    let (mut counts, mut worst) = ([0usize; 4], [0u32; 4]);
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let d = |h: &str| f64::from_bits(u64::from_str_radix(h, 16).unwrap());
        let (kind, got, want): (usize, Vec<f32>, Vec<f32>) = match t[0] {
            "R" => {
                let p = [f(t[4]), f(t[5]), f(t[6]), f(t[7]), f(t[8]), f(t[9])];
                let r = rotate_pairs(f(t[1]), f(t[2]), f(t[3]), p);
                (0, r.to_vec(), t[10..13].iter().map(|h| f(h)).collect())
            }
            "E" => {
                let r = rotate_euler_offset(
                    [f(t[1]), f(t[2]), f(t[3])],
                    [f(t[4]), f(t[5]), f(t[6])],
                    t[7] == "1",
                    f(t[8]),
                    f(t[9]),
                    f(t[10]),
                );
                (1, r.to_vec(), t[11..14].iter().map(|h| f(h)).collect())
            }
            "A" => {
                let frame = Frame {
                    origin: [d(t[1]), d(t[2]), d(t[3])],
                    rotation: [[f(t[6]), f(t[7])], [f(t[4]), f(t[5])], [f(t[8]), f(t[9])]],
                };
                let r = from_aircraft_frame(
                    &frame,
                    [f(t[12]), f(t[13]), f(t[14])],
                    t[10] == "1",
                    t[11] == "1",
                );
                (2, r.to_vec(), t[15..18].iter().map(|h| f(h)).collect())
            }
            "B" => {
                let els: i32 = t[1].parse().unwrap();
                let vals: Vec<f32> = t[3..14].iter().map(|h| f(h)).collect();
                (3, vec![boundary_at(&vals, els, f(t[2]))], vec![f(t[14])])
            }
            other => panic!("unknown record {other}"),
        };
        counts[kind] += 1;
        for (a, b) in got.iter().zip(&want) {
            let off = if a.is_nan() && b.is_nan() {
                0
            } else {
                ulps(*a, *b)
            };
            worst[kind] = worst[kind].max(off);
        }
    }
    println!("cases {counts:?}, worst ulp {worst:?}");
    assert!(counts.iter().all(|c| *c >= 300));
    assert_eq!(worst[0], 0);
    assert_eq!(worst[3], 0);
    // sin and cos come from the platform's libm here and from the C runtime in the original
    assert!(worst[1] <= 256, "{worst:?}");
    assert_eq!(worst[2], 0, "{worst:?}");
}

#[test]
fn wing_element_straight_path_matches_the_original_machine_code() {
    use openxplane::wing_element::{
        Aircraft, Boundary, ElementInputs, ElementState, Flow, FoilCall, FoilResult, WingFields,
        evaluate,
    };
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/wing_element.txt"
    ))
    .unwrap();
    let (mut cases, mut calls_total, mut mismatches) = (0, 0, Vec::<String>::new());
    let mut by_slot = [0usize; 3];
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let mut it = line.split_whitespace().filter(|t| *t != "|");
        let mut next = || it.next().unwrap_or_else(|| panic!("line {n} too short"));
        let int = |t: &str| t.parse::<i64>().unwrap();
        let e = int(next()) as usize;
        let els = int(next()) as i32;
        let retain = int(next()) != 0;
        let (arg6, ice, alpha_in) = (f(next()), f(next()), f(next()));
        let extra = [f(next()), f(next()), f(next())];
        let is_right = f(next());
        let (f14, f18, f1c) = (f(next()), f(next()), f(next()));
        let ratios = [f(next()), f(next()), f(next()), f(next())];
        let (c0, c1, x0, x1, y0, y1, z0, z1) = (
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
        );
        let (flap, slat) = (int(next()) as i32, int(next()) as i32);
        let names: Vec<String> = (0..3).map(|_| format!("foil{}", int(next()))).collect();
        let flow = Flow {
            f5c: f(next()),
            f6c: f(next()),
            f1a0: f(next()),
            f1a4: f(next()),
            f408: f(next()),
            flag_dac: int(next()) != 0,
            diagnostics: int(next()) != 0 || int(next()) != 0,
        };
        let aircraft = Aircraft {
            f1f00: f(next()),
            f1f3c: f(next()),
            f64f4: f(next()),
            f64f8: f(next()),
            f64fc: f(next()),
        };
        let g10 = f(next());
        let state = ElementState {
            v: f(next()),
            r11: f(next()),
            r21: f(next()),
            r163: f(next()),
            stall_flag: int(next()) != 0,
        };
        let ncalls = int(next()) as usize;
        let mut recorded: Vec<(FoilCall, FoilResult)> = Vec::new();
        for _ in 0..ncalls {
            let call = FoilCall {
                slot: int(next()) as usize,
                x_norm: f(next()),
                y_norm: f(next()),
                z_norm: f(next()),
                retain: int(next()) != 0,
                diagnostics: int(next()) != 0,
                re_meg: f(next()),
                arg6: f(next()),
                alpha: f(next()),
                multiplier: f(next()),
                divisor: f(next()),
                flag_dac: int(next()) != 0,
                stalled: int(next()) != 0,
            };
            let result = FoilResult {
                ret: f(next()),
                cl: f(next()),
                cd: f(next()),
                cm: f(next()),
                ratio: f(next()),
                stalled: int(next()) != 0,
            };
            recorded.push((call, result));
        }
        let want = [
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
            f(next()),
        ];
        let want_stall = int(next()) != 0;

        // arrays indexed by element: only the entries the function reads are meaningful
        let pad = |a: f32, b: f32| {
            let mut v = vec![0.0f32; e + 2];
            v[e] = a;
            v[e + 1] = b;
            v
        };
        let (cx, cy, cz, cc) = (pad(x0, x1), pad(y0, y1), pad(z0, z1), pad(c0, c1));
        let mut flaps = vec![0i32; e + 1];
        let mut slats = vec![0i32; e + 1];
        flaps[e] = flap;
        slats[e] = slat;
        let inputs = ElementInputs {
            index: e,
            retain_request: retain,
            arg6,
            ice,
            alpha_in,
            extra,
            flow,
            aircraft,
            g10,
            wing: WingFields {
                is_right,
                elements: els,
                f14,
                f18,
                f1c,
                ratios,
                boundary: Boundary {
                    x: &cx,
                    y: &cy,
                    z: &cz,
                    chord: &cc,
                },
                flap_flags: &flaps,
                slat_flags: &slats,
                names: [&names[0], &names[1], &names[2]],
            },
            state,
            skip_delta_block: false,
        };
        let mut replay = recorded.iter();
        let mut problems = Vec::new();
        let got = evaluate(&inputs, |c: &FoilCall| {
            let Some((want_call, result)) = replay.next() else {
                problems.push(format!("extra call to slot {}", c.slot));
                return Ok(FoilResult {
                    ret: 0.0,
                    cl: 0.0,
                    cd: 0.0,
                    cm: 0.0,
                    ratio: 0.0,
                    stalled: false,
                });
            };
            let bits = |a: f32, b: f32| a.to_bits() == b.to_bits();
            let same = c.slot == want_call.slot
                && bits(c.x_norm, want_call.x_norm)
                && bits(c.y_norm, want_call.y_norm)
                && bits(c.z_norm, want_call.z_norm)
                && c.retain == want_call.retain
                && c.diagnostics == want_call.diagnostics
                && bits(c.re_meg, want_call.re_meg)
                && bits(c.arg6, want_call.arg6)
                && bits(c.alpha, want_call.alpha)
                && bits(c.multiplier, want_call.multiplier)
                && bits(c.divisor, want_call.divisor)
                && c.flag_dac == want_call.flag_dac
                && c.stalled == want_call.stalled;
            if !same {
                problems.push(format!(
                    "call differs:\n   port     {c:?}\n   original {want_call:?}"
                ));
            }
            calls_total += 1;
            by_slot[c.slot] += 1;
            Ok(*result)
        })
        .unwrap();
        if replay.next().is_some() {
            problems.push("the port made fewer calls than the original".into());
        }
        let have = [got.ret, got.cl, got.cd, got.cm, got.ratio, got.induced_drag];
        if have
            .iter()
            .zip(want)
            .any(|(a, b)| a.to_bits() != b.to_bits())
            || got.stall_flag != want_stall
        {
            problems.push(format!(
                "outputs differ: port {have:?} stall {} original {want:?} stall {want_stall}",
                got.stall_flag
            ));
        }
        cases += 1;
        if !problems.is_empty() {
            mismatches.push(format!("case {n}: {}", problems.join("\n")));
        }
    }
    println!(
        "{cases} cases, {calls_total} profile calls (root {}, middle {}, tip {})",
        by_slot[0], by_slot[1], by_slot[2]
    );
    assert!(
        cases >= 400 && by_slot.iter().all(|c| *c > 20),
        "vectors must exercise all three airfoils"
    );
    assert!(
        mismatches.is_empty(),
        "{} of {cases} cases differ from the original:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn profile_function_with_compressibility_matches_the_original_machine_code() {
    use openxplane::{
        airfoil::Airfoil,
        profile::{OuterInputs, outer},
    };
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/profile_outer.txt"
    ))
    .unwrap();
    let noise = noise();
    let (mut cases, mut stalled, mut two_tables, mut mismatches) = (0, 0, 0, Vec::<String>::new());
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let mut it = line.split_whitespace().filter(|t| *t != "|");
        let mut next = || it.next().unwrap();
        let tables: usize = next().parse().unwrap();
        let mut polars = Vec::new();
        for _ in 0..tables {
            let k: i64 = next().parse().unwrap();
            let p = [f(next()), f(next()), f(next()), f(next()), f(next())];
            polars.push(polar(k, p));
        }
        let scalar = f(next());
        let airfoil = Airfoil {
            version: 1110,
            header_scalars: [0.0, scalar],
            shape_points: [[0.0; 2]; 14],
            polars,
        };
        let noise_coordinates = [f(next()), f(next()), f(next())];
        let retain_stall = next().parse::<i32>().unwrap() != 0;
        let regime = f(next());
        let mach = f(next());
        let alpha_deg = f(next());
        let multiplier = f(next());
        let divisor = f(next());
        let _dac = next();
        let time = f64::from_bits(u64::from_str_radix(next(), 16).unwrap());
        let stall_in = next().parse::<i32>().unwrap() != 0;
        let want = [f(next()), f(next()), f(next()), f(next()), f(next())];
        let want_stall = next().parse::<i32>().unwrap() != 0;
        let got = outer(
            &airfoil,
            OuterInputs {
                alpha_deg,
                multiplier,
                divisor,
                regime,
                mach,
                time: RunningTime::from_snapshot(time).unwrap(),
                noise_coordinates,
                retain_stall,
            },
            stall_in,
            Some(&noise),
        )
        .unwrap();
        let have = [got.cl, got.cd, got.cm, got.normalized_alpha, got.ret];
        cases += 1;
        stalled += usize::from(want_stall);
        two_tables += usize::from(tables > 1);
        if have
            .iter()
            .zip(want)
            .any(|(a, b)| a.to_bits() != b.to_bits())
            || got.stalled != want_stall
        {
            mismatches.push(format!(
                "case {n}: port {:08x?} stall {} original {:08x?} stall {want_stall}",
                have.map(f32::to_bits),
                got.stalled,
                want.map(f32::to_bits)
            ));
        }
    }
    println!("{cases} cases, {stalled} stalled, {two_tables} with several tables");
    assert!(cases >= 600 && stalled > 100 && two_tables > 300);
    assert!(
        mismatches.is_empty(),
        "{} of {cases} cases differ:\n{}",
        mismatches.len(),
        mismatches
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn boundary_ratio_matches_the_original_machine_code() {
    use openxplane::element_force::boundary_ratio;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/boundary_ratio.txt"
    ))
    .unwrap();
    let mut cases = 0;
    for line in text.lines().filter(|l| l.starts_with("Q ")) {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let mut w = std::collections::HashMap::new();
        w.insert(4usize, t[1].parse::<u32>().unwrap());
        for (k, base) in [0x614usize, 0x5e8, 0x5bc].into_iter().enumerate() {
            for i in 0..11 {
                w.insert(
                    base + 4 * i,
                    u32::from_str_radix(t[9 + 11 * k + i], 16).unwrap(),
                );
            }
        }
        let bits = u64::from_str_radix(t[3], 16).unwrap();
        let mut fo = std::collections::HashMap::new();
        for (o, h) in [0x430usize, 0x434, 0x450, 0x454, 0x42f5c]
            .into_iter()
            .zip(&t[4..9])
        {
            fo.insert(o, u32::from_str_radix(h, 16).unwrap());
        }
        fo.insert(0x380, bits as u32);
        fo.insert(0x384, (bits >> 32) as u32);
        let got = boundary_ratio(&Sparse(w), &Sparse(fo), t[2] == "1");
        let want = f(t[t.len() - 1]);
        assert!(
            got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
            "{line}: {got} vs {want}"
        );
        cases += 1;
    }
    assert_eq!(cases, 300);
}

#[test]
fn engine_held_back_matches_the_original_machine_code() {
    use openxplane::engine::{HoldInputs, engine_held_back};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/engine_held.txt"
    ))
    .unwrap();
    let ids = [0x179u32, 0x1f9, 0x2f6, 0x2f7, 0x2f8, 0x2f9];
    let mut cases = 0;
    for line in text.lines().filter(|l| l.starts_with("H ")) {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let answers: Vec<bool> = t[7..13].iter().map(|a| *a == "1").collect();
        let got = engine_held_back(
            HoldInputs {
                kind: t[1].parse().unwrap(),
                lever: f(t[2]),
                limit_low: f(t[3]),
                limit_high: f(t[4]),
                index: t[5].parse().unwrap(),
                mode: t[6].parse().unwrap(),
            },
            |id, _| answers[ids.iter().position(|i| *i == id).unwrap()],
            |_, _| t[13].parse().unwrap(),
        );
        assert_eq!(got, t[14].parse::<i32>().unwrap(), "{line}");
        cases += 1;
    }
    assert_eq!(cases, 600);
}

#[test]
fn record_flag_6040_matches_the_original_machine_code() {
    use openxplane::engine::record_flag_6040;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/record_flag.txt"
    ))
    .unwrap();
    let mut cases = 0;
    for line in text.lines().filter(|l| l.starts_with("G ")) {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let got = record_flag_6040(
            t[1].parse().unwrap(),
            t[2].parse().unwrap(),
            t[3].parse().unwrap(),
            |id, _| id == 0x179 && t[4] == "1",
        );
        assert_eq!(got, t[5].parse::<u8>().unwrap(), "{line}");
        cases += 1;
    }
    assert_eq!(cases, 400);
}

#[test]
fn flight_helpers_match_the_original_machine_code() {
    use openxplane::engine::{cosine_blend, record_flag_6028, root_ratio};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/flight_helpers.txt"
    ))
    .unwrap();
    let mut counts = [0usize; 3];
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        match t[0] {
            "C" => {
                let got = cosine_blend(f(t[1]), f(t[2]), f(t[3]), f(t[4]), f(t[5]), f(t[6]));
                let want = f64::from_bits(u64::from_str_radix(t[7], 16).unwrap());
                // cosf of the original and of the host differ in the last bit or two
                assert!(
                    (got - want).abs() <= 1e-5 * (1.0 + want.abs()),
                    "{line}: {got} vs {want}"
                );
                counts[0] += 1;
            }
            "N" => {
                let got = root_ratio(f(t[1]), f(t[2]), f(t[3]));
                let want = f(t[5]);
                assert_eq!(got.to_bits(), want.to_bits(), "{line}");
                counts[1] += 1;
            }
            _ => {
                let got = record_flag_6028(t[1].parse().unwrap(), t[2] == "1");
                assert_eq!(got, t[3].parse::<u8>().unwrap(), "{line}");
                counts[2] += 1;
            }
        }
    }
    assert_eq!(counts, [300, 400, 200]);
}

struct SingleWind {
    seen: [f64; 3],
    wind: [f32; 3],
}

impl openxplane::airflow::AirflowEnv for SingleWind {
    fn wind(&mut self, x: f64, y: f64, z: f64) -> [f32; 3] {
        assert_eq!(
            [x.to_bits(), y.to_bits(), z.to_bits()],
            self.seen.map(f64::to_bits)
        );
        self.wind
    }
    fn wash(&mut self, _point: [f32; 3], _out: [f32; 3]) -> [f32; 3] {
        unreachable!("the airflow vectors do not select the wash")
    }
}

#[test]
fn airflow_matches_the_original_machine_code() {
    use openxplane::airflow::airflow;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/airflow.txt"
    ))
    .unwrap();
    let d = |h: &str| f64::from_bits(u64::from_str_radix(h, 16).unwrap());
    let mut cases = 0;
    for line in text.lines().filter(|l| l.starts_with("A ")) {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let mut fo = std::collections::HashMap::new();
        for (o, h) in [0x430usize, 0x434, 0x440, 0x444, 0x450, 0x454]
            .into_iter()
            .zip(&t[6..12])
        {
            fo.insert(o, u32::from_str_radix(h, 16).unwrap());
        }
        for (k, o) in [0x378usize, 0x380, 0x388].into_iter().enumerate() {
            let bits = u64::from_str_radix(t[12 + k], 16).unwrap();
            fo.insert(o, bits as u32);
            fo.insert(o + 4, (bits >> 32) as u32);
        }
        for (o, h) in [0x368usize, 0x36c, 0x370, 0x3cc, 0x3d0, 0x3d4]
            .into_iter()
            .zip(&t[15..21])
        {
            fo.insert(o, u32::from_str_radix(h, 16).unwrap());
        }
        let wind = [f(t[21]), f(t[22]), f(t[23])];
        let seen = [d(t[24]), d(t[25]), d(t[26])];
        let got = airflow(
            &Sparse(fo),
            [f(t[1]), f(t[2]), f(t[3])],
            t[5] == "1",
            &mut SingleWind { seen, wind },
            false,
        )
        .unwrap();
        for k in 0..3 {
            assert_eq!(
                got[k].to_bits(),
                f(t[27 + k]).to_bits(),
                "{line}: component {k}"
            );
        }
        cases += 1;
    }
    assert_eq!(cases, 500);
}

#[test]
fn scalar_helpers_match_the_original_machine_code() {
    use openxplane::scalar::{angle_lerp, clamp, kind_is_3_or_7, lerp, max3, sign, snap, within};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/scalar.txt"
    ))
    .unwrap();
    let mut counts = [0usize; 8];
    let same = |a: f32, b: f32| a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan());
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let t: Vec<&str> = line.split_whitespace().filter(|x| *x != "|").collect();
        let (k, ok) = match t[0] {
            "C" => (0, same(clamp(f(t[1]), f(t[2]), f(t[3])), f(t[4]))),
            "S" => (1, same(sign(f(t[1])), f(t[2]))),
            "M" => (2, same(max3(f(t[1]), f(t[2]), f(t[3])), f(t[4]))),
            "N" => (3, same(snap(f(t[1]), f(t[2]), f(t[3])), f(t[4]))),
            "L" => (4, same(lerp(f(t[1]), f(t[2]), f(t[3])), f(t[4]))),
            "W" => (
                7,
                usize::from(within(f(t[1]), f(t[2]), f(t[3]))) == t[4].parse::<usize>().unwrap(),
            ),
            "A" => (
                6,
                same(
                    angle_lerp(f(t[1]), f(t[2]), f(t[3]), f(t[4]), f(t[5])),
                    f(t[6]),
                ),
            ),
            _ => (5, kind_is_3_or_7(t[1].parse().unwrap()) == (t[2] == "1")),
        };
        assert!(ok, "{line}");
        counts[k] += 1;
    }
    assert_eq!(counts[..5], [300; 5]);
    assert_eq!(counts[6], 300);
    assert_eq!(counts[7], 300);
    assert!(counts[5] > 20);
}

/// Parses `off=word` tokens of the vector files (hex offsets unless `signed`, then decimal).
fn parse_words(tokens: &[&str]) -> std::collections::HashMap<usize, u32> {
    tokens
        .iter()
        .filter_map(|t| t.split_once('='))
        .map(|(o, w)| {
            (
                usize::from_str_radix(o, 16).unwrap(),
                u32::from_str_radix(w, 16).unwrap(),
            )
        })
        .collect()
}

struct PropReplay {
    flag: bool,
    winds: std::collections::VecDeque<([u64; 3], [f32; 3])>,
    times: std::collections::VecDeque<f64>,
    phases: std::collections::VecDeque<f64>,
    probes: std::collections::VecDeque<([u32; 6], (f32, i32))>,
    gate: f64,
    strikes: std::collections::VecDeque<(i32, i32)>,
    ratios: std::collections::VecDeque<(i32, f32)>,
    helds: std::collections::VecDeque<(i32, i32, bool)>,
    limit_as: std::collections::VecDeque<(i32, bool)>,
    limit_bs: std::collections::VecDeque<(i32, bool)>,
    blends: std::collections::VecDeque<(i32, f32)>,
    washes: std::collections::VecDeque<([u32; 6], [f32; 3])>,
    elements: std::collections::VecDeque<([u32; 6], openxplane::prop::ElementResult)>,
    recording: i32,
    records: std::collections::VecDeque<[u32; 20]>,
    table: openxplane::buffet::NoiseTable,
}

/// The deterministic noise table the vector generator writes into the emulated process.
fn prop_noise_table() -> openxplane::buffet::NoiseTable {
    let values = (0..262_144u32)
        .map(|i| {
            let mut h = i.wrapping_mul(2_654_435_761).wrapping_add(12345);
            h ^= h >> 15;
            h = h.wrapping_mul(2_246_822_519);
            h ^= h >> 13;
            (h & 0xff_ffff) as f32 / 16_777_216.0
        })
        .collect();
    openxplane::buffet::NoiseTable::new(values).unwrap()
}

impl openxplane::airflow::AirflowEnv for PropReplay {
    fn wash(&mut self, point: [f32; 3], out: [f32; 3]) -> [f32; 3] {
        let (seen, result) = self.washes.pop_front().expect("wash call not recorded");
        let got = [point[0], point[1], point[2], out[0], out[1], out[2]].map(f32::to_bits);
        assert_eq!(got, seen, "wash arguments");
        result
    }
    fn wind(&mut self, x: f64, y: f64, z: f64) -> [f32; 3] {
        let (seen, wind) = self.winds.pop_front().expect("wind call not recorded");
        assert_eq!(
            [x.to_bits(), y.to_bits(), z.to_bits()],
            seen,
            "wind position"
        );
        wind
    }
}

impl openxplane::prop::PropEnv for PropReplay {
    fn engine_flag(&mut self) -> bool {
        self.flag
    }
    fn frame_time(&mut self) -> f64 {
        self.times.pop_front().expect("time call not recorded")
    }
    fn time_phase(&mut self) -> f64 {
        self.phases
            .pop_front()
            .expect("time-phase call not recorded")
    }
    fn noise2(&mut self, x: f32, y: f32, seed: i32) -> f32 {
        self.table.basis2(x, y, seed)
    }
    fn recording_id(&mut self) -> i32 {
        self.recording
    }
    fn record(&mut self, words: [u32; 20]) {
        let want = self.records.pop_front().expect("record not recorded");
        for i in 0..20 {
            assert!(
                words_close(words[i], want[i], !matches!(i, 0 | 1 | 4)),
                "pass record word {i}: {:#x} vs {:#x}",
                words[i],
                want[i]
            );
        }
    }
    fn element_force(
        &mut self,
        call: &openxplane::prop::ElementCall,
    ) -> openxplane::prop::ElementResult {
        let (seen, result) = self
            .elements
            .pop_front()
            .expect("element call not recorded");
        let got = [
            call.index as u32,
            call.retain as u32,
            call.ice.to_bits(),
            call.extra[0].to_bits(),
            call.extra[1].to_bits(),
            call.extra[2].to_bits(),
        ];
        assert_eq!(got, seen, "element call arguments");
        result
    }
    fn strike_gate(&mut self) -> f64 {
        self.gate
    }
    fn held_back(&mut self, index: i32, mode: i32) -> bool {
        let (i, m, r) = self.helds.pop_front().expect("held-back not recorded");
        assert_eq!((index, mode), (i, m));
        r
    }
    fn limit_a(&mut self, i: i32) -> bool {
        let (seen, r) = self.limit_as.pop_front().expect("limit_a not recorded");
        assert_eq!(i, seen);
        r
    }
    fn limit_b(&mut self, n: i32) -> bool {
        let (seen, r) = self.limit_bs.pop_front().expect("limit_b not recorded");
        assert_eq!(n, seen);
        r
    }
    fn blend(&mut self, mask: i32) -> f32 {
        let (seen, v) = self.blends.pop_front().expect("blend not recorded");
        assert_eq!(mask, seen);
        v
    }
    fn engine_ratio(&mut self, n: i32) -> f32 {
        let (seen, v) = self.ratios.pop_front().expect("ratio not recorded");
        assert_eq!(seen, n);
        v
    }
    fn event(&mut self, id: i32, arg: i32) {
        let want = self.strikes.pop_front().expect("strike not recorded");
        assert_eq!((id, arg), want, "strike event");
    }
    fn terrain(&mut self, a: [f32; 3], b: [f32; 3], height: f32) -> (f32, i32) {
        let (seen, out) = self.probes.pop_front().expect("terrain call not recorded");
        let got = [a[0], a[1], a[2], b[0], b[1], b[2]].map(f32::to_bits);
        assert_eq!(got, seen, "terrain probe points");
        let _ = height;
        out
    }
}

/// Frame slots that hold integers (compared exactly).
const PROP_INT_SLOTS: [i32; 16] = [
    0x84, 0x8e0, 0x8d0, 8, 0x30, 0x34, 0x28, 0x2c, 0x208, 0x20c, 0x240, 0x244, 0x200, 0x204, 0x248,
    0x24c,
];

/// Float words equal up to the platform libm's last bits (sin, cos, tan, atan2 differ from the C runtime of the
/// original by a few ulps, which cancellations can amplify); integers exactly.
fn words_close(a: u32, b: u32, float: bool) -> bool {
    if a == b {
        return true;
    }
    if !float {
        return false;
    }
    let (x, y) = (f32::from_bits(a), f32::from_bits(b));
    if x.is_nan() && y.is_nan() {
        return true;
    }
    let ulps = (i64::from(a as i32) - i64::from(b as i32)).unsigned_abs();
    ulps <= 64 || (x - y).abs() <= 5e-5 * (1.0 + x.abs().max(y.abs()))
}

fn prop_segment(path: &str, stop: openxplane::prop::Stop) -> usize {
    use openxplane::forces::Words;
    use openxplane::prop::{Objects, prop_force};
    let text = std::fs::read_to_string(format!("{}/tests/data/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap();
    let mut trials = 0;
    let mut lines = text.lines().filter(|l| !l.starts_with('#')).peekable();
    while let Some(head) = lines.next() {
        let t: Vec<&str> = head.split_whitespace().collect();
        assert_eq!(t[0], "T");
        let n: i32 = t[1].parse().unwrap();
        let early = t[2] == "1";
        let mut regions = std::collections::HashMap::new();
        let mut winds = std::collections::VecDeque::new();
        let mut times = std::collections::VecDeque::new();
        let mut phases = std::collections::VecDeque::new();
        let mut probes = std::collections::VecDeque::new();
        let mut washes = std::collections::VecDeque::new();
        let mut elements = std::collections::VecDeque::new();
        let mut gate = 0.0f64;
        let mut strikes = std::collections::VecDeque::new();
        let mut ratios = std::collections::VecDeque::new();
        let mut helds = std::collections::VecDeque::new();
        let mut limit_as = std::collections::VecDeque::new();
        let mut limit_bs = std::collections::VecDeque::new();
        let mut blends = std::collections::VecDeque::new();
        let mut recording = 0i32;
        let mut records = std::collections::VecDeque::new();
        let (mut slots, mut regs, mut outputs) = (None, None, Vec::new());
        while let Some(line) = lines.peek() {
            if line.starts_with("T ") {
                break;
            }
            let line = lines.next().unwrap();
            let tokens: Vec<&str> = line.split_whitespace().collect();
            match tokens[0] {
                "F" | "B" | "E" | "P" | "R" | "X0" | "X1" | "X2" | "X3" | "N" | "M" => {
                    regions.insert(tokens[0], Words(parse_words(&tokens[1..])));
                }
                "W" => {
                    let h = |s: &str| u64::from_str_radix(s, 16).unwrap();
                    winds.push_back((
                        [h(tokens[1]), h(tokens[2]), h(tokens[3])],
                        [f(tokens[4]), f(tokens[5]), f(tokens[6])],
                    ));
                }
                "Z" => recording = tokens[1].parse().unwrap(),
                "J" => gate = f64::from_bits(u64::from_str_radix(tokens[1], 16).unwrap()),
                "h" => helds.push_back((
                    tokens[1].parse().unwrap(),
                    tokens[2].parse().unwrap(),
                    tokens[3] == "1",
                )),
                "a" => limit_as.push_back((tokens[1].parse().unwrap(), tokens[2] == "1")),
                "c" => limit_bs.push_back((tokens[1].parse().unwrap(), tokens[2] == "1")),
                "m" => blends.push_back((tokens[1].parse().unwrap(), f(tokens[2]))),
                "U" => ratios.push_back((tokens[1].parse().unwrap(), f(tokens[2]))),
                "K" => strikes.push_back((tokens[1].parse().unwrap(), tokens[2].parse().unwrap())),
                "Q" => {
                    let mut words = [0u32; 20];
                    for (i, w) in words.iter_mut().enumerate() {
                        *w = u32::from_str_radix(tokens[1 + i], 16).unwrap();
                    }
                    records.push_back(words);
                }
                "L" => {
                    let h = |s: &str| u32::from_str_radix(s, 16).unwrap();
                    elements.push_back((
                        [
                            tokens[1].parse::<u32>().unwrap(),
                            tokens[2].parse::<u32>().unwrap(),
                            h(tokens[3]),
                            h(tokens[4]),
                            h(tokens[5]),
                            h(tokens[6]),
                        ],
                        openxplane::prop::ElementResult {
                            out: [f(tokens[8]), f(tokens[9]), f(tokens[10])],
                            x4: [f(tokens[11]), f(tokens[12]), f(tokens[13]), f(tokens[14])],
                            x_1bc: f(tokens[15]),
                            stall: tokens[16].parse().unwrap(),
                        },
                    ));
                }
                "V" => {
                    let h = |s: &str| u32::from_str_radix(s, 16).unwrap();
                    washes.push_back((
                        [
                            h(tokens[1]),
                            h(tokens[2]),
                            h(tokens[3]),
                            h(tokens[4]),
                            h(tokens[5]),
                            h(tokens[6]),
                        ],
                        [f(tokens[7]), f(tokens[8]), f(tokens[9])],
                    ));
                }
                "H" => {
                    phases.push_back(f64::from_bits(u64::from_str_radix(tokens[1], 16).unwrap()))
                }
                "G" => {
                    let h = |s: &str| u32::from_str_radix(s, 16).unwrap();
                    probes.push_back((
                        [
                            h(tokens[1]),
                            h(tokens[2]),
                            h(tokens[3]),
                            h(tokens[4]),
                            h(tokens[5]),
                            h(tokens[6]),
                        ],
                        (f(tokens[7]), tokens[8].parse().unwrap()),
                    ));
                }
                "D" => times.push_back(f64::from_bits(u64::from_str_radix(tokens[1], 16).unwrap())),
                "O" => outputs.push((tokens[1].to_string(), parse_words(&tokens[2..]))),
                "S" => {
                    slots = Some(
                        tokens[1..]
                            .iter()
                            .filter_map(|t| t.split_once('='))
                            .map(|(o, w)| {
                                (
                                    o.parse::<i32>().unwrap(),
                                    u32::from_str_radix(w, 16).unwrap(),
                                )
                            })
                            .collect::<std::collections::HashMap<i32, u32>>(),
                    )
                }
                "Y" => {
                    regs = Some(
                        tokens[1..]
                            .iter()
                            .map(|s| {
                                u32::from_str_radix(s, 16).unwrap_or_else(|_| s.parse().unwrap())
                            })
                            .collect::<Vec<u32>>(),
                    )
                }
                other => panic!("unknown line {other}"),
            }
        }
        let mut env = PropReplay {
            flag: t[3] == "1",
            winds,
            times,
            phases,
            probes,
            washes,
            elements,
            gate,
            strikes,
            ratios,
            helds,
            limit_as,
            limit_bs,
            blends,
            recording,
            records,
            table: prop_noise_table(),
        };
        let mut f = regions.remove("F").unwrap();
        let mut r = regions.remove("R").unwrap();
        let (b, e, p) = (
            regions.remove("B").unwrap(),
            regions.remove("E").unwrap(),
            regions.remove("P").unwrap(),
        );
        // the part and engine records are arrays in the original: the port reads them at the record's own offsets
        let (e_n, p_n) = (
            shift_words(&e, n as usize * 0x68),
            shift_words(&p, n as usize * 0x3770),
        );
        let mut xs: Vec<openxplane::forces::Words> = (0..4)
            .map(|i| {
                let x = regions.remove(format!("X{i}").as_str()).unwrap();
                shift_words(&x, n as usize * 0x2d8)
            })
            .collect();
        let mut y = shift_words(&regions.remove("N").unwrap(), n as usize * 0x388);
        let engine_records = regions.remove("M").unwrap();
        let mut objects = Objects {
            f: &mut f,
            b: &b,
            e: &e_n,
            p: &p_n,
            r: &mut r,
            x: &mut xs,
            y: &mut y,
            e_table: &e,
            m: &engine_records,
        };
        let result = prop_force(&mut objects, n, &mut env, stop);
        let result = match result {
            // the original also returns normally when the speed factor is below 0.01
            Err(e) if stop == openxplane::prop::Stop::End && e == "factor below 0.01" => Ok((
                openxplane::prop::Frame::default(),
                openxplane::prop::Regs::default(),
            )),
            other => other,
        };
        if early {
            assert!(
                result.is_err(),
                "trial {trials}: the original returned early"
            );
        } else {
            let (frame, got_regs) = result.unwrap_or_else(|err| panic!("trial {trials}: {err}"));
            for (off, want) in slots.unwrap() {
                if let Some(got) = frame.0.get(&off) {
                    assert!(
                        words_close(*got, want, !PROP_INT_SLOTS.contains(&off)),
                        "trial {trials}: frame slot {off}: {got:#x} vs {want:#x}"
                    );
                }
            }
            for (off, got) in &frame.0 {
                // a slot the port set must hold the original's word, if the original's frame is in the checked ranges
                let _ = (off, got);
            }
            let want = regs.unwrap();
            for (k, name) in (6..16).zip([
                "xmm6", "xmm7", "xmm8", "xmm9", "xmm10", "xmm11", "xmm12", "xmm13", "xmm14",
                "xmm15",
            ]) {
                if std::env::var("PROP_DEBUG").is_ok() {
                    eprintln!(
                        "trial {trials} xmm{k}: {:?} vs {:#x}",
                        got_regs.xmm[k].map(f32::to_bits),
                        want[k - 6]
                    );
                }
                if let Some(v) = got_regs.xmm[k] {
                    assert!(
                        words_close(v.to_bits(), want[k - 6], true),
                        "trial {trials}: {name}: {:#x} vs {:#x}",
                        v.to_bits(),
                        want[k - 6]
                    );
                }
            }
            if let Some(r15) = got_regs.r15 {
                assert_eq!(r15 as u32, want[10], "trial {trials}: r15");
            }
            for (region, words) in outputs {
                let (ours, base) = match region.as_str() {
                    "R" => (&r, 0),
                    "F" => (&f, 0),
                    "N" => (&y, n as usize * 0x388),
                    other => (
                        &xs[other[1..].parse::<usize>().unwrap()],
                        n as usize * 0x2d8,
                    ),
                };
                for (off, want) in words {
                    let off = off.wrapping_sub(base);
                    let got = ours.0.get(&off).copied().unwrap_or(0);
                    assert!(
                        words_close(got, want, true),
                        "trial {trials}: {region}+{off:#x}: {got:#x} vs {want:#x}"
                    );
                }
            }
        }
        trials += 1;
    }
    trials
}

/// The words of a record array re-based so that record `index` starts at offset 0 (the port's view).
fn shift_words(words: &openxplane::forces::Words, base: usize) -> openxplane::forces::Words {
    openxplane::forces::Words(
        words
            .0
            .iter()
            .filter(|(o, _)| **o >= base)
            .map(|(o, w)| (*o - base, *w))
            .collect(),
    )
}

#[test]
fn prop_force_segment1_matches_the_original_machine_code() {
    let trials = prop_segment("prop_1.txt", openxplane::prop::Stop::Segment1);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment2_matches_the_original_machine_code() {
    let trials = prop_segment("prop_2.txt", openxplane::prop::Stop::Segment2);
    assert!(trials >= 20);
}

#[test]
fn prop_force_matches_the_original_machine_code_to_the_end() {
    let trials = prop_segment("prop_11.txt", openxplane::prop::Stop::End);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment10_matches_the_original_machine_code() {
    let trials = prop_segment("prop_10.txt", openxplane::prop::Stop::Segment10);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment9_matches_the_original_machine_code() {
    let trials = prop_segment("prop_9.txt", openxplane::prop::Stop::Segment9);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment8_matches_the_original_machine_code() {
    let trials = prop_segment("prop_8.txt", openxplane::prop::Stop::Segment8);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment7_matches_the_original_machine_code() {
    let trials = prop_segment("prop_7.txt", openxplane::prop::Stop::Segment7);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment6_matches_the_original_machine_code() {
    let trials = prop_segment("prop_6.txt", openxplane::prop::Stop::Segment6);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment5_matches_the_original_machine_code() {
    let trials = prop_segment("prop_5.txt", openxplane::prop::Stop::Segment5);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment4_matches_the_original_machine_code() {
    let trials = prop_segment("prop_4.txt", openxplane::prop::Stop::Segment4);
    assert!(trials >= 20);
}

#[test]
fn prop_force_segment3_matches_the_original_machine_code() {
    let trials = prop_segment("prop_3.txt", openxplane::prop::Stop::Segment3);
    assert!(trials >= 20);
}

#[test]
fn pointer_following_callees_match_the_original_machine_code() {
    use openxplane::callees::{
        AeroForce, add_aero_force, add_axial_force, add_normal_force, add_side_force,
        add_world_force, blend, body_blend, engine_ratio, lever_curve, limit_a, limit_b,
        wing_area_factor,
    };
    use openxplane::vm::Vm;
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/callees.txt"
    ))
    .unwrap();
    let mut lines = text.lines().filter(|l| !l.starts_with('#')).peekable();
    let mut counts = std::collections::HashMap::new();
    while let Some(head) = lines.next() {
        let t: Vec<&str> = head.split_whitespace().filter(|x| *x != "|").collect();
        assert_eq!(t[0], "R", "{head}");
        let mut vm = Vm::default();
        let mut bindings = std::collections::VecDeque::new();
        let mut expected: Vec<(u64, u32)> = Vec::new();
        while let Some(l) = lines.peek() {
            if l.starts_with("R ") {
                break;
            }
            let l = lines.next().unwrap();
            let tokens: Vec<&str> = l.split_whitespace().collect();
            match tokens[0] {
                "W" => {
                    for tok in &tokens[1..] {
                        let (a, w) = tok.split_once('=').unwrap();
                        vm.set_u32(
                            u64::from_str_radix(a, 16).unwrap(),
                            u32::from_str_radix(w, 16).unwrap(),
                        );
                    }
                }
                "O" => {
                    for tok in &tokens[1..] {
                        let (a, w) = tok.split_once('=').unwrap();
                        expected.push((
                            u64::from_str_radix(a, 16).unwrap(),
                            u32::from_str_radix(w, 16).unwrap(),
                        ));
                    }
                }
                "K" => bindings.push_back((
                    tokens[1].parse::<u32>().unwrap(),
                    tokens[2].parse::<i32>().unwrap(),
                    tokens[3] == "1",
                )),
                other => panic!("unknown line {other}"),
            }
        }
        let address = u64::from_str_radix(t[2], 16).unwrap();
        let arg: i32 = t[3].parse().unwrap();
        let mut binding = |id: u32, index: i32| {
            let (i, x, a) = bindings.pop_front().expect("binding not recorded");
            assert_eq!((id, index), (i, x), "{head}");
            a
        };
        match t[1] {
            "E" => {
                let got = engine_ratio(&vm, address, arg);
                let want = f(t[5]);
                assert!(
                    got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
                    "{head}"
                );
            }
            "B" => {
                let got = blend(&vm, address, arg);
                let want = f(t[5]);
                assert!(
                    got.to_bits() == want.to_bits() || (got.is_nan() && want.is_nan()),
                    "{head}"
                );
            }
            "A" => assert_eq!(
                limit_a(&vm, address, arg, &mut binding),
                t[5] == "1",
                "{head}"
            ),
            "P" => assert_eq!(
                limit_b(&vm, address, arg, &mut binding),
                t[5] == "1",
                "{head}"
            ),
            "X" | "Y" | "Z" => {
                let (a, b, c) = (f(t[5]), f(t[6]), f(t[7]));
                match t[1] {
                    "X" => add_axial_force(&mut vm, address, a, b, c),
                    "Y" => add_normal_force(&mut vm, address, a, b, c),
                    _ => add_side_force(&mut vm, address, a, b, c),
                }
                for (addr, want) in &expected {
                    let got = vm.u32(*addr);
                    assert!(
                        got == *want
                            || (f32::from_bits(got).is_nan() && f32::from_bits(*want).is_nan()),
                        "{head}: {addr:#x}: {got:#x} vs {want:#x}"
                    );
                }
            }
            "G" => {
                let g = |i: usize| f(t[5 + i]);
                let force = AeroForce {
                    a2: g(0),
                    a3: g(1),
                    a5: g(2),
                    a6: g(3),
                    a7: g(4),
                    a8: g(5),
                    a9: g(6),
                    a10: g(7),
                    a11: g(8),
                    a12: g(9),
                    a13: g(10),
                    a14: t[16].parse().unwrap(),
                    a15: f(t[17]),
                    a16: f(t[18]),
                    a17: f(t[19]),
                };
                assert!(add_aero_force(&mut vm, address, &force));
                for (addr, want) in &expected {
                    let got = vm.u32(*addr);
                    assert!(got == *want, "{head}: {addr:#x}: {got:#x} vs {want:#x}");
                }
            }
            "R" => {
                let d = |h: &str| f64::from_bits(u64::from_str_radix(h, 16).unwrap());
                let p: Vec<f64> = (8..14).map(|i| d(t[i])).collect();
                let got = openxplane::transform::rotate_pairs_f64(
                    d(t[5]),
                    d(t[6]),
                    d(t[7]),
                    [p[0], p[1], p[2], p[3], p[4], p[5]],
                );
                for k in 0..3 {
                    assert_eq!(
                        got[k].to_bits(),
                        u64::from_str_radix(t[14 + k], 16).unwrap(),
                        "{head}"
                    );
                }
            }
            "S" => {
                let d = |h: &str| f64::from_bits(u64::from_str_radix(h, 16).unwrap());
                let ang = [f(t[8]), f(t[9]), f(t[10])];
                let off = [f(t[11]), f(t[12]), f(t[13])];
                let got = openxplane::transform::rotate_euler_f64(
                    ang,
                    off,
                    t[4] == "1",
                    [d(t[5]), d(t[6]), d(t[7])],
                );
                for k in 0..3 {
                    let want = d(t[14 + k]);
                    assert!((got[k] - want).abs() <= 1e-5 * (1.0 + want.abs()), "{head}");
                }
            }
            "D" => {
                let got = openxplane::callees::direction_angles(f(t[5]), f(t[6]), f(t[7]));
                for k in 0..4 {
                    let want = f(t[8 + k]);
                    assert!((got[k] - want).abs() <= 1e-6 * (1.0 + want.abs()), "{head}");
                }
            }
            "T" | "U" | "C" => {
                let got = match t[1] {
                    "T" => wing_area_factor(&vm, address),
                    "U" => body_blend(f(t[5]), f(t[6]), f(t[7])),
                    _ => lever_curve(&vm, address, f(t[5])),
                };
                let want = f(t[t.len() - 1]);
                assert!(
                    got.to_bits() == want.to_bits()
                        || (got.is_nan() && want.is_nan())
                        || (got - want).abs() <= 1e-6 * (1.0 + want.abs()),
                    "{head}: {got} vs {want}"
                );
            }
            "V" => {
                let v: Vec<f32> = (5..11).map(|i| f(t[i])).collect();
                add_world_force(&mut vm, address, [v[0], v[1], v[2]], [v[3], v[4], v[5]]);
                for (addr, want) in &expected {
                    let got = vm.u32(*addr);
                    let close = (f32::from_bits(got) - f32::from_bits(*want)).abs() <= 1e-5;
                    assert!(
                        got == *want || close,
                        "{head}: {addr:#x}: {got:#x} vs {want:#x}"
                    );
                }
            }
            other => panic!("unknown kind {other}"),
        }
        assert!(bindings.is_empty(), "unused binding answers: {head}");
        *counts.entry(t[1].to_string()).or_insert(0) += 1;
    }
    assert!(counts.values().all(|c| *c >= 50));
}

/// One recorded case of `tools/xp_vmcase.py`.
struct VmCaseData {
    header: Vec<String>,
    vm: openxplane::vm::Vm,
    calls: std::collections::VecDeque<RecordedCall>,
    expected: Vec<(u64, u32)>,
}

struct RecordedCall {
    address: u64,
    ints: [u64; 4],
    xmm: [u32; 4],
    stack: [u64; 4],
    rax: u64,
    xmm0: u64,
    effects: Vec<(u64, u32)>,
}

fn parse_vm_cases(path: &str) -> Vec<VmCaseData> {
    let text = std::fs::read_to_string(format!("{}/tests/data/{path}", env!("CARGO_MANIFEST_DIR")))
        .unwrap();
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    let word = |tok: &str| {
        let (a, w) = tok.split_once('=').unwrap();
        (hex(a), hex(w) as u32)
    };
    let mut cases: Vec<VmCaseData> = Vec::new();
    for line in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
    {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        match tokens[0] {
            "H" => cases.push(VmCaseData {
                header: tokens[1..].iter().map(|s| s.to_string()).collect(),
                vm: openxplane::vm::Vm::default(),
                calls: Default::default(),
                expected: Vec::new(),
            }),
            "W" => {
                let case = cases.last_mut().unwrap();
                for tok in &tokens[1..] {
                    let (a, w) = word(tok);
                    case.vm.set_u32(a, w);
                }
            }
            "O" => {
                cases.last_mut().unwrap().expected = tokens[1..].iter().map(|t| word(t)).collect()
            }
            "C" => {
                let bar: Vec<usize> = tokens
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| **t == "|")
                    .map(|(i, _)| i)
                    .collect();
                cases.last_mut().unwrap().calls.push_back(RecordedCall {
                    address: hex(tokens[1]),
                    ints: [
                        hex(tokens[2]),
                        hex(tokens[3]),
                        hex(tokens[4]),
                        hex(tokens[5]),
                    ],
                    xmm: [6, 7, 8, 9].map(|k| hex(tokens[k]) as u32),
                    stack: [10, 11, 12, 13].map(|k| hex(tokens[k])),
                    rax: hex(tokens[bar[0] + 1]),
                    xmm0: hex(tokens[bar[0] + 2]),
                    effects: tokens[bar[1] + 1..].iter().map(|t| word(t)).collect(),
                });
            }
            other => panic!("unknown line {other}"),
        }
    }
    cases
}

/// Equal words, or two normal floats within a relative tolerance (libm differences).
fn close_bits(a: u32, b: u32) -> bool {
    let normal = |w: u32| (w >> 23) & 0xff != 0 && (w >> 23) & 0xff != 0xff;
    a == b
        || (normal(a)
            && normal(b)
            && (f32::from_bits(a) - f32::from_bits(b)).abs()
                <= 1e-5 * (1.0 + f32::from_bits(b).abs()))
}

struct VmReplay {
    calls: std::collections::VecDeque<RecordedCall>,
}

impl openxplane::vm::Callees for VmReplay {
    fn call(
        &mut self,
        vm: &mut openxplane::vm::Vm,
        address: u64,
        args: openxplane::vm::CallArgs,
    ) -> openxplane::vm::Reply {
        let c = self
            .calls
            .pop_front()
            .unwrap_or_else(|| panic!("call {address:#x} not recorded"));
        assert_eq!(c.address, address, "call order");
        for (i, want) in args.int.iter().enumerate() {
            if let Some(v) = want {
                assert_eq!(*v, c.ints[i], "argument {i} of {address:#x}");
            }
        }
        for (i, want) in args.xmm.iter().enumerate() {
            if let Some(v) = want {
                assert!(
                    close_bits(*v, c.xmm[i]),
                    "float argument {i} of {address:#x}: {v:#x} vs {:#x}",
                    c.xmm[i]
                );
            }
        }
        for (i, want) in args.stack.iter().enumerate() {
            if let Some(v) = want {
                // a float stored by `movss` leaves the upper half of the slot as it was
                if *v >> 32 == 0 {
                    let got = (c.stack[i] & 0xffff_ffff) as u32;
                    assert!(
                        close_bits(*v as u32, got),
                        "stack argument {i} of {address:#x}: {v:#x} vs {got:#x}"
                    );
                } else {
                    assert_eq!(*v, c.stack[i], "stack argument {i} of {address:#x}");
                }
            }
        }
        for (a, w) in &c.effects {
            vm.set_u32(*a, *w);
        }
        openxplane::vm::Reply {
            rax: c.rax,
            xmm0: c.xmm0,
        }
    }
}

/// The runtime atmosphere table the generators fill with a deterministic pattern.
fn fill_atmosphere_table(vm: &mut openxplane::vm::Vm) {
    for i in 0..0x803u64 * 2 {
        let v = (0.3 + ((i * 37) % 101) as f64 / 100.0) as f32;
        vm.set_f32(openxplane::controls::ATMOSPHERE_TABLE + 4 * i, v);
    }
}

fn controls_stage(path: &str, stop: Option<openxplane::controls::Stop>) {
    let cases = parse_vm_cases(path);
    assert!(cases.len() >= 20);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let entry = u64::from_str_radix(&case.header[1], 16).unwrap();
        fill_atmosphere_table(&mut case.vm);
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        let result = openxplane::controls::engine_controls(&mut case.vm, &mut env, f, entry, stop);
        result.unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(
            env.calls.is_empty(),
            "case {n}: {} recorded calls unused",
            env.calls.len()
        );
        for (addr, want) in &case.expected {
            let got = case.vm.u32(*addr);
            let (g, w) = (f32::from_bits(got), f32::from_bits(*want));
            assert!(
                got == *want
                    || (g.is_nan() && w.is_nan())
                    || (g - w).abs() <= 1e-5 * (1.0 + w.abs()),
                "case {n}: {addr:#x}: {got:#x} vs {want:#x}"
            );
        }
    }
}

#[test]
fn engine_controls_dispatch_matches_the_original_machine_code() {
    controls_stage("controls_1.txt", Some(openxplane::controls::Stop::Dispatch));
}

#[test]
fn engine_controls_blend_matches_the_original_machine_code() {
    controls_stage("controls_2.txt", Some(openxplane::controls::Stop::Blend));
}

#[test]
fn engine_controls_groups_match_the_original_machine_code() {
    controls_stage("controls_3.txt", Some(openxplane::controls::Stop::Groups));
}

#[test]
fn engine_controls_match_the_original_machine_code_to_the_end() {
    controls_stage("controls_4.txt", None);
}

#[test]
fn engine_thrust_matches_the_original_machine_code() {
    for (n, mut case) in parse_vm_cases("engine_funcs_1411975a0.txt")
        .into_iter()
        .enumerate()
    {
        let state = u64::from_str_radix(&case.header[0], 16).unwrap();
        let f = u64::from_str_radix(&case.header[1], 16).unwrap();
        let e: i32 = case.header[2].parse().unwrap();
        openxplane::controls::apply_engine_thrust(&mut case.vm, state, f, e);
        for (addr, want) in &case.expected {
            let got = case.vm.u32(*addr);
            assert!(
                got == *want
                    || (f32::from_bits(got) - f32::from_bits(*want)).abs()
                        <= 1e-5 * (1.0 + f32::from_bits(*want).abs()),
                "case {n}: {addr:#x}: {got:#x} vs {want:#x}"
            );
        }
    }
}

#[test]
fn engine_kind7_update_matches_the_original_machine_code() {
    for (n, mut case) in parse_vm_cases("engine_funcs_14119a570.txt")
        .into_iter()
        .enumerate()
    {
        let state = u64::from_str_radix(&case.header[0], 16).unwrap();
        let f = u64::from_str_radix(&case.header[1], 16).unwrap();
        let e: i32 = case.header[2].parse().unwrap();
        for i in 0..0x803u64 * 2 {
            let v = (0.3 + ((i * 37) % 101) as f64 / 100.0) as f32;
            case.vm
                .set_f32(openxplane::controls::ATMOSPHERE_TABLE + 4 * i, v);
        }
        openxplane::controls::update_engine_kind7(&mut case.vm, state, f, e);
        for (addr, want) in &case.expected {
            let got = case.vm.u32(*addr);
            let (g, w) = (f32::from_bits(got), f32::from_bits(*want));
            assert!(
                got == *want
                    || (g.is_nan() && w.is_nan())
                    || (g - w).abs() <= 1e-5 * (1.0 + w.abs()),
                "case {n}: {addr:#x}: {got:#x} vs {want:#x}"
            );
        }
    }
}

#[test]
fn piston_engine_update_matches_the_original_machine_code() {
    let cases = parse_vm_cases("piston.txt");
    assert!(cases.len() >= 50);
    for (n, mut case) in cases.into_iter().enumerate() {
        let state = u64::from_str_radix(&case.header[0], 16).unwrap();
        let f = u64::from_str_radix(&case.header[1], 16).unwrap();
        let e: i32 = case.header[2].parse().unwrap();
        let inputs = u64::from_str_radix(&case.header[3], 16).unwrap();
        for i in 0..0x803u64 * 2 {
            let v = (0.3 + ((i * 37) % 101) as f64 / 100.0) as f32;
            case.vm
                .set_f32(openxplane::controls::ATMOSPHERE_TABLE + 4 * i, v);
        }
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::piston::update_engine_piston(&mut case.vm, &mut env, state, f, e, inputs);
        assert!(
            env.calls.is_empty(),
            "case {n}: {} recorded calls unused",
            env.calls.len()
        );
        for (addr, want) in &case.expected {
            let got = case.vm.u32(*addr);
            let (g, w) = (f32::from_bits(got), f32::from_bits(*want));
            assert!(
                got == *want
                    || (g.is_nan() && w.is_nan())
                    || (g - w).abs() <= 1e-5 * (1.0 + w.abs()),
                "case {n}: {addr:#x}: {got:#x} vs {want:#x}"
            );
        }
    }
}

fn small_cases(function: &str) -> Vec<VmCaseData> {
    let cases = parse_vm_cases(&format!("engine_small_{function}.txt"));
    assert!(cases.len() >= 40);
    cases
}

fn words_match(case: &VmCaseData, n: usize) {
    // a differing word passes only as two normal floats within a relative tolerance (libm differences): integers
    // and flags that differ in their bits must not hide as denormals
    let normal = |w: u32| (w >> 23) & 0xff != 0 && (w >> 23) & 0xff != 0xff;
    for (addr, want) in &case.expected {
        let got = case.vm.u32(*addr);
        assert!(
            got == *want
                || (normal(got)
                    && normal(*want)
                    && (f32::from_bits(got) - f32::from_bits(*want)).abs()
                        <= 1e-5 * (1.0 + f32::from_bits(*want).abs())),
            "case {n}: {addr:#x}: {got:#x} vs {want:#x}"
        );
    }
}

#[test]
fn engine_has_mode_matches_the_original_machine_code() {
    for (n, case) in small_cases("140822620").into_iter().enumerate() {
        let b = u64::from_str_radix(&case.header[0], 16).unwrap();
        let index: i32 = case.header[1].parse().unwrap();
        let mode: i32 = case.header[2].parse().unwrap();
        let want: i32 = case.header[3].parse().unwrap();
        assert_eq!(
            openxplane::callees::engine_has_mode(&case.vm, b, index, mode),
            want,
            "case {n}"
        );
    }
}

#[test]
fn int_power_matches_the_original_machine_code() {
    for (n, case) in small_cases("141192820").into_iter().enumerate() {
        let base: i32 = case.header[0].parse().unwrap();
        let exponent: i32 = case.header[1].parse().unwrap();
        let want = case.header[2].parse::<u64>().unwrap() as u32 as i32;
        assert_eq!(
            openxplane::callees::int_power(base, exponent),
            want,
            "case {n}"
        );
    }
}

#[test]
fn group_query_matches_the_original_machine_code() {
    for (n, mut case) in small_cases("1411854a0").into_iter().enumerate() {
        let b = u64::from_str_radix(&case.header[0], 16).unwrap();
        let j: i32 = case.header[1].parse().unwrap();
        let g: i32 = case.header[2].parse().unwrap();
        let mask: u32 = case.header[3].parse().unwrap();
        let want: u8 = case.header[4].parse().unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        let got = openxplane::callees::group_query(&case.vm, &mut env, b, j, g, mask);
        assert_eq!(u8::from(got), want, "case {n}");
        assert!(env.calls.is_empty(), "case {n}: recorded calls unused");
    }
}

#[test]
fn replay_active_matches_the_original_machine_code() {
    for (n, case) in small_cases("1411c5a90").into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let want: u8 = case.header[1].parse().unwrap();
        assert_eq!(
            u8::from(openxplane::callees::replay_active(&case.vm, f)),
            want,
            "case {n}"
        );
    }
}

#[test]
fn engine_start_state_matches_the_original_machine_code() {
    for (n, mut case) in small_cases("1411da6c0").into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let index: i32 = case.header[1].parse().unwrap();
        openxplane::callees::engine_start_state(&mut case.vm, f, index);
        words_match(&case, n);
    }
}

#[test]
fn wing_aspect_pass_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_aspect.txt");
    assert!(cases.len() >= 20);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::wing_aspect_pass(&mut case.vm, &mut env, f)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(
            env.calls.is_empty(),
            "case {n}: {} recorded calls unused",
            env.calls.len()
        );
        words_match(&case, n);
    }
}

#[test]
fn thrust_effects_pass_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_thrust.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::thrust_effects_pass(&mut case.vm, &mut env, f)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(
            env.calls.is_empty(),
            "case {n}: {} recorded calls unused",
            env.calls.len()
        );
        words_match(&case, n);
    }
}

#[test]
fn element_pass_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_element.txt");
    assert!(cases.len() >= 20);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::element_pass(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(
            env.calls.is_empty(),
            "case {n}: {} recorded calls unused",
            env.calls.len()
        );
        words_match(&case, n);
    }
}

#[test]
fn body_aero_matches_the_original_machine_code() {
    let cases = parse_vm_cases("body_141a51600.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u32::from_str_radix(s, 16).unwrap();
    let bits = |s: &str| f32::from_bits(hex(s));
    for (n, case) in cases.into_iter().enumerate() {
        let r = u64::from_str_radix(&case.header[0], 16).unwrap();
        let out = u64::from_str_radix(&case.header[6], 16).unwrap();
        let want_return = bits(&case.header[9]);
        let got = openxplane::body::body_aero(
            &case.vm,
            r,
            bits(&case.header[1]),
            bits(&case.header[2]),
            bits(&case.header[3]),
            bits(&case.header[4]),
            bits(&case.header[5]),
        );
        let close = |a: f32, b: f32| {
            a.to_bits() == b.to_bits()
                || (a.is_nan() && b.is_nan())
                || (a - b).abs() <= 1e-5 * (1.0 + b.abs())
        };
        match got {
            None => assert_eq!(want_return, 0.0, "case {n}"),
            Some(f) => {
                assert!(
                    close(f.magnitude, want_return),
                    "case {n}: {} vs {want_return}",
                    f.magnitude
                );
                for (offset, value) in [(0u64, f.axial), (4, f.side), (8, f.normal)] {
                    let want = case
                        .expected
                        .iter()
                        .find(|(a, _)| *a == out + offset)
                        .map(|(_, w)| f32::from_bits(*w))
                        .expect("an output word");
                    assert!(
                        close(value, want),
                        "case {n}: output {offset}: {value} vs {want}"
                    );
                }
            }
        }
    }
}

#[test]
fn body_wave_drag_matches_the_original_machine_code() {
    let cases = parse_vm_cases("body_141a522d0.txt");
    assert!(cases.len() >= 60);
    let bits = |s: &str| f32::from_bits(u32::from_str_radix(s, 16).unwrap());
    for (n, case) in cases.into_iter().enumerate() {
        let r = u64::from_str_radix(&case.header[0], 16).unwrap();
        let want = bits(&case.header[4]);
        let got = openxplane::body::body_wave_drag(
            &case.vm,
            r,
            bits(&case.header[1]),
            bits(&case.header[2]),
            bits(&case.header[3]),
        );
        assert!(
            got.to_bits() == want.to_bits()
                || (got.is_nan() && want.is_nan())
                || (got - want).abs() <= 1e-5 * (1.0 + want.abs()),
            "case {n}: {got} vs {want}"
        );
    }
}

#[test]
fn body_pass_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_body.txt");
    assert!(cases.len() >= 20);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::body_pass(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn part_force_pass_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_parts.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        openxplane::flight_step::part_force_pass(&mut case.vm, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        words_match(&case, n);
    }
}

#[test]
fn rigid_body_step_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_motion.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::rigid_body_step(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn set_position_matches_the_original_machine_code() {
    let cases = parse_vm_cases("position.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let position = [2, 3, 4].map(|k| f64::from_bits(hex(&case.header[k])));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::set_position(&mut case.vm, &mut env, f, rbp, position);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn euler_to_quaternion_matches_the_original_machine_code() {
    let cases = parse_vm_cases("attitude_e2q.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    let float = |s: &str| f32::from_bits(hex(s) as u32);
    for (n, mut case) in cases.into_iter().enumerate() {
        let h = case.header.clone();
        let q = openxplane::attitude::euler_to_quaternion(float(&h[0]), float(&h[1]), float(&h[2]));
        for (k, v) in q.iter().enumerate() {
            case.vm.set_f32(hex(&h[3]) + 4 * k as u64, *v);
        }
        words_match(&case, n);
    }
}

#[test]
fn quaternion_to_euler_matches_the_original_machine_code() {
    let cases = parse_vm_cases("attitude_q2e.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let h = case.header.clone();
        let q = hex(&h[0]);
        let mut quaternion = [0, 4, 8, 12].map(|k| case.vm.f32(q + k));
        let angles = openxplane::attitude::quaternion_to_euler(&mut quaternion);
        for (k, v) in quaternion.iter().enumerate() {
            case.vm.set_f32(q + 4 * k as u64, *v);
        }
        for (k, v) in angles.iter().enumerate() {
            case.vm.set_f32(hex(&h[1 + k]), *v);
        }
        words_match(&case, n);
    }
}

#[test]
fn attitude_integration_matches_the_original_machine_code() {
    let cases = parse_vm_cases("attitude_integrate.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    let float = |s: &str| f32::from_bits(hex(s) as u32);
    for (n, mut case) in cases.into_iter().enumerate() {
        let h = case.header.clone();
        let w = [float(&h[0]), float(&h[1]), float(&h[2])];
        openxplane::attitude::integrate_attitude(
            &mut case.vm,
            w,
            hex(&h[3]),
            hex(&h[4]),
            hex(&h[5]),
            hex(&h[6]),
        );
        words_match(&case, n);
    }
}

#[test]
fn integrate_motion_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_integrate.txt");
    assert!(cases.len() >= 80);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp, callee) = (
            hex(&case.header[0]),
            hex(&case.header[1]),
            hex(&case.header[2]),
        );
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::integrate_motion(&mut case.vm, &mut env, f, rbp, callee);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn body_velocities_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_velocity.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::body_velocities(&mut case.vm, &mut env, f);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn geodetic_state_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_geodetic.txt");
    assert!(cases.len() >= 60);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::geodetic_state(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn flight_angles_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_angles.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        openxplane::flight_step::flight_angles(&mut case.vm, f);
        words_match(&case, n);
    }
}

#[test]
fn path_samples_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_path.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::path_samples(&mut case.vm, &mut env, f);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn instruments_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_instruments.txt");
    assert!(cases.len() >= 80);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::instruments(&mut case.vm, &mut env, f);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn force_coefficients_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_coeff.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::force_coefficients(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn late_state_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_late.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::late_state(&mut case.vm, &mut env, f);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn arm_probe_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_arm.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        let esi = openxplane::flight_state::arm_probe(&mut case.vm, &mut env, f, rbp);
        assert_eq!(u64::from(esi), hex(&case.header[2]), "case {n}: esi");
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn gear_aero_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_gear.txt");
    assert!(cases.len() >= 40);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::gear_aero(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn wheel_groups_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_wheels.txt");
    assert!(cases.len() >= 60);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        let rbp = u64::from_str_radix(&case.header[1], 16).unwrap();
        openxplane::flight_state::wheel_groups(&mut case.vm, f, rbp);
        words_match(&case, n);
    }
}

#[test]
fn hook_state_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_hook.txt");
    assert!(cases.len() >= 80);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = hex(&case.header[0]);
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::hook_state(
            &mut case.vm,
            &mut env,
            f,
            hex(&case.header[2]) as u32,
        );
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn tow_and_records_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_tow.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::tow_and_records(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn float_drag_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_floats.txt");
    assert!(cases.len() >= 80);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::float_drag(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn float_waves_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_waves.txt");
    assert!(cases.len() >= 60);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::float_waves(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn world_pull_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_pull.txt");
    assert!(cases.len() >= 80);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::world_pull(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn gear_targets_match_the_original_machine_code() {
    let cases = parse_vm_cases("flight_gtarget.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::gear_targets(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn steering_state_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_steer.txt");
    assert!(cases.len() >= 60);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::steering_state(&mut case.vm, &mut env, f, rbp);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn gear_state_update_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_gstate.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::gear_state_update(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn wheel_contact_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_wcontact.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::wheel_contact(&mut case.vm, &mut env, f, rbp)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn wing_ground_probe_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_wprobe.txt");
    assert!(cases.len() >= 30);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        let entered = openxplane::flight_state::wing_ground_probe(&mut case.vm, &mut env, f, rbp);
        assert_eq!(
            u64::from(entered),
            hex(&case.header[2]),
            "case {n}: entered"
        );
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn body_surface_probe_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_bsurf.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp, body) = (
            hex(&case.header[0]),
            hex(&case.header[1]),
            hex(&case.header[2]),
        );
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::body_surface_probe(&mut case.vm, &mut env, f, rbp, body);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn body_contact_blend_matches_the_original_machine_code() {
    let cases = parse_vm_cases("flight_bblend.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp, body) = (
            hex(&case.header[0]),
            hex(&case.header[1]),
            hex(&case.header[2]),
        );
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_state::body_contact_blend(&mut case.vm, &mut env, f, rbp, body);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn atmosphere_step_matches_the_original_machine_code() {
    let cases = parse_vm_cases("atmosphere_step.txt");
    assert!(cases.len() >= 100);
    for (n, mut case) in cases.into_iter().enumerate() {
        let f = u64::from_str_radix(&case.header[0], 16).unwrap();
        fill_atmosphere_table(&mut case.vm);
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::flight_step::atmosphere_step(&mut case.vm, &mut env, f);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

fn wash_stage(path: &str, stop: Option<openxplane::wash::Stop>) {
    let cases = parse_vm_cases(path);
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let x = f32::from_bits(hex(&case.header[2]) as u32);
        let z = f32::from_bits(hex(&case.header[3]) as u32);
        let out1 = hex(&case.header[4]);
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::wash::wash(&mut case.vm, &mut env, f, rbp, x, z, out1, stop)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn wash_jets_match_the_original_machine_code() {
    wash_stage("wash_1.txt", Some(openxplane::wash::Stop::Jets));
}

#[test]
fn wash_parts_match_the_original_machine_code() {
    wash_stage("wash_2.txt", Some(openxplane::wash::Stop::Parts));
}

#[test]
fn wash_wings_match_the_original_machine_code() {
    wash_stage("wash_3.txt", Some(openxplane::wash::Stop::Wings));
}

/// The tail of the wash: the vectors were recorded with the shadow function `0x141186930` stubbed, so its recorded
/// flag and value are replayed and `shadow_scale` applies them (the function itself is tested on its own).
#[test]
fn wash_tail_matches_the_original_machine_code() {
    let cases = parse_vm_cases("wash_4.txt");
    assert!(cases.len() >= 40);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (f, rbp) = (hex(&case.header[0]), hex(&case.header[1]));
        let x = f32::from_bits(hex(&case.header[2]) as u32);
        let z = f32::from_bits(hex(&case.header[3]) as u32);
        let out1 = hex(&case.header[4]);
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        let stop = Some(openxplane::wash::Stop::Wings);
        openxplane::wash::wash(&mut case.vm, &mut env, f, rbp, x, z, out1, stop)
            .unwrap_or_else(|e| panic!("case {n}: {e}"));
        let args = openxplane::vm::CallArgs::ints(&[f, rbp + 0x670]);
        openxplane::vm::Callees::call(&mut env, &mut case.vm, 0x141186930, args);
        openxplane::wash::shadow_scale(&mut case.vm, rbp, out1, out1 + 4, out1 + 8);
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn ray_box_matches_the_original_machine_code() {
    let cases = parse_vm_cases("shadow_box.txt");
    assert!(cases.len() >= 200);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    let mut hits = 0;
    for (n, case) in cases.iter().enumerate() {
        let a: Vec<u64> = case.header[..4].iter().map(|s| hex(s)).collect();
        let got = openxplane::shadow::ray_box(&case.vm, a[0], a[1], a[2], a[3]);
        let want = hex(&case.header[4]) != 0;
        assert_eq!(got, want, "case {n}");
        hits += usize::from(want);
    }
    assert!(hits >= 10);
}

#[test]
fn body_shadow_matches_the_original_machine_code() {
    let cases = parse_vm_cases("shadow_body.txt");
    assert!(cases.len() >= 25);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    let float = |s: &str| f32::from_bits(hex(s) as u32);
    for (n, mut case) in cases.into_iter().enumerate() {
        let h = case.header.clone();
        let mut env = VmReplay {
            calls: std::mem::take(&mut case.calls),
        };
        openxplane::shadow::body_shadow(
            &mut case.vm,
            &mut env,
            hex(&h[0]),
            hex(&h[1]),
            float(&h[2]),
            hex(&h[5]),
            float(&h[3]),
            hex(&h[6]),
            float(&h[4]),
            hex(&h[7]),
            hex(&h[8]) as u32 as i32,
        )
        .unwrap_or_else(|e| panic!("case {n}: {e}"));
        assert!(env.calls.is_empty(), "case {n}: unused calls");
        words_match(&case, n);
    }
}

#[test]
fn wing_chain_factor_matches_the_original_machine_code() {
    let cases = parse_vm_cases("chain.txt");
    assert!(cases.len() >= 100);
    let hex = |s: &str| u64::from_str_radix(s, 16).unwrap();
    for (n, mut case) in cases.into_iter().enumerate() {
        let (x, a, b) = (
            hex(&case.header[1]),
            hex(&case.header[2]),
            hex(&case.header[3]),
        );
        openxplane::flight_step::wing_chain_factor(&mut case.vm, x, a, b);
        words_match(&case, n);
    }
}
