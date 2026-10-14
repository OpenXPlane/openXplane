//! Controller close helper 0x14150ce20 on explicit registered-window state.
//! Queries and kind-map stores are recovered; window/renderer effect bodies remain services.
use crate::ui_state::{self, WindowState};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub windows: WindowState,
    pub modes: Vec<i32>,
    pub delete_kinds: Vec<i32>,
    pub flag_11f48: i32,
    pub flag_2a8: u8,
}
impl State {
    fn index(&self, token: u64) -> Result<usize, &'static str> {
        self.windows
            .registered
            .iter()
            .position(|w| w.as_ref().is_some_and(|w| w.token == token))
            .ok_or("unregistered close window")
    }
    fn mode(&self, token: u64) -> Result<i32, &'static str> {
        let index = self.index(token)?;
        self.modes
            .get(index)
            .copied()
            .ok_or("missing registered mode")
    }
    fn visible(&self, token: u64) -> Result<bool, &'static str> {
        let index = self.index(token)?;
        let window = self.windows.registered[index].as_ref().unwrap();
        Ok(ui_state::cumulative_opacity(&window.opacity_root_to_leaf)? > 0.0)
    }
}
pub trait Services {
    fn update(&mut self, state: &mut State, kind: i32, flag: u8);
    fn reset(&mut self, state: &mut State, token: u64, mode: i32, extra: i32);
    fn visibility(&mut self, state: &mut State, token: u64, show: bool, mode: i32, time: f32);
    fn delete(&mut self, state: &mut State, token: u64, flag: u8);
    fn environment(&mut self, state: &mut State) -> u8;
    fn command_once(&mut self, state: &mut State);
    fn refresh(&mut self, state: &mut State);
    fn final_update(&mut self, state: &mut State, kind: i32);
}
/// Registered-only getter profile: protected-kind and observer lists empty, nonnull
/// mapped registered token after initial update. Factory/unsafe lifetime paths are unverified.
pub fn dispatch(
    state: &mut State,
    kind: i32,
    flag: u8,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if state.windows.query(kind)? {
        services.update(state, kind, 0);
        let token = *state
            .windows
            .by_kind
            .get(&kind)
            .filter(|&&t| t != 0)
            .ok_or("native registered close getter requires mapped window")?;
        if state.mode(token)? == 4 {
            services.reset(state, token, 0, -1);
        }
        if state.mode(token)? == 6 {
            services.reset(state, token, 0, -1);
        }
        let show = !state.visible(token)?;
        services.visibility(state, token, show, 0, 0.0);
        if state.delete_kinds.contains(&kind) {
            services.delete(state, token, 1);
            state.windows.by_kind.insert(kind, 0);
        }
        services.update(state, kind, flag);
        if state.flag_11f48 != 0 && services.environment(state) == 0 && state.flag_2a8 != 0 {
            services.command_once(state);
            services.refresh(state);
        }
    }
    services.final_update(state, kind);
    Ok(())
}
