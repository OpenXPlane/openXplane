//! Concrete main-menu panel key callback: original vtable 0x1426064f0 +0x38.
//! Getter and no-op slots are recovered; fallback action body remains a service.
use crate::ui_input::KeyInput;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// Getter 0x1412e33e0 reads object+0x168 without normalization.
    pub flag168: u8,
    /// Getter 0x1412e33d0 reads object+0x169 without normalization.
    pub flag169: u8,
}
pub trait Services {
    /// Native panel slot +0x170 is 0x140f00310. Its effects are not implemented here.
    fn fallback_action(&mut self, state: &mut State);
}
/// Own callback 0x1412e1de0 specialized to the panel's actual vtable.
/// Slot +0x140 is native constant true; +0x168/+0x178 actions
/// are native no-ops. Child routing/mask acceptance occur in ui_input beforehand.
pub fn dispatch(state: &mut State, input: KeyInput, services: &mut impl Services) -> u8 {
    if input.flags & 8 == 0 {
        return 0;
    }
    match input.virtual_key {
        27 => {
            if state.flag169 == 0 {
                services.fallback_action(state);
            }
            1
        }
        13 | 0xbb => {
            if state.flag168 == 0 && state.flag169 == 0 {
                services.fallback_action(state);
            }
            1
        }
        _ => 0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Cancel(usize);
    impl Services for Cancel {
        fn fallback_action(&mut self, _: &mut State) {
            self.0 += 1;
        }
    }
    #[test]
    fn concrete_panel_ignores_character_and_does_not_treat_all_return_codes_equally() {
        let mut state = State {
            flag168: 255,
            flag169: 0,
        };
        let mut calls = Cancel(0);
        let event = |key, flags| KeyInput {
            character: 13,
            virtual_key: key,
            flags,
        };
        assert_eq!(dispatch(&mut state, event(0xbc, 8), &mut calls), 0);
        assert_eq!(dispatch(&mut state, event(27, 16), &mut calls), 0);
        assert_eq!(dispatch(&mut state, event(13, 8), &mut calls), 1);
        assert_eq!(calls.0, 0);
        assert_eq!(dispatch(&mut state, event(27, 8), &mut calls), 1);
        assert_eq!(calls.0, 1);
        state.flag169 = 255;
        assert_eq!(dispatch(&mut state, event(27, 8), &mut calls), 1);
        assert_eq!(calls.0, 1);
    }
}
