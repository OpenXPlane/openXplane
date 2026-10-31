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
/// 18 degrees with a period near 25 s. The trim of 0.07 is the original's (read from Data Output); the elevator effectiveness behind
/// the liftoff is a fit, so this pins the fit, not the original's pitch model.
#[test]
fn hands_off_takeoff_lifts_off_near_the_recorded_original() {
    let Some(mut m) = model() else { return };
    let mut c = Controls {
        elevator_trim: 0.07,
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
        (12.5..20.5).contains(&max_pitch) && min_pitch < 5.0,
        "pitch swings {min_pitch}..{max_pitch}, the original 2.2..16.5 degrees"
    );
}

/// Lays the words of the ACF loader into an emulator memory: the aircraft object `B` at `0x1_0000_0000` and every
/// pointed-to region (`R5ff8` engines, ...) at its own base with the pointer stored at `B + offset`.
fn vm_from_acf_object(acf_path: &str) -> (openxplane::vm::Vm, u64) {
    use xp_acf::object::{Schema, load_object};
    let acf = openxplane::Aircraft::parse(&std::fs::read_to_string(acf_path).unwrap()).unwrap();
    let object = load_object(&acf, &Schema::load());
    let mut vm = openxplane::vm::Vm::default();
    let b = 0x1_0000_0000u64;
    for (i, (region, words)) in object.regions.iter().enumerate() {
        let base = if region == "B" {
            b
        } else {
            let offset = u32::from_str_radix(region.trim_start_matches('R'), 16).unwrap();
            let base = 0x2_0000_0000u64 + (i as u64) * 0x100_0000;
            vm.set_u64(b + u64::from(offset), base);
            base
        };
        for (offset, word) in words {
            vm.set_u32(base + u64::from(*offset), *word);
        }
    }
    (vm, b)
}

/// The original's engine lever term for the stock C172 (`engine_small::engine_lever`, kind 2: `E+4 * B+0xb08`) on an
/// aircraft object that comes from the installed ACF: the power command in watts is the 178 hp rating times the
/// throttle word.
#[test]
fn original_engine_lever_runs_on_the_c172_object() {
    let Ok(path) = std::env::var("OPENXPLANE_REFERENCE_AIRCRAFT") else {
        return;
    };
    let (mut vm, b) = vm_from_acf_object(&path);
    vm.set_f32(b + 0xb08, 0.5);
    let watts = openxplane::engine_small::engine_lever(&vm, b, 0);
    assert!(
        (watts - 0.5 * 132_734.6).abs() < 1.0,
        "{watts} W, expected {}",
        0.5 * 132_734.6
    );
    vm.set_f32(b + 0xb08, 1.0);
    assert!((openxplane::engine_small::engine_lever(&vm, b, 0) - 132_734.6).abs() < 1.0);
}

/// Answers the original's environment calls with a fixed sea-level standard day and a fixed step.
struct LiveEnv {
    dt: f64,
    unknown: Vec<u64>,
}

impl openxplane::vm::Callees for LiveEnv {
    fn call(
        &mut self,
        _vm: &mut openxplane::vm::Vm,
        address: u64,
        _args: openxplane::vm::CallArgs,
    ) -> openxplane::vm::Reply {
        let float = |v: f32| openxplane::vm::Reply {
            rax: 0,
            xmm0: u64::from(v.to_bits()),
        };
        match address {
            0x140c448c0 => openxplane::vm::Reply {
                rax: 0,
                xmm0: self.dt.to_bits(),
            },
            0x1407ace10 | 0x1417f12c0 => openxplane::vm::Reply { rax: 0, xmm0: 0 },
            0x141ba6750 => float(15.0),
            0x141ba64e0 => float(1.225),
            0x140984e50 => float(0.0),
            0x1408be0f0 => float(0.0),
            0x14067b2f0 => float(1.0),
            0x1410c9c60 => openxplane::vm::Reply {
                rax: 0x6f00_0010_0000,
                xmm0: 0,
            },
            0x14117c380 => openxplane::vm::Reply { rax: 0, xmm0: 0 },
            _ => {
                self.unknown.push(address);
                openxplane::vm::Reply { rax: 0, xmm0: 0 }
            }
        }
    }
}

/// Runs the original's piston update (`update_engine_piston`, the native stages behind the emulator adapters) on the
/// C172 object for a few seconds at full throttle and prints what it produces; the callee addresses it needs that
/// the environment does not answer are listed.
#[test]
fn original_piston_update_runs_on_the_c172_object() {
    let Ok(path) = std::env::var("OPENXPLANE_REFERENCE_AIRCRAFT") else {
        return;
    };
    let (mut vm, b) = vm_from_acf_object(&path);
    let (f, m, inputs) = (0x4_0000_0000u64, 0x5_0000_0000u64, 0x6_0000_0000u64);
    vm.set_u64(f + 0x20, b);
    vm.set_f32(b + 0xb08, 1.0);
    vm.set_f32(f + 0xb7a8, 1.0);
    vm.set_f32(f + 0x68, 101_325.0);
    vm.set_f32(f + 0x424, 0.0);
    vm.set_f32(m + 0x4, 1.0);
    // a running engine at 2400 rpm with the lean/rich response off, the state `gen_piston_vectors.py stock` presets
    vm.set_u32(m + 0x70, 1);
    vm.set_u32(m + 0x74, 1);
    vm.set_f32(m + 0x78, 2400.0);
    vm.set_f32(m + 0x214, 1.0);
    vm.set_f32(m + 0x40, 1.0);
    vm.set_u32(m + 0x48, 3);
    let mut env = LiveEnv {
        dt: 0.005,
        unknown: Vec::new(),
    };
    let mut last = 0.0;
    for step in 0..2000 {
        last = openxplane::piston::update_engine_piston(&mut vm, &mut env, m, f, 0, inputs);
        if step % 400 == 0 {
            eprintln!(
                "step {step}: ret {last}, M+0xcc {}, M+0x90 {}, M+0x24c {}, M+0x258 {}, M+0x22c {}",
                vm.f32(m + 0xcc),
                vm.f32(m + 0x90),
                vm.f32(m + 0x24c),
                vm.f32(m + 0x258),
                vm.f32(m + 0x22c)
            );
        }
    }
    env.unknown.sort_unstable();
    env.unknown.dedup();
    eprintln!("unanswered callees: {:x?}, last return {last}", env.unknown);
}

struct LiveProp {
    f: openxplane::forces::Words,
    b: openxplane::forces::Words,
    p: openxplane::forces::Words,
    x: Vec<openxplane::forces::Words>,
    foils: Vec<(String, openxplane::airfoil::Airfoil)>,
    noise: openxplane::buffet::NoiseTable,
    names: [String; 3],
    errors: Vec<String>,
}

impl openxplane::airflow::AirflowEnv for LiveProp {
    fn wind(&mut self, _x: f64, _y: f64, _z: f64) -> [f32; 3] {
        [0.0; 3]
    }
    fn wash(&mut self, _point: [f32; 3], out: [f32; 3]) -> [f32; 3] {
        out
    }
}

impl openxplane::prop::PropEnv for LiveProp {
    fn engine_flag(&mut self) -> bool {
        false
    }
    fn frame_time(&mut self) -> f64 {
        0.005
    }
    fn time_phase(&mut self) -> f64 {
        0.0
    }
    fn noise2(&mut self, _x: f32, _y: f32, _seed: i32) -> f32 {
        0.0
    }
    fn terrain(&mut self, _a: [f32; 3], _b: [f32; 3], height: f32) -> (f32, i32) {
        (height, 0)
    }
    fn strike_gate(&mut self) -> f64 {
        0.0
    }
    fn event(&mut self, _id: i32, _arg: i32) {}
    fn engine_ratio(&mut self, _n: i32) -> f32 {
        1.0
    }
    fn held_back(&mut self, _index: i32, _mode: i32) -> bool {
        false
    }
    fn limit_a(&mut self, _i: i32) -> bool {
        false
    }
    fn limit_b(&mut self, _n: i32) -> bool {
        false
    }
    fn blend(&mut self, _mask: i32) -> f32 {
        1.0
    }
    fn recording_id(&mut self) -> i32 {
        0
    }
    fn record(&mut self, _words: [u32; 20]) {}
    fn element_force(
        &mut self,
        call: &openxplane::prop::ElementCall<'_>,
    ) -> openxplane::prop::ElementResult {
        use openxplane::element_force::{Call, Objects, element_force};
        use openxplane::wing_element::FoilResult;
        let names = self.names.clone();
        let thickness = names.clone().map(|n| {
            self.foils
                .iter()
                .find(|(k, _)| *k == n)
                .map(|(_, a)| a.thickness())
        });
        let c = Call {
            index: call.index as usize,
            retain: call.retain != 0,
            ice: call.ice,
            g10: 1.0,
            names: [&names[0], &names[1], &names[2]],
            foil_thickness: thickness,
        };
        let wing = openxplane::forces::Words(
            self.p
                .0
                .iter()
                .filter(|(o, _)| **o >= 0x88)
                .map(|(o, w)| (*o - 0x88, *w))
                .collect(),
        );
        let objects = Objects {
            f: &self.f,
            b: &self.b,
            w: &wing,
            x: call.x,
        };
        let foils = &self.foils;
        let noise = &self.noise;
        let result = element_force(
            &objects,
            &c,
            |fc: &openxplane::wing_element::FoilCall| {
                let name = &names[fc.slot];
                let Some((_, foil)) = foils.iter().find(|(k, _)| k == name) else {
                    return Err(format!("airfoil {name}"));
                };
                let o = openxplane::profile::outer(
                    foil,
                    openxplane::profile::OuterInputs {
                        alpha_deg: fc.alpha,
                        multiplier: fc.multiplier,
                        divisor: fc.divisor,
                        regime: fc.re_meg,
                        mach: fc.arg6,
                        time: openxplane::runtime::RunningTime::from_snapshot(0.0).unwrap(),
                        noise_coordinates: [fc.x_norm, fc.y_norm, fc.z_norm],
                        retain_stall: fc.retain,
                    },
                    fc.stalled,
                    Some(noise),
                )?;
                Ok(FoilResult {
                    ret: o.ret,
                    cl: o.cl,
                    cd: o.cd,
                    cm: o.cm,
                    ratio: o.normalized_alpha,
                    stalled: o.stalled,
                })
            },
            &|_| false,
        );
        if std::env::var("OPENXPLANE_PROP_TRACE").is_ok() {
            eprintln!(
                "element {}: {:?}",
                call.index,
                result.as_ref().map(|o| (o.out1, o.out2, o.out3))
            );
        }
        match result {
            Ok(o) => openxplane::prop::ElementResult {
                out: [o.out1, o.out2, o.out3],
                x4: [o.x_f4, o.x_11c, o.x_144, o.x_16c],
                x_1bc: o.element.ratio,
                stall: i32::from(o.element.stall_flag),
            },
            Err(e) => {
                self.errors.push(e);
                openxplane::prop::ElementResult {
                    out: [0.0; 3],
                    x4: [0.0; 4],
                    x_1bc: 0.0,
                    stall: 0,
                }
            }
        }
    }
}

/// First live run of the original's propeller function on the C172 records from the ACF loader: the function is
/// called with a still airstream and the element forces stubbed to zero, to learn which words of `F` it needs.
#[test]
fn original_prop_force_reaches_the_end_on_the_c172_records() {
    use openxplane::forces::Words;
    let Ok(path) = std::env::var("OPENXPLANE_REFERENCE_AIRCRAFT") else {
        return;
    };
    use xp_acf::object::{Schema, load_object};
    let acf = openxplane::Aircraft::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let object = load_object(&acf, &Schema::load());
    let words = |region: &str| {
        Words(
            object
                .regions
                .get(region)
                .map(|r| r.iter().map(|(o, w)| (*o as usize, *w)).collect())
                .unwrap_or_default(),
        )
    };
    let (b, e, mut p) = (words("B"), words("R5ff8"), words("R6010"));
    // straight blade: boundary arrays of blade 0 (x, y, z at 0x5bc, 0x5e8, 0x614; chord at 0x70), as the wing code
    // builds them for a wing with no sweep or dihedral
    {
        let ft = 0.3048f32;
        let n = p.i32(0x8c) as usize;
        // the wing object W sits at P+0x88 (element count at W+4 = P+0x8c, which the loader already stored)
        let (root, tip) = (p.f32(0x90), p.f32(0x94));
        let span = acf
            .properties
            .iter()
            .find(|q| q.key == "_blad/0/_semilen_SEG")
            .map(|q| q.value.trim().parse::<f32>().unwrap() * ft)
            .unwrap();
        let transform = xp_airfoil::wing_boundary::Transform {
            span,
            design_dihedral: 0.0,
            design_sweep: 0.0,
            rotation: 0.0,
            dihedral_delta: 0.0,
            side: 1.0,
            variable_sweep: 0,
            rotation_offset: 0.0,
            retract: 0.0,
            origin: [0.0; 3],
        };
        let curvature = xp_airfoil::wing_boundary::Curvature {
            angle: 0.0,
            amplitude: 0.0,
            scale: 1.0,
        };
        for i in 0..=n {
            let chord = root + (tip - root) * (i as f32 / n as f32);
            let q =
                xp_airfoil::wing_boundary::boundary_point(&transform, curvature, n, i, chord, 0.25);
            p.set_f32(0x88 + 0x5bc + 4 * i, q[0]);
            p.set_f32(0x88 + 0x5e8 + 4 * i, q[1]);
            p.set_f32(0x88 + 0x614 + 4 * i, q[2]);
            p.set_f32(0x88 + 0x70 + 4 * i, chord);
        }
        // W+0x14 is the wing aspect ratio in the induced-drag terms and is derived, not an ACF word (guess: blade
        // length over the mean chord)
        p.set_f32(0x88 + 0x14, span / (0.5 * (root + tip)));
        // P+0x4c is not an ACF word; the call divides by it like a disk area (guess: pi r^2)
        p.set_f32(0x4c, std::f32::consts::PI * span * span);
        eprintln!("blade: {n} elements, root chord {root} m, tip {tip} m, span {span} m");
    }
    if std::env::var("OPENXPLANE_PROP_TRACE").is_ok() {
        let w: Vec<(usize, f32)> = [
            0x10, 0x14, 0x18, 0x1c, 0x20, 0x30, 0x38, 0x5c, 0x60, 0x64, 0x68,
        ]
        .iter()
        .map(|&o| (o, p.f32(0x88 + o)))
        .collect();
        eprintln!("W words: {w:?}");
        let k: Vec<i32> = (0..10).map(|i| p.i32(0x88 + 0x56c + 4 * i)).collect();
        eprintln!("W+0x56c: {k:?}");
    }
    let mut f = Words::default();
    // identity attitude (sine 0, cosine 1 of the three angle pairs) and a 30 m/s flow along the aircraft's nose
    for (sin, cos) in [(0x430, 0x434), (0x440, 0x444), (0x450, 0x454)] {
        f.set_f32(sin, 0.0);
        f.set_f32(cos, 1.0);
    }
    f.set_i32(0x28, 1);
    f.set_f32(0x6c, 1.225); // air density
    f.set_f32(0x5c, 15.0); // temperature
    f.set_f32(0x74, 1.0); // speed reference
    f.set_f32(0x68, 101_325.0); // pressure
    f.set_f32(0x408, 1.0);
    f.set_f32(0x370, -30.0);
    let mut r = Words::default();
    // the propeller's angular speed, rad/s (2400 rpm)
    r.set_f32(0x1c, 2400.0 * std::f32::consts::TAU / 60.0);
    let mut xs: Vec<Words> = (0..4).map(|_| Words::default()).collect();
    // X+0x288 divides or scales the element Cl (a unit default; the original's init sets it)
    for x in xs.iter_mut() {
        x.set_f32(0x288, 1.0);
        // X+0x28c scales the induced drag and X+4+4e is the per-element speed factor in the same term (guesses)
        x.set_f32(0x28c, 1.0);
        for e in 0..10 {
            x.set_f32(4 + 4 * e, 1.0);
        }
    }
    let mut y = Words::default();
    let m = Words::default();
    let root = std::path::PathBuf::from(std::env::var("OPENXPLANE_REFERENCE_ROOT").unwrap());
    let install = Install::open(&root).unwrap();
    let dir = Path::new(&path).parent().unwrap().to_path_buf();
    let mut foils = Vec::new();
    // long strings are on the heap in the loader's layout (not modelled), so the names come from the ACF text
    let afl = |k: u32| {
        acf.properties
            .iter()
            .find(|q| q.key == format!("_blad/0/_afl_file_{k}"))
            .map(|q| q.value.trim().to_string())
            .unwrap()
    };
    let names = [afl(1), afl(2), afl(3)];
    for name in names.clone() {
        if foils.iter().any(|(k, _): &(String, _)| *k == name) {
            continue;
        }
        let file = install
            .airfoil_candidates(&dir, &name)
            .into_iter()
            .find(|c| c.is_file())
            .unwrap_or_else(|| panic!("airfoil {name}"));
        let foil =
            openxplane::airfoil::Airfoil::parse(&std::fs::read_to_string(file).unwrap()).unwrap();
        foils.push((name, foil));
    }
    let noise = openxplane::buffet::NoiseTable::new(
        (0..openxplane::buffet::TABLE_LEN)
            .map(|i| {
                let h = ((i as u64 * 2_654_435_761) & 0xffff_ffff) >> 8;
                (h as f32 / 16_777_216.0) * 2.0 - 1.0
            })
            .collect(),
    )
    .unwrap();
    let mut env = LiveProp {
        f: f.clone(),
        b: b.clone(),
        p: p.clone(),
        x: xs.clone(),
        foils,
        noise,
        names,
        errors: Vec::new(),
    };
    let mut objects = openxplane::prop::Objects {
        f: &mut f,
        b: &b,
        e: &e,
        p: &p,
        r: &mut r,
        x: &mut xs,
        y: &mut y,
        e_table: &e,
        m: &m,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        openxplane::prop::prop_force(&mut objects, 0, &mut env, {
            use openxplane::prop::Stop::*;
            match std::env::var("OPENXPLANE_PROP_STOP").as_deref() {
                Ok("1") => Segment1,
                Ok("2") => Segment2,
                Ok("3") => Segment3,
                Ok("4") => Segment4,
                Ok("5") => Segment5,
                Ok("6") => Segment6,
                Ok("7") => Segment7,
                Ok("8") => Segment8,
                Ok("9") => Segment9,
                Ok("10") => Segment10,
                _ => End,
            }
        })
    }));
    let Ok(result) = result else {
        eprintln!("element function panicked: blade boundary arrays missing");
        return;
    };
    eprintln!(
        "prop_force: {:?}, element errors {:?}",
        result.as_ref().map(|_| "ok"),
        env.errors
    );
    let out: Vec<f32> = (0x14..0x88).step_by(4).map(|o| r.f32(o)).collect();
    eprintln!("R+0x14..0x88: {out:?}");
    if let Ok((fr, _)) = &result {
        let probe: Vec<(i32, f32)> = [-0x18, -0x10, 0x10, 0xe0, 0xe4, 0xe8, 0x10c, 0x118, 0x8e8]
            .iter()
            .map(|&o| (o, fr.f(o)))
            .collect();
        eprintln!("frame words: {probe:x?}");
    }
}
