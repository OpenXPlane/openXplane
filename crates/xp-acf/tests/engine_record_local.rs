//! The engine record `E` (region `R5ff8`, stride 0x68) that the original's engine and propeller code reads, built
//! from the installed Cessna 172 SP by the ACF loader.
//! Needs `OPENXPLANE_REFERENCE_AIRCRAFT` (path to `Cessna_172SP.acf`); skipped without it.
use xp_acf::{
    Aircraft,
    object::{Schema, load_object},
};

#[test]
fn c172_engine_record_is_filled_from_the_acf() {
    let Ok(path) = std::env::var("OPENXPLANE_REFERENCE_AIRCRAFT") else {
        eprintln!("skipped: set OPENXPLANE_REFERENCE_AIRCRAFT");
        return;
    };
    let acf = Aircraft::parse(&std::fs::read_to_string(path).unwrap()).unwrap();
    let object = load_object(&acf, &Schema::load());
    let f = |region: &str, offset: u32| f32::from_bits(object.word(region, offset));
    // E+4 `_power_max_limit`, E+0x14 `_RSC_max_pwr_ENGN`, E+0x18 `_RSC_redline_ENGN`, E+0x64 `_engn_health_var`
    let e = |offset: u32| f("R5ff8", offset);
    eprintln!(
        "E: +0 type word {}, +4 {}, +0x14 {}, +0x18 {}, +0x64 {}",
        object.word("R5ff8", 0),
        e(4),
        e(0x14),
        e(0x18),
        e(0x64)
    );
    assert_eq!(object.word("R5ff8", 0), 2, "RCP_INJ is kind 2");
    assert!(e(4) > 100.0 && e(4) < 200_000.0, "power {}", e(4));
    assert!((e(0x64) - 1.0).abs() < 1e-6, "health {}", e(0x64));
    // E+0x4c..0x54: `_part_x/y/z` in metres relative to the CG (the original keeps the point here)
    eprintln!("E: part point ({}, {}, {})", e(0x4c), e(0x50), e(0x54));
    assert!(
        e(0x54) < 0.0,
        "the engine sits ahead of the CG, z {}",
        e(0x54)
    );
}
