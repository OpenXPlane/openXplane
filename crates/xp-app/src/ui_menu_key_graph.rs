//! Concrete own/focused key entry -> fallback -> direct -> actual shared graph close.
//! Focused profile has no focused child and a supplied acceptance mask.
use crate::{
    ui_input::{self, KeyInput},
    ui_menu_action as action, ui_menu_action_graph, ui_menu_close_graph as close,
    ui_menu_direct as direct, ui_menu_key as key,
};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub key: key::State,
    pub action: action::State,
    pub panel: direct::State,
    pub controller: close::State,
    pub controller_identity: u64,
}
struct Bridge<'a, S> {
    action: &'a mut action::State,
    panel: &'a mut direct::State,
    controller: &'a mut close::State,
    identity: u64,
    effects: &'a mut S,
    error: Option<&'static str>,
}
impl<S: action::Services + direct::Services + close::Services> key::Services for Bridge<'_, S> {
    fn fallback_action(&mut self, _: &mut key::State) {
        if let Err(e) = ui_menu_action_graph::dispatch(
            self.action,
            self.panel,
            self.controller,
            self.identity,
            self.effects,
        ) {
            self.error = Some(e);
        }
    }
}
pub fn dispatch(
    state: &mut State,
    input: KeyInput,
    effects: &mut (impl action::Services + direct::Services + close::Services),
) -> Result<u8, &'static str> {
    let mut bridge = Bridge {
        action: &mut state.action,
        panel: &mut state.panel,
        controller: &mut state.controller,
        identity: state.controller_identity,
        effects,
        error: None,
    };
    let result = key::dispatch(&mut state.key, input, &mut bridge);
    if let Some(error) = bridge.error {
        return Err(error);
    }
    Ok(result)
}
/// Original focused dispatcher with absent child; live focus selection is external.
pub fn dispatch_focused(
    state: &mut State,
    focused: bool,
    mask: u32,
    input: KeyInput,
    effects: &mut (impl action::Services + direct::Services + close::Services),
) -> Result<u8, &'static str> {
    let mut error = None;
    let consumed = ui_input::dispatch_focused(
        focused,
        || mask,
        input,
        None,
        |input| match dispatch(state, input, effects) {
            Ok(result) => result,
            Err(e) => {
                error = Some(e);
                0
            }
        },
    )?;
    if let Some(error) = error {
        return Err(error);
    }
    Ok(consumed)
}
