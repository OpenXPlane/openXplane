//! Native Begin/End command dispatch with explicit callback effects.
use crate::command_gate::CommandGate;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Begin,
    End,
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
}

pub trait Services {
    fn callback(&mut self, state: &mut State, object: u64, command: i32, phase: Phase);
    /// Native arguments are (command, Begin=1/End=0, Begin=0/End=1, before=1/after=0).
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

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Effects {
        calls: Vec<u64>,
        mutate: bool,
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
