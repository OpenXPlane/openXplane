//! Recovered Windows XPLMKeyPressHook dispatch. Plugin context, validation,
//! user callbacks and deferred-list service bodies are explicit boundaries.
use crate::keyboard_dispatch::InterceptInput;

#[derive(Clone, Copy, Debug, Default)]
pub struct Iteration {
    pub depth: i32,
    pub pending: u8,
}
#[derive(Clone, Debug)]
pub struct Sniffer {
    pub deleted: u8,
    pub before: i32,
    pub plugin: i32,
    pub refcon: u64,
}
#[derive(Clone, Debug)]
pub struct Hotkey {
    pub deleted: u8,
    pub key: u8,
    pub flags: u32,
    pub plugin: i32,
    pub refcon: u64,
}
#[derive(Clone, Debug)]
pub struct Window {
    pub token: u64,
    pub plugin: i32,
    pub refcon: u64,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub sniffers: Vec<Sniffer>,
    pub hotkeys: Vec<Hotkey>,
    pub focused: Option<Window>,
    pub sniffer_iteration: Iteration,
    pub hotkey_iteration: Iteration,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum List {
    Sniffers,
    Hotkeys,
}
pub trait Services {
    fn validate(&mut self, stage: u8);
    /// Context acquisition plus activation check; initialized-TLS bodies are in xplm_context.
    fn enter(&mut self, state: &mut State, plugin: i32) -> bool;
    fn leave(&mut self, state: &mut State, plugin: i32);
    fn sniff(&mut self, state: &mut State, index: usize, input: InterceptInput, refcon: u64)
    -> i32;
    fn hotkey(&mut self, state: &mut State, index: usize, refcon: u64);
    fn window(&mut self, state: &mut State, token: u64, input: InterceptInput, refcon: u64);
    /// Original validate/drain-deleted/validate sequence; pending clears between steps 1 and 2.
    fn flush(&mut self, state: &mut State, list: List, step: u8);
}
fn leave_iteration(state: &mut State, list: List, services: &mut impl Services) {
    let iteration = match list {
        List::Sniffers => &mut state.sniffer_iteration,
        List::Hotkeys => &mut state.hotkey_iteration,
    };
    iteration.depth = iteration.depth.wrapping_sub(1).max(0);
    if iteration.depth == 0 && iteration.pending != 0 {
        services.flush(state, list, 0);
        services.flush(state, list, 1);
        match list {
            List::Sniffers => state.sniffer_iteration.pending = 0,
            List::Hotkeys => state.hotkey_iteration.pending = 0,
        }
        services.flush(state, list, 2);
    }
}
/// Export 0x18001a670 and complete body 0x18000b6f0. Explicit synthetic state;
/// list topology/lifetime changes, recursive callbacks and assertion services are not modeled.
pub fn dispatch(
    state: &mut State,
    input: InterceptInput,
    target: u64,
    services: &mut impl Services,
) -> u32 {
    services.validate(0);
    services.validate(1);
    if input.character == 0 && input.virtual_key == 0 && input.flags == 0 {
        return 1;
    }
    if input.phase == 0 || input.phase == 2 {
        state.sniffer_iteration.depth = state.sniffer_iteration.depth.wrapping_add(1);
        for index in 0..state.sniffers.len() {
            let sniffer = &state.sniffers[index];
            let eligible = if input.phase == 0 {
                sniffer.deleted == 0 && sniffer.before != 0
            } else {
                sniffer.before == 0
            };
            if !eligible {
                continue;
            }
            let plugin = sniffer.plugin;
            let active = services.enter(state, plugin);
            if active {
                let refcon = state.sniffers[index].refcon;
                if services.sniff(state, index, input, refcon) == 0 {
                    leave_iteration(state, List::Sniffers, services);
                    services.leave(state, plugin);
                    return 0;
                }
            }
            services.leave(state, plugin);
        }
        leave_iteration(state, List::Sniffers, services);
        if input.phase == 2 {
            return 1;
        }
        state.hotkey_iteration.depth = state.hotkey_iteration.depth.wrapping_add(1);
        for index in 0..state.hotkeys.len() {
            let hotkey = &state.hotkeys[index];
            if hotkey.deleted != 0 || hotkey.key != input.virtual_key || hotkey.flags != input.flags
            {
                continue;
            }
            let plugin = hotkey.plugin;
            if services.enter(state, plugin) {
                let refcon = state.hotkeys[index].refcon;
                services.hotkey(state, index, refcon);
                services.leave(state, plugin);
                // The observed matched-hotkey path returns without closing its iteration.
                return 0;
            }
            services.leave(state, plugin);
        }
        leave_iteration(state, List::Hotkeys, services);
        return 1;
    }
    if input.phase == 1 {
        let Some(window) = state.focused.as_ref() else {
            return 1;
        };
        if target != 0 && target != window.token {
            return 1;
        }
        let plugin = window.plugin;
        if services.enter(state, plugin) {
            let window = state
                .focused
                .as_ref()
                .expect("native focused window must remain valid");
            let token = window.token;
            let refcon = window.refcon;
            services.window(state, token, input, refcon);
            services.leave(state, plugin);
            return 0;
        }
        services.leave(state, plugin);
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Calls(Vec<String>);
    impl Services for Calls {
        fn validate(&mut self, _: u8) {}
        fn enter(&mut self, _: &mut State, _: i32) -> bool {
            true
        }
        fn leave(&mut self, _: &mut State, _: i32) {}
        fn sniff(&mut self, _: &mut State, i: usize, _: InterceptInput, _: u64) -> i32 {
            self.0.push(format!("sniff{i}"));
            1
        }
        fn hotkey(&mut self, _: &mut State, i: usize, _: u64) {
            self.0.push(format!("hotkey{i}"));
        }
        fn window(&mut self, _: &mut State, _: u64, _: InterceptInput, _: u64) {}
        fn flush(&mut self, _: &mut State, _: List, _: u8) {}
    }
    fn input(phase: u32) -> InterceptInput {
        InterceptInput {
            character: 27,
            virtual_key: 27,
            flags: 8,
            phase,
        }
    }
    #[test]
    fn after_sniffer_does_not_reuse_before_phase_deleted_gate() {
        let mut s = State {
            sniffers: vec![Sniffer {
                deleted: 1,
                before: 0,
                plugin: 0,
                refcon: 0,
            }],
            ..State::default()
        };
        let mut calls = Calls::default();
        assert_eq!(dispatch(&mut s, input(0), 0, &mut calls), 1);
        assert!(calls.0.is_empty());
        assert_eq!(dispatch(&mut s, input(2), 0, &mut calls), 1);
        assert_eq!(calls.0, ["sniff0"]);
    }
    #[test]
    fn matched_hotkey_retains_observed_iteration_depth() {
        let mut s = State {
            hotkeys: vec![Hotkey {
                deleted: 0,
                key: 27,
                flags: 8,
                plugin: 0,
                refcon: 0,
            }],
            ..State::default()
        };
        let mut calls = Calls::default();
        assert_eq!(dispatch(&mut s, input(0), 0, &mut calls), 0);
        assert_eq!(s.hotkey_iteration.depth, 1);
        assert_eq!(calls.0, ["hotkey0"]);
    }
}
