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
