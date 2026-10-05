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
