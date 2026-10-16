//! Child constructor1410d7490, including retained parent callback registration.
//! Refresh1411396d0 remains a service. Callback-tree backing is represented by
//! logical links; the raw object fields and base sentinel retain byte images.
use crate::{
    ui_callback_links, ui_callback_list,
    ui_input_construct::{self, Block},
};
use std::collections::BTreeMap;
pub struct Context<'a> {
    pub identity: u64,
    pub temporary: u64,
    pub callbacks: &'a mut ui_callback_links::State,
    pub parents: &'a mut BTreeMap<u64, ui_callback_list::List>,
}
pub trait Services: ui_input_construct::Services + ui_callback_list::Services {
    fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str>;
}
pub struct Initialization {
    pub base_sentinel: Block,
    /// Reference identity only: callback-tree backing is managed by the link model.
    pub callback_head: u64,
}
fn q(image: &mut [u8], off: usize, value: u64) {
    image[off..off + 8].copy_from_slice(&value.to_le_bytes());
}
struct View<'a, S> {
    prefix: &'a [u8],
    suffix: &'a [u8],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for View<'_, S> {
    fn allocate(&mut self, callback: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let mut image = self.prefix.to_vec();
        image.extend_from_slice(callback);
        image.extend_from_slice(self.suffix);
        ui_input_construct::Services::allocate(self.services, &image, bytes)
    }
}
pub fn construct(
    image: &mut [u8; 96],
    parent: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    let callback = context
        .identity
        .checked_add(0x20)
        .ok_or("callback identity overflow")?;
    if context.callbacks.nodes.contains_key(&callback) {
        return Err("fresh child callback required");
    }
    q(image, 0, 0x142642d00);
    q(image, 8, 0);
    q(image, 16, 0);
    let mut base = ui_input_construct::Services::allocate(services, image, 120)?;
    if base.address == 0 || base.bytes.len() != 120 {
        return Err("successful base allocation required");
    }
    q(&mut base.bytes, 0, base.address);
    q(&mut base.bytes, 8, base.address);
    q(image, 8, base.address);
    image[0x18] = 0;
    let head = {
        let (prefix, tail) = image.split_at_mut(0x20);
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
    context.callbacks.adopt_container(callback)?;
    q(image, 0, 0x142642ff0);
    q(image, 0x20, 0x142643020);
    image[0x38..0x3c].copy_from_slice(&80u32.to_le_bytes());
    image[0x3c..0x40].copy_from_slice(&12f32.to_bits().to_le_bytes());
    q(image, 0x40, 0);
    q(image, 0x48, 0);
    image[0x50] = 1;
    q(image, 0x58, parent);
    services.refresh_context(image)?;
    // Native reloads parent+58 after refresh. A rebind must target that parent.
    let parent = u64::from_le_bytes(image[0x58..0x60].try_into().unwrap());
    context
        .callbacks
        .construct(context.temporary, Some(callback), 0, 0, services)?;
    context
        .parents
        .get_mut(&parent)
        .ok_or("missing live parent callback list")?
        .append(context.temporary, context.callbacks, services)?;
    context.callbacks.destroy(context.temporary, services)?;
    q(
        image,
        0x30,
        context.callbacks.nodes[&callback].peers.len() as u64,
    );
    Ok(Initialization {
        base_sentinel: base,
        callback_head: head.address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Heap {
        next: u64,
    }
    impl ui_input_construct::Services for Heap {
        fn allocate(&mut self, _: &[u8], bytes: usize) -> Result<Block, &'static str> {
            let p = self.next;
            self.next += 0x1000;
            Ok(Block {
                address: p,
                bytes: vec![0xa5; bytes],
            })
        }
    }
    impl ui_callback_links::Services for Heap {
        fn allocate(&mut self, _: usize) {}
        fn free(&mut self, _: usize) {}
    }
    impl ui_callback_list::Services for Heap {
        fn allocate_entry(&mut self, _: usize) -> Result<u64, &'static str> {
            Ok(0x5000)
        }
    }
    impl Services for Heap {
        fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
            q(image, 0x58, 2);
            image[0x50] = 255;
            Ok(())
        }
    }
    #[test]
    fn refresh_parent_reload_retains_copy_and_releases_temporary() {
        let mut parents = BTreeMap::from([
            (1, ui_callback_list::List::new(10).unwrap()),
            (2, ui_callback_list::List::new(20).unwrap()),
        ]);
        let mut callbacks = ui_callback_links::State::default();
        let mut image = [0xcc; 96];
        let mut heap = Heap { next: 0x10000 };
        construct(
            &mut image,
            1,
            Context {
                identity: 0x3000,
                temporary: 0x6000,
                callbacks: &mut callbacks,
                parents: &mut parents,
            },
            &mut heap,
        )
        .unwrap();
        assert!(parents[&1].entries.is_empty());
        assert_eq!(parents[&2].entries, vec![0x5000]);
        assert_eq!(image[0x50], 255);
        assert!(!callbacks.nodes.contains_key(&0x6000));
        assert_eq!(
            callbacks.nodes[&0x3020]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x5010]
        );
    }
}

/// Adapter for kind2 of ui_dialog_children::Services::construct. Internal base
/// ownership is returned to the caller; callback-tree backing remains logical.
/// This does not certify whole native heap images for a composed factory run.
pub fn construct_factory_child(
    factory: &mut crate::ui_dialog_children::State,
    allocation: u64,
    parent: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    if context.identity != allocation {
        return Err("factory allocation identity mismatch");
    }
    let mut image: [u8; 96] = factory
        .blocks
        .get(&allocation)
        .ok_or("missing factory child allocation")?
        .as_slice()
        .try_into()
        .map_err(|_| "context child allocation size mismatch")?;
    let result = construct(&mut image, parent, context, services)?;
    factory.blocks.insert(allocation, image.to_vec());
    Ok(result)
}

#[cfg(test)]
mod factory_integration {
    use super::*;
    struct Inner {
        next: u64,
        entry: u64,
    }
    impl ui_input_construct::Services for Inner {
        fn allocate(&mut self, _: &[u8], n: usize) -> Result<Block, &'static str> {
            let p = self.next;
            self.next += 0x1000;
            Ok(Block {
                address: p,
                bytes: vec![0xa5; n],
            })
        }
    }
    impl ui_callback_links::Services for Inner {
        fn allocate(&mut self, _: usize) {}
        fn free(&mut self, _: usize) {}
    }
    impl ui_callback_list::Services for Inner {
        fn allocate_entry(&mut self, _: usize) -> Result<u64, &'static str> {
            let p = self.entry;
            self.entry += 0x1000;
            Ok(p)
        }
    }
    impl Services for Inner {
        fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
            image[0x50] = 255;
            Ok(())
        }
    }
    struct Host {
        next: u64,
        inner: Inner,
        callbacks: ui_callback_links::State,
        parents: BTreeMap<u64, ui_callback_list::List>,
        initialized: Vec<Initialization>,
    }
    impl crate::ui_dialog_children::Services for Host {
        fn allocate(
            &mut self,
            _: &mut crate::ui_dialog_children::State,
            n: usize,
        ) -> Result<Option<Block>, &'static str> {
            let p = self.next;
            self.next += 0x1000;
            Ok(Some(Block {
                address: p,
                bytes: vec![0xcc; n],
            }))
        }
        fn construct(
            &mut self,
            state: &mut crate::ui_dialog_children::State,
            kind: u8,
            p: u64,
            args: &[u64],
        ) -> Result<u64, &'static str> {
            match kind {
                1 => {
                    self.parents
                        .insert(p, ui_callback_list::List::new(p + 0x8000)?);
                }
                2 => {
                    let initialized = construct_factory_child(
                        state,
                        p,
                        args[0],
                        Context {
                            identity: p,
                            temporary: 0x9000,
                            callbacks: &mut self.callbacks,
                            parents: &mut self.parents,
                        },
                        &mut self.inner,
                    )?;
                    self.initialized.push(initialized);
                }
                _ => {}
            }
            Ok(p)
        }
        fn resource(
            &mut self,
            _: &mut crate::ui_dialog_children::State,
            _: u32,
        ) -> Result<u64, &'static str> {
            Ok(0)
        }
        fn refresh(
            &mut self,
            _: &mut crate::ui_dialog_children::State,
            _: u64,
        ) -> Result<(), &'static str> {
            Ok(())
        }
    }
    #[test]
    fn outer_factory_dispatches_kind2_through_recovered_constructor_and_receiver() {
        let mut host = Host {
            next: 0x100000,
            inner: Inner {
                next: 0x10000,
                entry: 0x5000,
            },
            callbacks: Default::default(),
            parents: BTreeMap::new(),
            initialized: vec![],
        };
        let mut factory = crate::ui_dialog_children::State::new([0xdd; 40]);
        factory.construct(&mut host).unwrap();
        let child = u64::from_le_bytes(factory.bundle[16..24].try_into().unwrap());
        let parent = u64::from_le_bytes(factory.bundle[8..16].try_into().unwrap());
        assert_eq!(factory.blocks[&child][0x50], 255);
        assert_eq!(host.parents[&parent].entries, vec![0x5000]);
        assert_eq!(
            host.callbacks.nodes[&(child + 32)]
                .peers
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            vec![0x5010]
        );
        assert_eq!(host.initialized.len(), 1);
        assert!(!host.callbacks.nodes.contains_key(&0x9000));
    }
}
