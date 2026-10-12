fn f(hex: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(hex, 16).unwrap())
}
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

impl xp_airfoil::element_force::Mem for Sparse {
    fn f32(&self, offset: usize) -> f32 {
        f32::from_bits(self.0.get(&offset).copied().unwrap_or(0))
    }
    fn i32(&self, offset: usize) -> i32 {
        self.0.get(&offset).copied().unwrap_or(0) as i32
    }
}

#[test]
fn delta_element_force_matches_the_original_machine_code() {
    use xp_airfoil::element_force::{Call, Objects, element_force_with_delta};
    use xp_airfoil::wing_element::{FoilCall, FoilResult};
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/data/delta_force.txt"
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
    let (mut cases, mut exact, mut worst, mut problems) = (0, 0, 0u32, Vec::<String>::new());
    for (n, line) in text
        .lines()
        .filter(|l| !l.starts_with('#') && !l.is_empty())
        .enumerate()
    {
        let parts: Vec<&str> = line.splitn(6, " | ").collect();
        let prefix: Vec<&str> = parts[0].split_whitespace().collect();
        let time = f64::from_bits(u64::from_str_radix(prefix[0], 16).unwrap());
        let delta_weight = f(prefix[1]);
        assert!(delta_weight > 0.0);
        let head = &prefix[2..];
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
        let got = element_force_with_delta(
            &objects,
            &call,
            &noise,
            time,
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
                    && c.diagnostics == want_call.diagnostics
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
        if replay.next().is_some() {
            bad.push("fewer profile calls than original".into());
        }
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
        if off > 64 && std::env::var_os("ELEMENT_FORCE_DEBUG").is_some() {
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
