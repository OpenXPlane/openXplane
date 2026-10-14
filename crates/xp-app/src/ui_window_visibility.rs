//! Show 0x1412f97e0 and hide 0x1412d2370 with native registry/membership/alpha stores.
//! One stable manager/implementation and empty global listener list are required.
//! Parent widget effects, activation, notifications and refresh remain services.
use crate::{ui_state::cumulative_opacity, ui_window_order::Registry};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub registry: Registry,
    /// Native manager+0x90 list contains entry identities, including duplicates.
    pub membership: Vec<usize>,
    /// Native entry+0x88 is this manager (true) or null (false) in this profile.
    pub attached_manager: Vec<bool>,
    /// Whether the manager's ancestor chain reaches global 0x145a0def0.
    pub root_manager: bool,
}
impl State {
    fn first(&self, token: u64) -> Result<usize, &'static str> {
        self.registry
            .order
            .iter()
            .copied()
            .find(|&i| {
                self.registry
                    .entries
                    .get(i)
                    .is_some_and(|e| e.token == token)
            })
            .ok_or("visibility requires registered window")
    }
    pub fn visible(&self, token: u64) -> Result<bool, &'static str> {
        Ok(
            cumulative_opacity(&self.registry.entries[self.first(token)?].opacity_root_to_leaf)?
                > 0.0,
        )
    }
    fn alpha(&mut self, index: usize, value: f32) -> Result<(), &'static str> {
        *self
            .registry
            .entries
            .get_mut(index)
            .ok_or("invalid visibility entry")?
            .opacity_root_to_leaf
            .last_mut()
            .ok_or("missing visibility widget")? = value;
        Ok(())
    }
    fn attach(&mut self, index: usize, value: bool) -> Result<(), &'static str> {
        *self
            .attached_manager
            .get_mut(index)
            .ok_or("missing manager attachment")? = value;
        Ok(())
    }
}
pub trait Services {
    /// 0x1412f6250(entry, impl or null); distinct from entry+0x88 attachment.
    fn owner(&mut self, state: &mut State, entry: usize, attached: bool);
    /// 0x1412fc4b0(entry+0x80 object, 1).
    fn prepare_hide(&mut self, state: &mut State, entry: usize);
    /// 0x1412e47a0 on show / 0x1412e4680 on hide, both with flag 1.
    fn activate(&mut self, state: &mut State, entry: usize, show: bool);
    /// 0x1412f77a0(widget, impl on show / null on hide).
    fn reparent(&mut self, state: &mut State, entry: usize, show: bool)
    -> Result<(), &'static str>;
    fn refresh(&mut self, state: &mut State);
    /// 0x1410c90c0(4, 1, token), after the hide body.
    fn event(&mut self, state: &mut State, token: u64);
}
/// The original tests the full show dword, while insertion tests a low byte.
/// Mode/time are not read by this body. Empty listeners and stable impl identity
/// exclude callback-driven manager replacement and unsafe widget lifetimes.
pub fn dispatch(
    state: &mut State,
    token: u64,
    show_word: u32,
    _mode: i32,
    _time: f32,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let initial = state.first(token)?;
    let visible = state.visible(token)?;
    if show_word != 0 {
        if !visible {
            state.attach(initial, true)?;
            state.membership.push(initial);
            services.owner(state, initial, true);
            if state.root_manager {
                services.activate(state, initial, true);
            }
        }
        let entry = state.registry.extract(token)?;
        state.registry.insert(entry, 1)?;
        state.alpha(entry, 1.0)?;
        services.reparent(state, entry, true)?;
        services.refresh(state);
    } else if visible {
        let entry = state.registry.extract(token)?;
        state.registry.insert(entry, 0)?;
        services.prepare_hide(state, entry);
        if state.root_manager {
            services.activate(state, entry, false);
        }
        state.attach(entry, false)?;
        services.owner(state, entry, false);
        if let Some(i) = state.membership.iter().position(|&i| i == entry) {
            state.membership.remove(i);
        }
        state.alpha(entry, 0.0)?;
        services.reparent(state, entry, false)?;
        services.refresh(state);
        services.event(state, token);
    }
    Ok(())
}
/// Original toggle wrapper 0x1412fb9a0 computes !positive cumulative opacity.
pub fn toggle(
    state: &mut State,
    token: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let show = u32::from(!state.visible(token)?);
    dispatch(state, token, show, 0, 0.0, services)
}
