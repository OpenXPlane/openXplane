//! What the default commands of the reference build do to this engine's aircraft controls.
//! The keys come from `keymap`; only the commands that mean something to the approximate flight
//! model act (throttle, flaps, brakes, trims, pause, view); the others are recognised and reported
//! as not simulated. Step sizes are openXplane's choice: the original's are not recovered.
use crate::flight::Controls;
use xp_dataref::commands::Phase;

/// Throttle change per press or key repeat for "a bit".
pub const THROTTLE_STEP: f32 = 0.05;
pub const TRIM_STEP: f32 = 0.01;
/// "Regular" brake effort while the key is held.
pub const BRAKE_REGULAR: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewAction {
    Default,
    ToggleFree,
    /// Move the view by this much along (right, up) in metres.
    Shift(f32, f32),
    Rotate {
        yaw: f32,
        pitch: f32,
    },
    Zoom(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    /// The aircraft controls changed.
    Controls,
    Pause,
    View(ViewAction),
    /// The command is known but the engine has nothing for it to act on (yet).
    NotSimulated,
    Unknown,
}

fn bump(value: &mut f32, delta: f32, lo: f32, hi: f32) {
    *value = (*value + delta).clamp(lo, hi);
}

/// Applies a command in the given phase to the controls. `Phase::Begin` and `Phase::Continue` (key
/// repeat) both act on step commands; hold commands act on begin and end.
pub fn apply_command(c: &mut Controls, command: &str, phase: Phase) -> Effect {
    let acts = phase != Phase::End;
    match command {
        "sim/engines/throttle_up" if acts => bump(&mut c.throttle, THROTTLE_STEP, 0.0, 1.0),
        "sim/engines/throttle_down" if acts => bump(&mut c.throttle, -THROTTLE_STEP, 0.0, 1.0),
        "sim/engines/throttle_full" if phase == Phase::Begin => c.throttle = 1.0,
        "sim/flight_controls/flaps_up" if phase == Phase::Begin => {
            bump(&mut c.flaps, -1.0 / 3.0, 0.0, 1.0)
        }
        "sim/flight_controls/flaps_down" if phase == Phase::Begin => {
            bump(&mut c.flaps, 1.0 / 3.0, 0.0, 1.0)
        }
        "sim/flight_controls/brakes_regular" => {
            c.brake = if phase == Phase::End {
                0.0
            } else {
                BRAKE_REGULAR
            };
        }
        "sim/flight_controls/brakes_toggle_max" if phase == Phase::Begin => {
            c.brake = if c.brake >= 1.0 { 0.0 } else { 1.0 };
        }
        "sim/flight_controls/pitch_trim_up" if acts => {
            bump(&mut c.elevator_trim, TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/pitch_trim_down" if acts => {
            bump(&mut c.elevator_trim, -TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/aileron_trim_left" if acts => {
            bump(&mut c.aileron_trim, -TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/aileron_trim_right" if acts => {
            bump(&mut c.aileron_trim, TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/aileron_trim_center" if phase == Phase::Begin => c.aileron_trim = 0.0,
        "sim/flight_controls/rudder_trim_left" if acts => {
            bump(&mut c.rudder_trim, -TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/rudder_trim_right" if acts => {
            bump(&mut c.rudder_trim, TRIM_STEP, -1.0, 1.0)
        }
        "sim/flight_controls/rudder_trim_center" if phase == Phase::Begin => c.rudder_trim = 0.0,
        "sim/operation/pause_toggle" if phase == Phase::Begin => return Effect::Pause,
        "sim/view/default_view" if phase == Phase::Begin => {
            return Effect::View(ViewAction::Default);
        }
        "sim/view/free_camera" if phase == Phase::Begin => {
            return Effect::View(ViewAction::ToggleFree);
        }
        "sim/general/left" if acts => return Effect::View(ViewAction::Shift(-0.5, 0.0)),
        "sim/general/right" if acts => return Effect::View(ViewAction::Shift(0.5, 0.0)),
        "sim/general/up" if acts => return Effect::View(ViewAction::Shift(0.0, 0.5)),
        "sim/general/down" if acts => return Effect::View(ViewAction::Shift(0.0, -0.5)),
        "sim/general/rot_left" if acts => {
            return Effect::View(ViewAction::Rotate {
                yaw: 0.04,
                pitch: 0.0,
            });
        }
        "sim/general/rot_right" if acts => {
            return Effect::View(ViewAction::Rotate {
                yaw: -0.04,
                pitch: 0.0,
            });
        }
        "sim/general/rot_up" if acts => {
            return Effect::View(ViewAction::Rotate {
                yaw: 0.0,
                pitch: 0.03,
            });
        }
        "sim/general/rot_down" if acts => {
            return Effect::View(ViewAction::Rotate {
                yaw: 0.0,
                pitch: -0.03,
            });
        }
        "sim/general/zoom_in" if acts => return Effect::View(ViewAction::Zoom(-1.0)),
        "sim/general/zoom_out" if acts => return Effect::View(ViewAction::Zoom(1.0)),
        "sim/general/forward" if acts => return Effect::View(ViewAction::Zoom(-2.0)),
        "sim/general/backward" if acts => return Effect::View(ViewAction::Zoom(2.0)),
        _ if is_stateless_noop(command, phase) => return Effect::Controls,
        _ if xp_dataref::keymap::key_for_command(command).is_some() => return Effect::NotSimulated,
        _ => return Effect::Unknown,
    }
    Effect::Controls
}

/// Phases in which a handled command has nothing more to do (for example the end of a step command).
fn is_stateless_noop(command: &str, phase: Phase) -> bool {
    phase == Phase::End
        && matches!(
            command,
            "sim/engines/throttle_up"
                | "sim/engines/throttle_down"
                | "sim/engines/throttle_full"
                | "sim/flight_controls/flaps_up"
                | "sim/flight_controls/flaps_down"
                | "sim/flight_controls/brakes_toggle_max"
                | "sim/flight_controls/pitch_trim_up"
                | "sim/flight_controls/pitch_trim_down"
                | "sim/flight_controls/aileron_trim_left"
                | "sim/flight_controls/aileron_trim_right"
                | "sim/flight_controls/aileron_trim_center"
                | "sim/flight_controls/rudder_trim_left"
                | "sim/flight_controls/rudder_trim_right"
                | "sim/flight_controls/rudder_trim_center"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(c: &mut Controls, cmd: &str, phase: Phase) -> Effect {
        apply_command(c, cmd, phase)
    }

    #[test]
    fn throttle_steps_clamp_and_full_sets_one() {
        let mut c = Controls::default();
        for _ in 0..30 {
            run(&mut c, "sim/engines/throttle_up", Phase::Begin);
        }
        assert_eq!(c.throttle, 1.0);
        run(&mut c, "sim/engines/throttle_down", Phase::Continue);
        assert!((c.throttle - 0.95).abs() < 1e-6);
        c.throttle = 0.1;
        run(&mut c, "sim/engines/throttle_full", Phase::Begin);
        assert_eq!(c.throttle, 1.0);
        for _ in 0..30 {
            run(&mut c, "sim/engines/throttle_down", Phase::Begin);
        }
        assert_eq!(c.throttle, 0.0);
    }

    #[test]
    fn flaps_move_one_detent_per_press_and_stay_in_range() {
        let mut c = Controls::default();
        run(&mut c, "sim/flight_controls/flaps_down", Phase::Begin);
        assert!((c.flaps - 1.0 / 3.0).abs() < 1e-6);
        for _ in 0..5 {
            run(&mut c, "sim/flight_controls/flaps_down", Phase::Begin);
        }
        assert_eq!(c.flaps, 1.0);
        // key repeat does not skip detents
        run(&mut c, "sim/flight_controls/flaps_up", Phase::Continue);
        assert_eq!(c.flaps, 1.0);
        for _ in 0..5 {
            run(&mut c, "sim/flight_controls/flaps_up", Phase::Begin);
        }
        assert_eq!(c.flaps, 0.0);
    }

    #[test]
    fn brakes_hold_and_toggle_max() {
        let mut c = Controls::default();
        run(&mut c, "sim/flight_controls/brakes_regular", Phase::Begin);
        assert_eq!(c.brake, BRAKE_REGULAR);
        run(&mut c, "sim/flight_controls/brakes_regular", Phase::End);
        assert_eq!(c.brake, 0.0);
        run(
            &mut c,
            "sim/flight_controls/brakes_toggle_max",
            Phase::Begin,
        );
        assert_eq!(c.brake, 1.0);
        run(&mut c, "sim/flight_controls/brakes_toggle_max", Phase::End);
        assert_eq!(c.brake, 1.0);
        run(
            &mut c,
            "sim/flight_controls/brakes_toggle_max",
            Phase::Begin,
        );
        assert_eq!(c.brake, 0.0);
    }

    #[test]
    fn trims_step_clamp_and_centre() {
        let mut c = Controls::default();
        for _ in 0..3 {
            run(&mut c, "sim/flight_controls/pitch_trim_up", Phase::Continue);
        }
        assert!((c.elevator_trim - 3.0 * TRIM_STEP).abs() < 1e-6);
        run(
            &mut c,
            "sim/flight_controls/aileron_trim_left",
            Phase::Begin,
        );
        run(
            &mut c,
            "sim/flight_controls/rudder_trim_right",
            Phase::Begin,
        );
        assert!(c.aileron_trim < 0.0 && c.rudder_trim > 0.0);
        run(
            &mut c,
            "sim/flight_controls/aileron_trim_center",
            Phase::Begin,
        );
        run(
            &mut c,
            "sim/flight_controls/rudder_trim_center",
            Phase::Begin,
        );
        assert_eq!((c.aileron_trim, c.rudder_trim), (0.0, 0.0));
        for _ in 0..500 {
            run(&mut c, "sim/flight_controls/pitch_trim_down", Phase::Begin);
        }
        assert_eq!(c.elevator_trim, -1.0);
    }

    #[test]
    fn non_control_commands_report_their_effect() {
        let mut c = Controls::default();
        assert_eq!(
            run(&mut c, "sim/operation/pause_toggle", Phase::Begin),
            Effect::Pause
        );
        assert_eq!(
            run(&mut c, "sim/operation/pause_toggle", Phase::End),
            Effect::NotSimulated
        );
        assert_eq!(
            run(&mut c, "sim/view/default_view", Phase::Begin),
            Effect::View(ViewAction::Default)
        );
        assert_eq!(
            run(&mut c, "sim/engines/mixture_up", Phase::Begin),
            Effect::NotSimulated
        );
        assert_eq!(
            run(&mut c, "sim/nonsense/command", Phase::Begin),
            Effect::Unknown
        );
        // step commands end quietly
        assert_eq!(
            run(&mut c, "sim/engines/throttle_up", Phase::End),
            Effect::Controls
        );
    }
}
