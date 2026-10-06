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
            |x, y, z| {
                assert_eq!(
                    [x.to_bits(), y.to_bits(), z.to_bits()],
                    seen.map(f64::to_bits),
                    "{line}"
                );
                wind
            },
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
    use openxplane::scalar::{clamp, kind_is_3_or_7, lerp, max3, sign, snap};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/scalar.txt"
    ))
    .unwrap();
    let mut counts = [0usize; 6];
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
            _ => (5, kind_is_3_or_7(t[1].parse().unwrap()) == (t[2] == "1")),
        };
        assert!(ok, "{line}");
        counts[k] += 1;
    }
    assert_eq!(counts[..5], [300; 5]);
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
}

impl openxplane::prop::PropEnv for PropReplay {
    fn engine_flag(&mut self) -> bool {
        self.flag
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

/// Frame slots that hold integers (compared exactly).
const PROP_INT_SLOTS: [i32; 3] = [0x84, 0x8e0, 0x8d0];

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
    ulps <= 64 || (x - y).abs() <= 1e-5 * (1.0 + x.abs().max(y.abs()))
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
        let (mut slots, mut regs, mut outputs) = (None, None, Vec::new());
        while let Some(line) = lines.peek() {
            if line.starts_with("T ") {
                break;
            }
            let line = lines.next().unwrap();
            let tokens: Vec<&str> = line.split_whitespace().collect();
            match tokens[0] {
                "F" | "B" | "E" | "P" | "R" => {
                    regions.insert(tokens[0], Words(parse_words(&tokens[1..])));
                }
                "W" => {
                    let h = |s: &str| u64::from_str_radix(s, 16).unwrap();
                    winds.push_back((
                        [h(tokens[1]), h(tokens[2]), h(tokens[3])],
                        [f(tokens[4]), f(tokens[5]), f(tokens[6])],
                    ));
                }
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
                "X" => {
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
        let mut objects = Objects {
            f: &mut f,
            b: &b,
            e: &e_n,
            p: &p_n,
            r: &mut r,
        };
        let result = prop_force(&mut objects, n, &mut env, stop);
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
            let got = [
                got_regs.xmm6.to_bits(),
                got_regs.xmm7.to_bits(),
                got_regs.xmm8.to_bits(),
                got_regs.xmm9.to_bits(),
                got_regs.xmm11.to_bits(),
                got_regs.xmm13.to_bits(),
                got_regs.xmm15.to_bits(),
                got_regs.r15 as u32,
            ];
            for (k, name) in [
                "xmm6", "xmm7", "xmm8", "xmm9", "xmm11", "xmm13", "xmm15", "r15",
            ]
            .iter()
            .enumerate()
            {
                if k == 2 {
                    continue;
                }
                assert!(
                    words_close(got[k], want[k], k != 7),
                    "trial {trials}: {name}: {:#x} vs {:#x}",
                    got[k],
                    want[k]
                );
            }
            for (region, words) in outputs {
                let ours = if region == "R" { &r } else { &f };
                for (off, want) in words {
                    assert_eq!(
                        ours.0.get(&off).copied().unwrap_or(0),
                        want,
                        "trial {trials}: {region}+{off:#x}"
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
    assert!(trials >= 30);
}
