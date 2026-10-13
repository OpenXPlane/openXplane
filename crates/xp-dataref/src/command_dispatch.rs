//! Native Begin/End command dispatch with explicit callback effects.
use crate::command_gate::CommandGate;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Begin,
    End,
    Continue,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CommandState {
    pub count: i32,
    pub started: f32,
    /// Opaque primary callback object; zero is null.
    pub primary: u64,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    pub commands: Vec<CommandState>,
    /// Ordered opaque object identities, matching unsigned native pointer ordering.
    pub observers: BTreeSet<u64>,
    pub overrides: BTreeMap<i32, u64>,
    pub gate: CommandGate,
    pub block_byte: u8,
    pub now: f64,
    pub dt: f32,
    /// Raw owner+0 float written by Continue.
    pub rate_delta: f32,
}

pub trait Services {
    fn callback(&mut self, state: &mut State, object: u64, command: i32, phase: Phase);
    /// Native args: (command, begin, end, before). Continue uses begin=end=0.
    fn intercept(&mut self, state: &mut State, command: i32, phase: Phase, before: bool) -> i32;
}

/// Complete routing order of 0x140c7fac0 / 0x140c7fc40, with supplied service effects.
/// Invalid vector indices return an error instead of reproducing unsafe memory access.
/// Observer snapshot/iteration order is preserved; allocator effects are not modeled.
pub fn dispatch(
    state: &mut State,
    command: i32,
    source_mask: u32,
    phase: Phase,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if phase == Phase::Begin
        && (state.block_byte != 0 || state.gate.is_blocked(command, source_mask))
    {
        return Ok(());
    }
    let slot = usize::try_from(command)
        .ok()
        .and_then(|i| state.commands.get_mut(i))
        .ok_or("invalid native command-vector index")?;
    match phase {
        Phase::Begin => {
            if slot.count == 0 {
                slot.started = state.now as f32;
            }
            slot.count = slot.count.wrapping_add(1);
        }
        Phase::Continue => return Err("use continue_command for Continue callbacks"),
        Phase::End => {
            if slot.count > 0 {
                slot.count -= 1;
            }
        }
    }
    if let Some(object) = state.overrides.get(&command).copied().filter(|&o| o != 0) {
        services.callback(state, object, command, phase);
        return Ok(());
    }
    let snapshot: Vec<_> = state.observers.iter().copied().collect();
    for (index, object) in snapshot.into_iter().enumerate() {
        // The first snapshot entry is called immediately. Later entries are revalidated
        // against the current registry, so removals during earlier callbacks are observed.
        if index == 0 || state.observers.contains(&object) {
            services.callback(state, object, command, phase);
        }
    }
    if services.intercept(state, command, phase, true) != 0 {
        let object = state.commands[command as usize].primary;
        if object != 0 {
            services.callback(state, object, command, phase);
        }
    }
    services.intercept(state, command, phase, false);
    Ok(())
}

/// Continue virtual slot +0x8 also receives cached elapsed time and current dt.
pub trait ContinueServices: Services {
    fn continued(&mut self, state: &mut State, object: u64, command: i32, elapsed: f32, dt: f32);
}

/// Entire native Continue 0x140c7f8c0, including a real Begin when count is zero.
/// Callback changes made by Begin are observed, without repeating the initial gates.
/// Finite float arithmetic preserves the original subtraction/cast and f32 ramp order.
#[allow(clippy::manual_clamp)] // Keep the recovered comparison order explicit.
pub fn continue_command(
    state: &mut State,
    command: i32,
    source_mask: u32,
    services: &mut impl ContinueServices,
) -> Result<(), &'static str> {
    if state.block_byte != 0 || state.gate.is_blocked(command, source_mask) {
        return Ok(());
    }
    let index = usize::try_from(command)
        .ok()
        .filter(|&i| i < state.commands.len())
        .ok_or("invalid native command-vector index")?;
    if state.commands[index].count == 0 {
        dispatch(state, command, source_mask, Phase::Begin, services)?;
    }
    let elapsed = (state.now - f64::from(state.commands[index].started)) as f32;
    let ramp = (elapsed - 1.0) * 3.0 + 1.0;
    let ramp = if 1.0 > ramp {
        1.0
    } else if 4.0 < ramp {
        4.0
    } else {
        ramp
    };
    state.rate_delta = ramp * state.dt;
    if let Some(object) = state.overrides.get(&command).copied().filter(|&o| o != 0) {
        let dt = state.dt;
        services.continued(state, object, command, elapsed, dt);
        return Ok(());
    }
    let snapshot: Vec<_> = state.observers.iter().copied().collect();
    for (i, object) in snapshot.into_iter().enumerate() {
        if i == 0 || state.observers.contains(&object) {
            let dt = state.dt;
            services.continued(state, object, command, elapsed, dt);
        }
    }
    if services.intercept(state, command, Phase::Continue, true) != 0 {
        let object = state.commands[index].primary;
        if object != 0 {
            let dt = state.dt;
            services.continued(state, object, command, elapsed, dt);
        }
    }
    services.intercept(state, command, Phase::Continue, false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Effects {
        calls: Vec<u64>,
        mutate: bool,
        held: Vec<(u64, f32, f32)>,
    }
    impl Services for Effects {
        fn callback(&mut self, state: &mut State, object: u64, _: i32, _: Phase) {
            self.calls.push(object);
            if self.mutate && object == 1 {
                state.observers.remove(&2);
                state.observers.insert(4);
            }
        }
        fn intercept(&mut self, _: &mut State, _: i32, _: Phase, _: bool) -> i32 {
            1
        }
    }
    impl ContinueServices for Effects {
        fn continued(
            &mut self,
            state: &mut State,
            object: u64,
            command: i32,
            elapsed: f32,
            dt: f32,
        ) {
            self.callback(state, object, command, Phase::Continue);
            self.held.push((object, elapsed, dt));
        }
    }
    #[test]
    fn continue_begins_zero_count_and_preserves_ramp_order() {
        let mut state = State {
            commands: vec![
                CommandState {
                    primary: 5,
                    ..CommandState::default()
                };
                4
            ],
            now: 2.75,
            dt: 0.25,
            ..State::default()
        };
        let mut effects = Effects::default();
        continue_command(&mut state, 3, 0, &mut effects).unwrap();
        assert_eq!(state.commands[3].count, 1);
        assert_eq!(state.commands[3].started, 2.75);
        assert_eq!(state.rate_delta, 0.25);
        assert_eq!(effects.calls, [5, 5]);
        state.now = 4.25;
        continue_command(&mut state, 3, 0, &mut effects).unwrap();
        assert_eq!(state.commands[3].count, 1);
        assert_eq!(state.rate_delta, 0.625);
        state.now = 9.75;
        continue_command(&mut state, 3, 0, &mut effects).unwrap();
        assert_eq!(state.rate_delta, 1.0);
        assert_eq!(
            effects.held,
            [(5, 0.0, 0.25), (5, 1.5, 0.25), (5, 7.0, 0.25)]
        );
    }
    #[test]
    fn repeated_begin_preserves_first_time_and_blocked_end_still_releases() {
        let mut state = State {
            commands: vec![
                CommandState {
                    primary: 5,
                    ..CommandState::default()
                };
                4
            ],
            now: 1.5,
            ..State::default()
        };
        let mut effects = Effects::default();
        dispatch(&mut state, 3, 0, Phase::Begin, &mut effects).unwrap();
        state.now = 2.5;
        dispatch(&mut state, 3, 0, Phase::Begin, &mut effects).unwrap();
        assert_eq!(state.commands[3].count, 2);
        assert_eq!(state.commands[3].started, 1.5);
        state.block_byte = 255;
        for _ in 0..3 {
            dispatch(&mut state, 3, 0, Phase::End, &mut effects).unwrap();
        }
        assert_eq!(state.commands[3].count, 0);
        assert_eq!(state.commands[3].started, 1.5);
        assert_eq!(effects.calls, [5, 5, 5, 5, 5]);
    }
    #[test]
    fn observer_removals_take_effect_but_additions_wait_for_next_snapshot() {
        let mut state = State {
            commands: vec![
                CommandState {
                    primary: 7,
                    ..CommandState::default()
                };
                4
            ],
            observers: [1, 2, 3].into_iter().collect(),
            ..State::default()
        };
        let mut effects = Effects {
            mutate: true,
            ..Effects::default()
        };
        dispatch(&mut state, 3, 0, Phase::Begin, &mut effects).unwrap();
        assert_eq!(effects.calls, [1, 3, 7]);
        effects.calls.clear();
        dispatch(&mut state, 3, 0, Phase::End, &mut effects).unwrap();
        assert_eq!(effects.calls, [1, 3, 4, 7]);
    }
}
