fn f(hex: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(hex, 16).unwrap())
}

#[test]
fn wing_element_delta_path_matches_the_original_machine_code() {
    use xp_airfoil::wing_element::{
        Aircraft, Boundary, DeltaContext, ElementInputs, ElementState, Flow, FoilCall, FoilResult,
        WingFields, delta_wing_weight, evaluate_with_delta,
    };
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/delta_element.txt"
    ))
    .unwrap();
    let noise = xp_airfoil::buffet::NoiseTable::new(
        (0..xp_airfoil::buffet::TABLE_LEN)
            .map(|i| {
                let hash = (i as u32).wrapping_mul(1664525).wrapping_add(1013904223);
                (hash >> 8) as f32 / 16777216.0
            })
            .collect(),
    )
    .unwrap();
    let mut exact_outputs = 0;
    let mut worst_error = 0.0_f32;
    let (mut cases, mut calls_total, mut mismatches) = (0, 0, Vec::<String>::new());
    let mut by_slot = [0usize; 3];
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let mut it = line.split_whitespace().filter(|t| *t != "|");
        let mut next = || it.next().unwrap_or_else(|| panic!("line {n} too short"));
        let time = f64::from_bits(u64::from_str_radix(next(), 16).unwrap());
        let thickness = [Some(f(next())), Some(f(next())), Some(f(next()))];
        let weight = f(next());
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
            skip_delta_block: n % 2 == 0,
        };
        let mut replay = recorded.iter();
        let mut problems = Vec::new();
        assert_eq!(
            delta_wing_weight(&inputs.wing.boundary, e, f18, f1c).to_bits(),
            weight.to_bits(),
            "weight case {n}"
        );
        assert!(weight > 0.0);
        let context = DeltaContext {
            noise: &noise,
            running_time_seconds: time,
            foil_thickness: thickness,
        };
        let got = evaluate_with_delta(&inputs, &context, |c: &FoilCall| {
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
        for (channel, (a, b)) in have.iter().zip(want).enumerate() {
            exact_outputs += usize::from(a.to_bits() == b.to_bits());
            let error = (a - b).abs();
            worst_error = worst_error.max(error);
            let same = if matches!(channel, 1..=3) {
                error <= 2e-6
            } else {
                a.to_bits() == b.to_bits()
            };
            if !same {
                problems.push(format!(
                    "output {channel}: port {a:?} original {b:?} error {error}"
                ));
            }
        }
        if got.stall_flag != want_stall {
            problems.push(format!("stall differs: {} vs {want_stall}", got.stall_flag));
        }
        cases += 1;
        if !problems.is_empty() {
            mismatches.push(format!("case {n}: {}", problems.join("\n")));
        }
    }
    println!(
        "{exact_outputs}/{} output words exact; max absolute error {worst_error}",
        cases * 6
    );
    println!(
        "{cases} cases, {calls_total} profile calls (root {}, middle {}, tip {})",
        by_slot[0], by_slot[1], by_slot[2]
    );
    assert!(
        cases >= 350 && by_slot.iter().all(|c| *c > 20),
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
