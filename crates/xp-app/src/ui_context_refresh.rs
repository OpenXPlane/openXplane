//! Whole child refresh1411396d0: context selection, CLI overrides and listeners.
//! Engine/context/option parsing/conversion/notification bodies remain services.
use crate::{
    ui_input_construct::{self, Block},
    ui_native_string::{self, NativeString},
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Zulu,
    Day,
}
#[derive(Clone, Debug)]
pub struct State {
    pub image: [u8; 96],
    pub active_global: u64,
    /// Shared global142640504, read after engine gating.
    pub resource_index: u32,
    pub next: BTreeMap<u64, u64>,
}
pub trait Services {
    fn engine(&mut self, state: &mut State) -> u8;
    fn resource_present(&mut self, state: &mut State, index: u32) -> bool;
    fn alternate(&mut self, state: &mut State) -> [u8; 32];
    fn manager(&mut self, state: &mut State) -> u64;
    fn default_context(&mut self, state: &mut State, manager: u64) -> [u8; 32];
    fn has(&mut self, state: &mut State, key: Key) -> u8;
    fn parent_context(&mut self, state: &mut State, parent: u64) -> [u8; 16];
    fn context_day(&mut self, state: &mut State, payload: [u8; 32], parent: [u8; 16]) -> i32;
    fn option(&mut self, state: &mut State, key: Key) -> Result<NativeString, &'static str>;
    fn parse(&mut self, state: &mut State, key: Key, text: &NativeString) -> f64;
    fn release(&mut self, state: &mut State, block: Block) -> Result<(), &'static str>;
    fn context_hour(&mut self, state: &mut State, payload: [u8; 32], parent: [u8; 16]) -> u32;
    fn notify(&mut self, state: &mut State, wrapper: u64);
}
fn q(image: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(image[off..off + 8].try_into().unwrap())
}
fn payload(state: &State) -> [u8; 32] {
    state.image[0x38..0x58].try_into().unwrap()
}
struct Release<'a, S> {
    state: &'a mut State,
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for Release<'_, S> {
    fn allocate(&mut self, _: &[u8], _: usize) -> Result<Block, &'static str> {
        Err("refresh string consumption does not allocate")
    }
}
impl<S: Services> ui_native_string::Services for Release<'_, S> {
    fn release(&mut self, _: &[u8], block: Block) -> Result<(), &'static str> {
        self.services.release(self.state, block)
    }
}
fn consume(
    text: &mut NativeString,
    state: &mut State,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let snapshot = state.image;
    text.consume(&snapshot, &mut Release { state, services })
}
fn narrow(value: f64) -> f32 {
    // CVTSD2SS quiets NaNs while retaining the representable payload/sign.
    let bits = value.to_bits();
    if bits & 0x7ff0000000000000 == 0x7ff0000000000000 && bits & 0xfffffffffffff != 0 {
        f32::from_bits(
            ((bits >> 32) as u32 & 0x80000000) | 0x7fc00000 | ((bits >> 29) as u32 & 0x7fffff),
        )
    } else {
        value as f32
    }
}
fn hours(value: f64) -> f32 {
    let value = narrow(value);
    let clamped = if 0.0 > value {
        0.0
    } else if 86400.0 < value {
        86400.0
    } else {
        value
    };
    let minutes = clamped / 60.0;
    minutes / 60.0
}
fn day(value: f64) -> i32 {
    let value = narrow(value);
    if !value.is_finite() || !(0.0..2147483648.0).contains(&value) {
        0
    } else {
        (value as i32).min(366)
    }
}
pub fn refresh(state: &mut State, services: &mut impl Services) -> Result<(), &'static str> {
    let alternate = services.engine(state) != 0
        && state.active_global == 0
        && services.resource_present(state, state.resource_index);
    let initial = if alternate {
        services.alternate(state)
    } else {
        let manager = services.manager(state);
        services.default_context(state, manager)
    };
    state.image[0x38..0x58].copy_from_slice(&initial);
    if !alternate {
        if services.has(state, Key::Zulu) != 0 {
            let parent = q(&state.image, 0x58);
            let context = services.parent_context(state, parent);
            let code = services.context_day(state, payload(state), context);
            let mut text = services.option(state, Key::Zulu)?;
            text.validate()?;
            let value = hours(services.parse(state, Key::Zulu, &text));
            state.image[0x38..0x3c].copy_from_slice(&code.to_le_bytes());
            state.image[0x3c..0x40].copy_from_slice(&value.to_bits().to_le_bytes());
            state.image[0x50] = 2;
            consume(&mut text, state, services)?;
        }
        if services.has(state, Key::Day) != 0 {
            let mut text = services.option(state, Key::Day)?;
            text.validate()?;
            let value = day(services.parse(state, Key::Day, &text));
            consume(&mut text, state, services)?;
            if state.image[0x50] == 1 {
                state.image[0x38..0x3c].copy_from_slice(&value.to_le_bytes());
            } else {
                let parent = q(&state.image, 0x58);
                let context = services.parent_context(state, parent);
                let hour = services.context_hour(state, payload(state), context);
                state.image[0x38..0x3c].copy_from_slice(&value.to_le_bytes());
                state.image[0x3c..0x40].copy_from_slice(&hour.to_le_bytes());
                state.image[0x40..0x50].fill(0);
                state.image[0x50] = 1;
            }
        }
    }
    state.image[0x18] = 1;
    let head = q(&state.image, 8);
    let mut cursor = *state.next.get(&head).ok_or("missing listener head")?;
    let mut visits = 0;
    while cursor != q(&state.image, 8) {
        visits += 1;
        if visits > 100000 {
            return Err("unverified nonterminating listener traversal");
        }
        let current = cursor;
        services.notify(
            state,
            current.checked_add(16).ok_or("listener wrapper overflow")?,
        );
        cursor = *state
            .next
            .get(&current)
            .ok_or("missing retained listener link")?;
    }
    state.image[0x18] = 0;
    Ok(())
}
/// Bridge for child-constructor refresh services; preserves runtime global/list
/// changes and partial state on errors. This compatibility entry assumes index0;
/// use refresh_image_with_index when carrying shared caller state. No native
/// callback-tree layout is implied.
pub fn refresh_image(
    image: &mut [u8; 96],
    active_global: &mut u64,
    next: &mut BTreeMap<u64, u64>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    refresh_image_with_index(image, active_global, &mut 0, next, services)
}
/// Bridge carrying the shared resource index between child refresh callers.
pub fn refresh_image_with_index(
    image: &mut [u8; 96],
    active_global: &mut u64,
    resource_index: &mut u32,
    next: &mut BTreeMap<u64, u64>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let mut state = State {
        image: *image,
        active_global: *active_global,
        resource_index: *resource_index,
        next: std::mem::take(next),
    };
    let result = refresh(&mut state, services);
    *image = state.image;
    *active_global = state.active_global;
    *resource_index = state.resource_index;
    *next = state.next;
    result
}

#[cfg(test)]
mod integration {
    use super::*;
    use crate::{ui_callback_links, ui_callback_list, ui_dialog_child_context};
    struct Host {
        next: u64,
        entry: u64,
        global: u64,
        links: BTreeMap<u64, u64>,
    }
    impl Services for Host {
        fn engine(&mut self, _: &mut State) -> u8 {
            0
        }
        fn resource_present(&mut self, _: &mut State, _: u32) -> bool {
            panic!("engine gate skips resource")
        }
        fn alternate(&mut self, _: &mut State) -> [u8; 32] {
            panic!("engine gate skips alternate")
        }
        fn manager(&mut self, _: &mut State) -> u64 {
            17
        }
        fn default_context(&mut self, _: &mut State, m: u64) -> [u8; 32] {
            assert_eq!(m, 17);
            let mut p = [0; 32];
            p[24] = 1;
            p
        }
        fn has(&mut self, _: &mut State, key: Key) -> u8 {
            if key == Key::Day { 255 } else { 0 }
        }
        fn parent_context(&mut self, _: &mut State, _: u64) -> [u8; 16] {
            panic!("mode1 retains local day")
        }
        fn context_day(&mut self, _: &mut State, _: [u8; 32], _: [u8; 16]) -> i32 {
            panic!("no Zulu override")
        }
        fn option(&mut self, _: &mut State, key: Key) -> Result<NativeString, &'static str> {
            assert_eq!(key, Key::Day);
            let mut raw = [0; 32];
            raw[..5].copy_from_slice(b"366.9");
            raw[16..24].copy_from_slice(&5u64.to_le_bytes());
            raw[24..].copy_from_slice(&15u64.to_le_bytes());
            NativeString::inline(raw)
        }
        fn parse(&mut self, _: &mut State, key: Key, _: &NativeString) -> f64 {
            assert_eq!(key, Key::Day);
            366.9
        }
        fn release(&mut self, _: &mut State, _: Block) -> Result<(), &'static str> {
            panic!("inline option does not allocate")
        }
        fn context_hour(&mut self, _: &mut State, _: [u8; 32], _: [u8; 16]) -> u32 {
            panic!("mode1 retains hour")
        }
        fn notify(&mut self, _: &mut State, _: u64) {
            panic!("new child has no listeners")
        }
    }
    impl ui_input_construct::Services for Host {
        fn allocate(&mut self, _: &[u8], bytes: usize) -> Result<Block, &'static str> {
            let p = self.next;
            self.next += 0x1000;
            if bytes == 120 {
                self.links.insert(p, p);
            }
            Ok(Block {
                address: p,
                bytes: vec![0xa5; bytes],
            })
        }
    }
    impl ui_callback_links::Services for Host {
        fn allocate(&mut self, _: usize) {}
        fn free(&mut self, _: usize) {}
    }
    impl ui_callback_list::Services for Host {
        fn allocate_entry(&mut self, _: usize) -> Result<u64, &'static str> {
            let p = self.entry;
            self.entry += 0x1000;
            Ok(p)
        }
    }
    impl ui_dialog_child_context::Services for Host {
        fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
            let mut global = self.global;
            let mut links = std::mem::take(&mut self.links);
            let result = refresh_image(image, &mut global, &mut links, self);
            self.global = global;
            self.links = links;
            result
        }
    }
    #[test]
    fn child_constructor_executes_recovered_refresh_before_retaining_parent_callback() {
        let mut host = Host {
            next: 0x10000,
            entry: 0x5000,
            global: 7,
            links: BTreeMap::new(),
        };
        let mut callbacks = ui_callback_links::State::default();
        let mut parents = BTreeMap::from([(0x7000, ui_callback_list::List::new(0x8000).unwrap())]);
        let mut image = [0xcc; 96];
        ui_dialog_child_context::construct(
            &mut image,
            0x7000,
            ui_dialog_child_context::Context {
                identity: 0x3000,
                temporary: 0x6000,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut host,
        )
        .unwrap();
        assert_eq!(
            i32::from_le_bytes(image[0x38..0x3c].try_into().unwrap()),
            366
        );
        assert_eq!(image[0x50], 1);
        assert_eq!(image[0x18], 0);
        assert_eq!(parents[&0x7000].entries, vec![0x5000]);
        assert_eq!(
            callbacks.nodes[&0x3020]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x5010]
        );
        assert!(!callbacks.nodes.contains_key(&0x6000));
    }
}
