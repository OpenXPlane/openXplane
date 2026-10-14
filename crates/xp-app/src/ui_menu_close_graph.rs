//! Controller close 0x14150ce20 composed with the actual shared window/widget graph.
//! Stable registered getter profile; reset/delete/update/environment effects remain services.
use crate::ui_window_graph::{self, State as Graph};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct State {
    pub graph: Graph,
    pub by_kind: BTreeMap<i32, u64>,
    /// Modes aligned with stable registered entry identities, not current list positions.
    pub modes: Vec<i32>,
    pub delete_kinds: Vec<i32>,
    pub flag_11f48: i32,
    pub flag_2a8: u8,
}
impl State {
    pub fn index(&self, token: u64) -> Result<usize, &'static str> {
        self.graph
            .windows
            .registry
            .order
            .iter()
            .copied()
            .find(|&i| {
                self.graph
                    .windows
                    .registry
                    .entries
                    .get(i)
                    .is_some_and(|e| e.token == token)
            })
            .ok_or("unregistered close graph token")
    }
    pub fn query(&self, kind: i32) -> Result<bool, &'static str> {
        let Some(&token) = self.by_kind.get(&kind).filter(|&&t| t != 0) else {
            return Ok(false);
        };
        if self.index(token).is_err() {
            return Ok(false);
        }
        self.graph.visible(token)
    }
    fn mode(&self, token: u64) -> Result<i32, &'static str> {
        self.modes
            .get(self.index(token)?)
            .copied()
            .ok_or("missing close graph mode")
    }
}
pub trait Services: ui_window_graph::Services {
    fn update(&mut self, state: &mut State, kind: i32, flag: u8);
    fn reset(&mut self, state: &mut State, token: u64, mode: i32, extra: i32);
    fn delete(&mut self, state: &mut State, token: u64, flag: u8);
    fn environment(&mut self, state: &mut State) -> u8;
    fn command_once(&mut self, state: &mut State);
    fn controller_refresh(&mut self, state: &mut State);
    fn final_update(&mut self, state: &mut State, kind: i32);
}
/// Protected-kind/observer lists empty; mapped window remains registered after update.
/// Native null-map factory and unsafe destruction branches are unsupported.
pub fn dispatch(
    state: &mut State,
    kind: i32,
    flag: u8,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if state.query(kind)? {
        services.update(state, kind, 0);
        let token = *state
            .by_kind
            .get(&kind)
            .filter(|&&t| t != 0)
            .ok_or("close graph getter requires mapped registered window")?;
        state.index(token)?;
        if state.mode(token)? == 4 {
            services.reset(state, token, 0, -1);
        }
        if state.mode(token)? == 6 {
            services.reset(state, token, 0, -1);
        }
        ui_window_graph::toggle(&mut state.graph, token, services)?;
        if state.delete_kinds.contains(&kind) {
            services.delete(state, token, 1);
            state.by_kind.insert(kind, 0);
        }
        services.update(state, kind, flag);
        if state.flag_11f48 != 0 && services.environment(state) == 0 && state.flag_2a8 != 0 {
            services.command_once(state);
            services.controller_refresh(state);
        }
    }
    services.final_update(state, kind);
    Ok(())
}
