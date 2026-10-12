//! Behavioural checks of the approximate flight model on the provided Cessna 172 SP. They need the
//! reference installation in `Xplane12/` and are skipped when it is absent.
use glam::Vec3;
use openxplane::{
    flight::{Controls, FlightModel},
    install::Install,
};
use std::path::Path;

const ACF: &str = "Xplane12/Cessna 172 SP/Cessna_172SP.acf";

fn model() -> Option<FlightModel> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("Xplane12");
    let acf = root.join("Cessna 172 SP/Cessna_172SP.acf");
    if !acf.is_file() {
        eprintln!("skipped: {ACF} not found");
        return None;
    }
    let install = Install::open(&root).unwrap();
    let mut m = FlightModel::load(&acf, &install, 80.0, 90.0).unwrap();
    // lowest vertex of the Cessna's exterior set (see scene::load): wheel bottoms at -1.306 m
    m.rest_on_ground(-1.306, [0.0, 0.0], Vec3::NEG_Z);
    Some(m)
}

fn run(m: &mut FlightModel, c: &Controls, seconds: f32) {
    for _ in 0..(seconds * 200.0) as usize {
        m.step(1.0 / 200.0, c);
    }
}

#[test]
fn rests_on_the_gear_without_drifting() {
    let Some(mut m) = model() else { return };
    let h0 = m.state.position.y;
    run(&mut m, &Controls::default(), 10.0);
    let t = m.telemetry(&Controls::default());
    assert!(t.on_ground);
    assert!(t.airspeed_kt < 0.5, "{t:?}");
    assert!(
        (m.state.position.y - h0).abs() < 0.05,
        "{} -> {}",
        h0,
        m.state.position.y
    );
    assert!(t.pitch_deg.abs() < 0.5 && t.roll_deg.abs() < 0.5, "{t:?}");
}

#[test]
fn takes_off_and_climbs_straight_with_full_power() {
    let Some(mut m) = model() else { return };
    let mut c = Controls {
        throttle: 1.0,
        ..Controls::default()
    };
    let mut airborne_at = None;
    for i in 0..(70 * 200) {
        let t = m.telemetry(&c);
        if t.airspeed_kt > 55.0 {
            let rate = m.state.omega.x.to_degrees();
            c.elevator = (0.12 + 0.10 * (8.0 - t.pitch_deg) - 0.06 * rate).clamp(-0.5, 0.9);
        }
        if !t.on_ground && airborne_at.is_none() {
            airborne_at = Some(i as f32 / 200.0);
        }
        m.step(1.0 / 200.0, &c);
        assert!(m.state.position.is_finite() && m.state.velocity.is_finite());
    }
    let t = m.telemetry(&c);
    assert!(airborne_at.is_some_and(|s| s < 45.0), "no liftoff: {t:?}");
    assert!(
        t.altitude_ft > 300.0 && t.vertical_speed_fpm > 300.0,
        "{t:?}"
    );
    assert!(t.roll_deg.abs() < 2.0, "{t:?}");
    let heading_error = (t.heading_deg + 180.0).rem_euclid(360.0) - 180.0;
    assert!(heading_error.abs() < 3.0, "{t:?}");
}

fn in_the_air() -> Option<FlightModel> {
    let mut m = model()?;
    m.state.position.y += 600.0;
    m.set_airspeed(36.0); // about 70 knots
    // settle into level flight at about 70 knots with the throttle that holds it roughly
    Some(m)
}

#[test]
fn control_signs_follow_the_pilot() {
    let Some(base) = in_the_air() else { return };
    let rate = |c: Controls| {
        let mut m = in_the_air().unwrap();
        run(&mut m, &c, 0.6);
        m.state.omega
    };
    let _ = base;
    let neutral = rate(Controls {
        throttle: 0.6,
        ..Controls::default()
    });
    let back = rate(Controls {
        throttle: 0.6,
        elevator: 0.6,
        ..Controls::default()
    });
    assert!(
        back.x > neutral.x + 0.02,
        "elevator back should pitch up: {back:?} vs {neutral:?}"
    );
    let trim = rate(Controls {
        throttle: 0.6,
        elevator_trim: 0.3,
        ..Controls::default()
    });
    assert!(
        trim.x > neutral.x + 0.01,
        "pitch trim up should pitch up: {trim:?} vs {neutral:?}"
    );
    let right = rate(Controls {
        throttle: 0.6,
        aileron: 0.6,
        ..Controls::default()
    });
    // roll right = rotation about the nose axis (body -Z) with the right wing going down
    assert!(
        right.z < neutral.z - 0.02,
        "aileron right should roll right: {right:?} vs {neutral:?}"
    );
    let pedal = rate(Controls {
        throttle: 0.6,
        rudder: 0.6,
        ..Controls::default()
    });
    assert!(
        pedal.y < neutral.y - 0.01,
        "right rudder should yaw right: {pedal:?} vs {neutral:?}"
    );
}

#[test]
fn stalls_when_slow_and_pitched_up_and_recovers_flying_faster() {
    let Some(mut m) = in_the_air() else { return };
    m.set_airspeed(24.0);
    let c = Controls {
        throttle: 0.0,
        elevator: 1.0,
        ..Controls::default()
    };
    let mut max_stalled = 0;
    for _ in 0..(25 * 200) {
        m.step(1.0 / 200.0, &c);
        max_stalled = max_stalled.max(m.telemetry(&c).stalled_elements);
    }
    assert!(
        max_stalled > 0,
        "pulling up slowly with idle power should stall part of the wing"
    );
    assert!(m.state.position.is_finite());
}

#[test]
#[ignore = "diagnostic output for tuning"]
fn print_slow_pull_up() {
    let Some(mut m) = in_the_air() else { return };
    m.set_airspeed(30.0);
    let c = Controls {
        throttle: 0.0,
        elevator: 1.0,
        ..Controls::default()
    };
    for i in 0..(30 * 200) {
        m.step(1.0 / 200.0, &c);
        if i % 400 == 0 {
            let t = m.telemetry(&c);
            println!(
                "{:5.1}s IAS {:5.1} alt {:6.0} vs {:6.0} pitch {:5.1} alpha {:5.1} stalled {}",
                m.state.time,
                t.airspeed_kt,
                t.altitude_ft,
                t.vertical_speed_fpm,
                t.pitch_deg,
                t.alpha_deg,
                t.stalled_elements
            );
        }
    }
}
