//! Complete outer selection-child ctor1410d6fc0. Property/config/refresh/final
//! effects remain services; inline data initializer141a3c750 and empty-tree
//! clear, selection comparison/assignment and retained callbacks are ported.
use crate::{
    ui_callback_links, ui_callback_list,
    ui_input_construct::{self, Block},
    ui_interface_chain,
};
use std::collections::BTreeMap;
pub struct State {
    pub image: Vec<u8>,
    pub active_global: u64,
    /// Shared global142640504, reloaded after the engine gate.
    pub resource_index: u32,
    pub catalog_base: u64,
    pub catalog_values: Vec<u64>,
    pub interfaces: ui_interface_chain::State,
}
pub struct Context<'a> {
    pub identity: u64,
    pub interface: usize,
    pub addresses: &'a [u64],
    pub temporaries: [u64; 2],
    pub callbacks: &'a mut ui_callback_links::State,
    pub parents: &'a mut BTreeMap<u64, ui_callback_list::List>,
}
pub trait Services: ui_input_construct::Services + ui_callback_list::Services {
    fn bind_property(&mut self, state: &mut State, field: u64, enabled: u32, key: &str);
    fn engine(&mut self, state: &mut State) -> u8;
    /// Supply the version-bound table descriptor and mapped resource fields.
    fn resource_memory(&self) -> Result<&crate::ui_payload_move::State, &'static str> {
        Err("resource memory provider required")
    }
    fn resource_present(&mut self, _: &mut State, index: u32) -> Result<bool, &'static str> {
        crate::ui_indexed_records::resource_present(self.resource_memory()?, index)
    }
    fn alternate(&mut self, state: &mut State);
    fn alternate_checked(&mut self, state: &mut State) -> Result<(), &'static str> {
        self.alternate(state);
        Ok(())
    }
    fn selection_value(&mut self, state: &mut State) -> u64;
    fn selected_index(&mut self, state: &mut State, token: u64) -> i32;
    fn changed(&mut self, state: &mut State);
    fn changed_checked(
        &mut self,
        state: &mut State,
        _base_sentinel: &mut Block,
    ) -> Result<(), &'static str> {
        self.changed(state);
        Ok(())
    }
    fn reset(&mut self, state: &mut State, argument: u32);
    fn reset_checked(
        &mut self,
        state: &mut State,
        argument: u32,
        _base_sentinel: &mut Block,
    ) -> Result<(), &'static str> {
        self.reset(state, argument);
        Ok(())
    }
    fn finish(&mut self, state: &mut State);
}
pub struct Initialization {
    pub base_sentinel: Block,
    pub record_sentinel: Block,
    pub callback_head: u64,
}
fn q(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn d(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn getq(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}
fn required(
    image: &[u8],
    bytes: usize,
    services: &mut impl Services,
) -> Result<Block, &'static str> {
    let b = ui_input_construct::Services::allocate(services, image, bytes)?;
    if b.address == 0 || b.bytes.len() != bytes {
        return Err("successful exact allocation required");
    }
    Ok(b)
}
struct View<'a, S> {
    prefix: &'a [u8],
    suffix: &'a [u8],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for View<'_, S> {
    fn allocate(&mut self, raw: &[u8], n: usize) -> Result<Block, &'static str> {
        let mut image = self.prefix.to_vec();
        image.extend_from_slice(raw);
        image.extend_from_slice(self.suffix);
        ui_input_construct::Services::allocate(self.services, &image, n)
    }
}
fn clear_inline_optional(image: &mut [u8]) -> Result<(), &'static str> {
    for off in [0x90, 0x70] {
        if getq(image, off + 24) > 15 {
            return Err("heap optional destruction profile unverified");
        }
        q(image, off + 16, 0);
        q(image, off + 24, 15);
        image[off] = 0;
    }
    Ok(())
}
pub fn construct(
    state: &mut State,
    parent: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    if state.image.len() != 0x2d0
        || context.addresses.len() != state.interfaces.nodes.len()
        || context.addresses.get(context.interface) != Some(&(context.identity + 0x20))
        || context.parents.contains_key(&context.identity)
    {
        return Err("fresh valid selection-child profile required");
    }
    q(&mut state.image, 0, 0x142642d00);
    q(&mut state.image, 8, 0);
    q(&mut state.image, 16, 0);
    let mut base = required(&state.image, 120, services)?;
    for off in [0, 8] {
        q(&mut base.bytes, off, base.address);
    }
    q(&mut state.image, 8, base.address);
    state.image[0x18] = 0;
    context
        .parents
        .insert(context.identity, ui_callback_list::List::new(base.address)?);
    state.interfaces.construct(context.interface)?;
    let older = state.interfaces.nodes[context.interface]
        .older
        .map_or(0, |i| context.addresses[i]);
    q(&mut state.image, 0x20, ui_interface_chain::BASE_TABLE);
    q(&mut state.image, 0x28, 0);
    q(&mut state.image, 0x30, older);
    let head = {
        let (prefix, tail) = state.image.split_at_mut(0x38);
        let (raw, suffix) = tail.split_at_mut(24);
        ui_input_construct::construct_callback(
            raw,
            &mut View {
                prefix,
                suffix,
                services,
            },
        )?
    };
    context.callbacks.adopt_container(context.identity + 0x38)?;
    q(&mut state.image, 0, 0x142643098);
    q(&mut state.image, 0x20, 0x1426430c8);
    q(&mut state.image, 0x38, 0x1426430e0);
    state.interfaces.nodes[context.interface].table = 0x1426430c8;
    for off in [0x50, 0x58] {
        q(&mut state.image, off, 0);
    }
    state.image[0x60] = 0;
    d(&mut state.image, 0x64, 0);
    d(&mut state.image, 0x68, 0);
    state.image[0xb0] = 0;
    state.image[0xb8] = 0;
    d(&mut state.image, 0xc0, 0);
    q(&mut state.image, 0xc8, parent);
    for off in (0xd0..=0x120).step_by(8) {
        q(&mut state.image, off, 0);
    }
    d(&mut state.image, 0x128, 0);
    let tree = crate::ui_record_construct::construct(&mut state.image, 0x130, services)?;
    d(&mut state.image, 0x2c8, 0);
    services.bind_property(
        state,
        context.identity + 0x120,
        1,
        "&m_async_apt_filter_done",
    );
    if services.engine(state) != 0
        && state.active_global == 0
        && services.resource_present(state, state.resource_index)?
    {
        services.alternate_checked(state)?;
    } else {
        let max = state
            .catalog_values
            .len()
            .checked_sub(1)
            .ok_or("nonempty catalog required")?;
        let token = services.selection_value(state);
        let raw = services.selected_index(state, token);
        let index = if raw < 0 { 0 } else { (raw as usize).min(max) };
        let value = *state
            .catalog_values
            .get(index)
            .ok_or("stable valid catalog extent required")?;
        let address = state
            .catalog_base
            .checked_add((index as u64) * 168)
            .ok_or("catalog address overflow")?;
        d(&mut state.image, 0xc0, 0);
        let same = state.image[0xb8] == 0
            && getq(&state.image, 0x50) == address
            && getq(&state.image, 0x58) == value
            && state.image[0x60] == 0
            && f32::from_bits(u32::from_le_bytes(
                state.image[0x64..0x68].try_into().unwrap(),
            )) == 0.0
            && u32::from_le_bytes(state.image[0x68..0x6c].try_into().unwrap()) == 0
            && state.image[0xb0] == 0;
        if !same {
            let mode = state.image[0xb8];
            if mode != 0 && mode != 255 && mode > 4 {
                return Err("native invalid variant jump unverified");
            }
            if mode != 0 {
                state.image[0xb8] = 255;
            }
            q(&mut state.image, 0x50, address);
            q(&mut state.image, 0x58, value);
            state.image[0x60] = 0;
            d(&mut state.image, 0x64, 0);
            d(&mut state.image, 0x68, 0);
            if mode == 0 && state.image[0xb0] != 0 {
                clear_inline_optional(&mut state.image)?;
            }
            state.image[0xb0] = 0;
            if mode != 0 {
                state.image[0xb8] = 0;
            }
            services.changed_checked(state, &mut base)?;
        }
    }
    for (off, temp) in [
        (0xc8, context.temporaries[0]),
        (0xd0, context.temporaries[1]),
    ] {
        let target = getq(&state.image, off);
        if target != 0 {
            context.callbacks.construct(
                temp,
                Some(context.identity + 0x38),
                0x1e1b9,
                0,
                services,
            )?;
            context
                .parents
                .get_mut(&target)
                .ok_or("live parent callback list required")?
                .append(temp, context.callbacks, services)?;
            context.callbacks.destroy(temp, services)?;
            q(
                &mut state.image,
                0x48,
                context.callbacks.nodes[&(context.identity + 0x38)]
                    .peers
                    .len() as u64,
            );
        }
    }
    services.reset_checked(state, 0, &mut base)?;
    services.finish(state);
    Ok(Initialization {
        base_sentinel: base,
        record_sentinel: tree,
        callback_head: head.address,
    })
}

/// Kind1 adapter; internal sentinel ownership remains with the caller and
/// callback backing stays logical. Whole-factory native heap parity is separate.
pub fn construct_factory_child(
    factory: &mut crate::ui_dialog_children::State,
    allocation: u64,
    parent: u64,
    state: &mut State,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    if context.identity != allocation {
        return Err("factory allocation identity mismatch");
    }
    state.image = factory
        .blocks
        .get(&allocation)
        .ok_or("missing factory selection child")?
        .clone();
    let initialized = construct(state, parent, context, services)?;
    factory.blocks.insert(allocation, state.image.clone());
    Ok(initialized)
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Host {
        next: u64,
        entry: u64,
    }
    impl ui_input_construct::Services for Host {
        fn allocate(&mut self, _: &[u8], n: usize) -> Result<Block, &'static str> {
            let p = self.next;
            self.next += 0x1000;
            Ok(Block {
                address: p,
                bytes: vec![0xa5; n],
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
    impl Services for Host {
        fn bind_property(&mut self, _: &mut State, field: u64, enabled: u32, key: &str) {
            assert_eq!(
                (field, enabled, key),
                (0x3120, 1, "&m_async_apt_filter_done")
            );
        }
        fn engine(&mut self, _: &mut State) -> u8 {
            0
        }
        fn resource_present(&mut self, _: &mut State, _: u32) -> Result<bool, &'static str> {
            panic!("engine gate")
        }
        fn alternate(&mut self, _: &mut State) {
            panic!("engine gate")
        }
        fn selection_value(&mut self, _: &mut State) -> u64 {
            17
        }
        fn selected_index(&mut self, _: &mut State, token: u64) -> i32 {
            assert_eq!(token, 17);
            0
        }
        fn changed(&mut self, _: &mut State) {}
        fn reset(&mut self, _: &mut State, arg: u32) {
            assert_eq!(arg, 0);
        }
        fn finish(&mut self, _: &mut State) {}
    }
    impl crate::ui_dialog_child_context::Services for Host {
        fn refresh_context(&mut self, _: &mut [u8; 96]) -> Result<(), &'static str> {
            Ok(())
        }
    }
    impl crate::ui_dialog_child_root::Services for Host {
        fn allocate(&mut self, image: &[u8], _: &[Block], n: usize) -> Result<Block, &'static str> {
            ui_input_construct::Services::allocate(self, image, n)
        }
        fn refresh_nested(
            &mut self,
            _: &mut [u8],
            _: &mut Vec<Block>,
            _: u64,
        ) -> Result<(), &'static str> {
            Ok(())
        }
        fn refresh_root(
            &mut self,
            _: &mut [u8],
            _: &mut Vec<Block>,
            _: u64,
        ) -> Result<(), &'static str> {
            Ok(())
        }
    }
    impl crate::ui_dialog_child_weather::Services for Host {
        fn weather(&mut self, _: &mut [u8], _: usize, _: i32, _: u32) {}
        fn altitude(&mut self, _: &mut [u8], parent: u64) -> u32 {
            assert_eq!(parent, 0x7000);
            0
        }
        fn selection(&mut self, _: &mut [u8], parent: u64) -> [u8; 32] {
            assert_eq!(parent, 0x3000);
            [0; 32]
        }
        fn refresh(&mut self, _: &mut [u8]) {}
        fn special(&mut self, _: &mut [u8]) {
            panic!("default weather mode is3")
        }
    }
    #[test]
    fn constructed_selection_receiver_accepts_context_child_copy() {
        let mut host = Host {
            next: 0x10000,
            entry: 0x5000,
        };
        let mut callbacks = ui_callback_links::State::default();
        let mut parents = BTreeMap::new();
        let mut state = State {
            image: vec![0xcc; 0x2d0],
            active_global: 0,
            resource_index: 0,
            catalog_base: 0x900000,
            catalog_values: vec![42],
            interfaces: ui_interface_chain::State {
                head: None,
                nodes: vec![Default::default()],
            },
        };
        let mut factory = crate::ui_dialog_children::State::new([0xdd; 40]);
        let mut root = crate::ui_dialog_child_root::State::new(vec![0xaa; 0x3e0]);
        factory.blocks.insert(0x7000, root.image.clone());
        crate::ui_dialog_child_root::construct_factory_child(
            &mut factory,
            0x7000,
            1,
            &mut root,
            crate::ui_dialog_child_root::Context {
                identity: 0x7000,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut host,
        )
        .unwrap();
        assert_eq!(factory.blocks[&0x7000], root.image);
        assert!(parents[&0x71e0].entries.is_empty());
        assert!(parents[&0x7268].entries.is_empty());
        assert!(callbacks.nodes[&0x7288].peers.is_empty());
        factory.blocks.insert(0x3000, state.image.clone());
        let init = construct_factory_child(
            &mut factory,
            0x3000,
            0x7000,
            &mut state,
            Context {
                identity: 0x3000,
                interface: 0,
                addresses: &[0x3020],
                temporaries: [0x9000, 0x9100],
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut host,
        )
        .unwrap();
        assert_eq!(parents[&0x3000].sentinel, init.base_sentinel.address);
        assert!(parents[&0x3000].entries.is_empty());
        assert_eq!(parents[&0x7000].entries, vec![0x5000]);
        let mut image = [0xee; 96];
        crate::ui_dialog_child_context::construct(
            &mut image,
            0x3000,
            crate::ui_dialog_child_context::Context {
                identity: 0x4000,
                temporary: 0x9200,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut host,
        )
        .unwrap();
        assert_eq!(parents[&0x3000].entries, vec![0x6000]);
        assert!(parents[&0x4000].entries.is_empty());
        assert_eq!(
            callbacks.nodes[&0x3038]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x5010]
        );
        assert_eq!(
            callbacks.nodes[&0x4020]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x6010]
        );
        assert_eq!(
            callbacks.nodes[&0x5010].kind,
            ui_callback_links::Kind::Wrapper {
                first: 0x1e1b9,
                second: 0
            }
        );
        assert_eq!(
            callbacks.nodes[&0x6010].kind,
            ui_callback_links::Kind::Wrapper {
                first: 0,
                second: 0
            }
        );
        host.entry = 0x500000;
        factory.blocks.insert(0xa000, vec![0xbb; 0x3e8]);
        crate::ui_dialog_child_weather::construct_factory_child(
            &mut factory,
            0xa000,
            0x7000,
            0x3000,
            crate::ui_dialog_child_weather::Context {
                identity: 0xa000,
                temporaries: [0x9300, 0x9400],
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut host,
        )
        .unwrap();
        assert_eq!(parents[&0x7000].entries, vec![0x5000, 0x500000]);
        assert_eq!(parents[&0x3000].entries, vec![0x6000, 0x501000]);
        assert!(parents[&0xa000].entries.is_empty());
        assert_eq!(
            callbacks.nodes[&0xa020]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x500010, 0x501010]
        );
        assert!(!callbacks.nodes.contains_key(&0x9300));
        assert!(!callbacks.nodes.contains_key(&0x9400));
        assert_eq!(
            &factory.blocks[&0xa000][0x58..0x200],
            &factory.blocks[&0xa000][0x230..0x3d8]
        );
    }
}
