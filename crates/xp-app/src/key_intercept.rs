//! Native keyboard interceptor wrapper and its observed registered adapter.
//! Clock, scope services, exported hook and context helpers remain explicit boundaries.
use crate::keyboard_dispatch::InterceptInput;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub registered: bool,
    pub export_present: bool,
    pub mode: i32,
    pub scope_mask: u32,
    pub enabled_mask: u32,
    pub scope_context: u64,
    pub elapsed: f64,
}

pub trait Services {
    fn clock(&mut self, state: &mut State) -> f64;
    fn enter_scope(&mut self, state: &mut State, context: u64) -> u64;
    fn leave_scope(&mut self, state: &mut State, context: u64, token: u64);
    fn enter_hook(&mut self, state: &mut State) -> i32;
    fn call_hook(&mut self, state: &mut State, input: InterceptInput, refcon: u64) -> i32;
    fn leave_hook(&mut self, state: &mut State, token: i32);
}

/// Outer wrapper 0x141913800 and observed adapter 0x141917f20.
/// This models the observed adapter, not arbitrary replacement of the registered function.
/// Hook absence inside the adapter returns 0, unlike absent outer registration (1).
/// Endpoint mutations of elapsed/context/export presence are observed in native order.
/// Mode/profile field meanings and concrete service internals are still unverified.
pub fn intercept(
    state: &mut State,
    input: InterceptInput,
    refcon: u64,
    services: &mut impl Services,
) -> i32 {
    if (input.character == 0 && input.flags == 0 && input.virtual_key == 0)
        || !state.registered
        || state.mode != 0
    {
        return 1;
    }
    let start = services.clock(state);
    let scope = if state.scope_mask & state.enabled_mask != 0 {
        let context = state.scope_context;
        Some(services.enter_scope(state, context))
    } else {
        None
    };
    let token = services.enter_hook(state);
    let result = if state.export_present {
        services.call_hook(state, input, refcon)
    } else {
        0
    };
    if token >= 0 {
        services.leave_hook(state, token);
    }
    let end = services.clock(state);
    state.elapsed += end - start;
    if let Some(token) = scope {
        let context = state.scope_context;
        services.leave_scope(state, context, token);
    }
    result
}
