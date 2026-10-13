//! Native keyboard entry gates joining focused UI input and the binding-vector loop.
use crate::ui_input::KeyInput;
use xp_dataref::key_event::{Action, Entry, route};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InterceptInput {
    pub character: u8,
    pub virtual_key: u8,
    pub flags: u32,
    pub phase: u32,
}

/// Native service boundaries. The capture query and interceptor implementations
/// still require recovery; focused input can use `ui_input::dispatch_focused`.
pub trait Services {
    fn has_visible_capture(&mut self) -> bool;
    fn intercept(&mut self, input: InterceptInput) -> i32;
    fn focused(&mut self, input: KeyInput) -> u8;
    /// Read after callbacks. Exact mode 1000 or any nonzero block byte gates commands.
    fn command_gate(&mut self) -> (i32, u8);
}

/// Complete routing order of 0x140c627a0 with inert command callback bodies.
/// Signed characters >127 become 0; negative characters survive for focused UI,
/// but are truncated to a byte at interceptor boundaries. Virtual keys remain full
/// words for UI/bindings and are truncated only for the interceptor.
/// Begin/End requests use the previously recovered stateful binding loop.
pub fn dispatch(
    entries: &mut [Entry],
    suppress_release: &mut i32,
    input: KeyInput,
    services: &mut impl Services,
) -> Vec<Action> {
    let input = KeyInput {
        character: if input.character > 127 {
            0
        } else {
            input.character
        },
        ..input
    };
    let intercept = |phase| InterceptInput {
        character: input.character as u8,
        virtual_key: input.virtual_key as u8,
        flags: input.flags,
        phase,
    };
    if services.has_visible_capture() {
        if services.focused(input) != 0 {
            return Vec::new();
        }
    } else {
        if services.intercept(intercept(0)) == 0 || services.focused(input) != 0 {
            return Vec::new();
        }
        if services.intercept(intercept(2)) == 0 {
            return Vec::new();
        }
    }
    let (mode, block) = services.command_gate();
    if mode == 1000 || input.virtual_key == 0 || block != 0 {
        return Vec::new();
    }
    route(entries, input.virtual_key, input.flags, suppress_release)
}
