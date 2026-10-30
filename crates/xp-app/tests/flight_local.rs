//! Behavioural checks of the approximate flight model on the provided Cessna 172 SP. They need the
//! reference installation in `Xplane12/` and are skipped when it is absent.
use glam::Vec3;
use openxplane::{
    flight::{Controls, FlightModel},
    install::Install,
};
use std::path::Path;

const ACF: &str = "Xplane12/Cessna 172 SP/Cessna_172SP.acf";

fn reference_root() -> std::path::PathBuf {
    std::env::var_os("OPENXPLANE_REFERENCE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Xplane12"))
}

fn model() -> Option<FlightModel> {
    let root = reference_root();
    let acf = std::env::var_os("OPENXPLANE_REFERENCE_AIRCRAFT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.join("Cessna 172 SP/Cessna_172SP.acf"));
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
fn physical_terrain_height_moves_contact_and_reset_together() {
    let Some(mut m) = model() else { return };
    let before = m.state.position.y;
    let ground = std::sync::Arc::new(openxplane::ground::Ground::new(vec![
        [
            [-1000.0, 25.0, -1000.0],
            [1000.0, 25.0, -1000.0],
            [-1000.0, 25.0, 1000.0],
        ],
        [
            [1000.0, 25.0, -1000.0],
            [1000.0, 25.0, 1000.0],
            [-1000.0, 25.0, 1000.0],
        ],
    ]));
    m.set_ground(ground.clone());
    m.set_ground(ground);
    assert!((m.state.position.y - before - 25.0).abs() < 1e-4);
    let raised = m.state.position.y;
    run(&mut m, &Controls::default(), 5.0);
    assert!(m.telemetry(&Controls::default()).on_ground);
    assert!((m.state.position.y - raised).abs() < 0.05);
    m.reset();
    assert_eq!(m.state.position.y, raised);
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

#[test]
fn swept_wings_run_through_delta_context_in_the_live_model() {
    let root = reference_root();
    let source = root.join("Cessna 172 SP/Cessna_172SP.acf");
    if !source.is_file() {
        eprintln!("skipped: {ACF} not found");
        return;
    }
    let text = std::fs::read_to_string(source).unwrap();
    let modified = text
        .lines()
        .map(|line| {
            for wing in 0..4 {
                let key = format!("P _wing/{wing}/_sweep_design ");
                if line.starts_with(&key) {
                    return format!("{key}60.0");
                }
            }
            line.to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "openxplane-swept-{}-{nonce}.acf",
        std::process::id()
    ));
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    file.write_all(modified.as_bytes()).unwrap();
    drop(file);
    let install = Install::open(&root).unwrap();
    let loaded = FlightModel::load(&path, &install, 80.0, 90.0);
    std::fs::remove_file(path).unwrap();
    let mut model = loaded.unwrap();
    assert_eq!(
        model
            .wings()
            .iter()
            .filter(|w| w.index < 4 && w.sweep_deg == 60.0)
            .count(),
        4
    );
    model.state.position.y = 600.0;
    model.state.orientation = glam::Quat::from_rotation_x(10.0_f32.to_radians());
    model.set_airspeed(45.0);
    // A downward flight-path component gives the swept surfaces nonzero incidence.
    model.state.velocity.y -= 5.0;
    for _ in 0..50 {
        model.step(1.0 / 200.0, &Controls::default());
        assert!(model.state.position.is_finite() && model.state.velocity.is_finite());
    }
    for wing in 0..4 {
        let report = model.wing_report.iter().find(|r| r.0 == wing).unwrap();
        assert!(report.1.is_finite() && report.2.is_finite());
        assert!(
            report.1.length() > 1.0,
            "swept wing {wing} contributes no force"
        );
    }
}

#[test]
fn animation_bindings_use_model_controls_and_original_gear_slots() {
    let Some(mut m) = model() else { return };
    let c = Controls {
        aileron: 0.5,
        elevator: 0.3,
        flaps: 1.0,
        elevator_trim: 0.2,
        ..Controls::default()
    };
    let left = m
        .animation_dataref("sim/flightmodel2/wing/aileron1_deg[2]", &c)
        .unwrap();
    let right = m
        .animation_dataref("sim/flightmodel2/wing/aileron1_deg[3]", &c)
        .unwrap();
    assert!(left * right < 0.0, "left={left}, right={right}");
    assert!(
        m.animation_dataref("sim/flightmodel2/wing/flap1_deg[0]", &c)
            .unwrap()
            > 0.0
    );
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/controls/elevator_trim", &c),
        Some(0.2)
    );
    m.gears[0].index = 7;
    m.gears[0].compression = 0.123;
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/gear/tire_vertical_deflection_mtr[7]", &c),
        Some(0.123)
    );
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/gear/tire_vertical_deflection_mtr[0]", &c),
        None
    );
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/engines/prop_rotation_angle_deg[0]", &c),
        None
    );
}

#[test]
fn default_state_animation_datarefs_are_deployed_rich_and_closed() {
    let Some(m) = model() else { return };
    let c = Controls::default();
    // The Cessna's gear is fixed: every modelled gear reads fully deployed, unknown slots read nothing.
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/gear/deploy_ratio[0]", &c),
        Some(1.0)
    );
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/gear/deploy_ratio[999]", &c),
        None
    );
    assert_eq!(
        m.animation_dataref("sim/cockpit2/engine/actuators/mixture_ratio[0]", &c),
        Some(1.0)
    );
    assert_eq!(
        m.animation_dataref("sim/flightmodel2/misc/door_open_ratio[3]", &c),
        Some(0.0)
    );
}

#[test]
fn flight_datarefs_follow_state_signs_and_units() {
    let Some(mut m) = model() else { return };
    let c = Controls::default();
    let get = |m: &FlightModel, n: &str| m.flight_dataref(n, &c).unwrap();
    // At rest on the ground: level attitude, no speed, no rates.
    assert_eq!(get(&m, "sim/flightmodel/position/true_airspeed"), 0.0);
    assert_eq!(get(&m, "sim/flightmodel/position/Q"), 0.0);
    assert!(get(&m, "sim/flightmodel/position/theta").abs() < 3.0);
    // Body axes: x right, y up, z back. Positive pitch rate about +x raises the nose; rolling right is
    // negative about z; yawing right is negative about y.
    m.state.omega = Vec3::new(0.1, 0.0, 0.0);
    assert!((get(&m, "sim/flightmodel/position/Q") - 0.1_f32.to_degrees()).abs() < 1e-4);
    m.state.omega = Vec3::new(0.0, 0.0, -0.2);
    assert!((get(&m, "sim/flightmodel/position/P") - 0.2_f32.to_degrees()).abs() < 1e-4);
    m.state.omega = Vec3::new(0.0, -0.3, 0.0);
    assert!((get(&m, "sim/flightmodel/position/R") - 0.3_f32.to_degrees()).abs() < 1e-4);
    // Vertical speed: the original keeps fpm and derives m/s with 0.00508.
    m.state.velocity = Vec3::new(0.0, 5.08, 0.0);
    let fpm = get(&m, "sim/flightmodel/position/vh_ind_fpm");
    assert!((fpm - 5.08 * 196.85).abs() < 0.01);
    assert!((get(&m, "sim/flightmodel/position/vh_ind") - fpm * 0.00508).abs() < 1e-3);
    assert!(
        m.flight_dataref("sim/flightmodel/position/not_a_dataref", &c)
            .is_none()
    );
    // Orientation sign check: a nose-up rotation increases theta.
    m.state.omega = Vec3::ZERO;
    m.state.velocity = Vec3::ZERO;
    m.state.orientation = glam::Quat::from_rotation_x(10.0_f32.to_radians()) * m.state.orientation;
    assert!(get(&m, "sim/flightmodel/position/theta") > 5.0);
}

#[test]
fn clock_datarefs_advance_with_simulation_time() {
    let Some(mut m) = model() else { return };
    let c = Controls::default();
    m.clock.zulu_start_sec = 3599.0;
    let seconds =
        |m: &FlightModel| m.animation_dataref("sim/cockpit2/clock_timer/zulu_time_seconds", &c);
    let hours =
        |m: &FlightModel| m.animation_dataref("sim/cockpit2/clock_timer/zulu_time_hours", &c);
    assert_eq!(hours(&m), Some(0.0));
    run(&mut m, &c, 2.1);
    assert_eq!(hours(&m), Some(1.0));
    assert_eq!(seconds(&m), Some(1.0));
}

#[test]
fn cold_and_dark_defaults_are_answered() {
    let Some(m) = model() else { return };
    let c = Controls::default();
    for name in [
        "sim/cockpit2/autopilot/autopilot_on",
        "sim/graphics/view/hide_yoke",
        "sim/time/hobbs_time",
        "sim/cockpit2/switches/landing_lights_on",
        "sim/cockpit2/radios/actuators/HSI_source_select_pilot",
        "sim/flightmodel2/misc/custom_slider_ratio[3]",
        "sim/cockpit2/switches/custom_slider_on[1]",
    ] {
        assert_eq!(m.animation_dataref(name, &c), Some(0.0), "{name}");
    }
}

#[test]
fn tires_roll_with_ground_speed_and_stay_still_at_rest() {
    let Some(mut m) = model() else { return };
    let name = "sim/flightmodel2/gear/tire_rotation_angle_deg[1]";
    let rest = Controls::default();
    run(&mut m, &rest, 1.0);
    let at_rest = m.animation_dataref(name, &rest).unwrap();
    assert!(at_rest < 5.0 || at_rest > 355.0, "at rest {at_rest}");
    let roll = Controls {
        throttle: 1.0,
        ..Default::default()
    };
    run(&mut m, &roll, 8.0);
    let angle = m.animation_dataref(name, &roll).unwrap();
    assert!(
        (0.0..360.0).contains(&angle) && (angle - at_rest).abs() > 1.0,
        "rolled {angle}"
    );
    assert!(
        m.telemetry(&roll).airspeed_kt > 10.0,
        "the take-off roll built up speed"
    );
    let radius = m.gears.iter().find(|g| g.index == 1).unwrap().tire_radius;
    assert!(radius > 0.1 && radius < 0.5, "radius {radius} m");
}

/// Times for the full-throttle roll from rest, taken from the original's Data Output
/// (`research/REFERENCE_TAKEOFF_C172.md`), one recording, so the tolerance is loose.
#[test]
fn full_throttle_roll_follows_the_recorded_original() {
    let Some(mut m) = model() else { return };
    let c = Controls {
        throttle: 1.0,
        ..Default::default()
    };
    let reference = [(20.0, 4.2), (40.0, 10.9), (60.0, 18.6), (70.0, 23.1)];
    let mut reached = [f32::NAN; 4];
    let dt = 1.0 / 200.0;
    for step in 0..(40.0 / dt) as usize {
        m.step(dt, &c);
        let kt = m.telemetry(&c).airspeed_kt;
        for (slot, (speed, _)) in reached.iter_mut().zip(reference) {
            if slot.is_nan() && kt >= speed {
                *slot = step as f32 * dt;
            }
        }
    }
    for ((speed, want), got) in reference.iter().zip(reached) {
        assert!(
            (got - want).abs() < 2.0,
            "{speed} kt after {got} s, original {want} s"
        );
    }
}

/// Hands-off takeoff against `research/reference/c172_noinput_uhww_20261030.csv`: full power, no yoke input.
/// The original lifts off 33.8 s after the roll starts at 86 kias and then swings in pitch between about 3 and
/// 18 degrees with a period near 25 s. The elevator trim of 0.24 is a fit to that liftoff (the original's
/// start trim is not read yet), so this pins the fit, not the original's pitch model.
#[test]
fn hands_off_takeoff_lifts_off_near_the_recorded_original() {
    let Some(mut m) = model() else { return };
    let mut c = Controls {
        elevator_trim: 0.24,
        ..Default::default()
    };
    let dt = 1.0 / 200.0;
    let (mut rolling, mut liftoff) = (None, None);
    let (mut min_pitch, mut max_pitch) = (f32::MAX, f32::MIN);
    for step in 0..(90.0 / dt) as usize {
        let time = step as f32 * dt;
        if time > 2.0 {
            c.throttle = 1.0;
        }
        let t = m.telemetry(&c);
        if rolling.is_none() && t.airspeed_kt > 1.0 {
            rolling = Some(time);
        }
        if let (Some(start), None) = (rolling, liftoff)
            && !t.on_ground
        {
            liftoff = Some((time - start, t.airspeed_kt));
        }
        if let Some((since, _)) = liftoff
            && time - rolling.unwrap() - since > 5.0
        {
            min_pitch = min_pitch.min(t.pitch_deg);
            max_pitch = max_pitch.max(t.pitch_deg);
        }
        m.step(dt, &c);
    }
    let (since, kt) = liftoff.expect("no liftoff");
    assert!(
        (since - 33.8).abs() < 4.0,
        "liftoff {since} s, original 33.8 s"
    );
    assert!(
        (kt - 86.0).abs() < 5.0,
        "liftoff at {kt} kt, original 86 kt"
    );
    assert!(
        max_pitch - min_pitch > 6.0,
        "pitch stays within {min_pitch}..{max_pitch}, the original swings by about 15 degrees"
    );
}
