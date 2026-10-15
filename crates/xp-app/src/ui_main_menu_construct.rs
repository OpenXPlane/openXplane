//! Concrete Main Menu constructor prefix 0x140ecb340..0x140ecb592.
//! Stops before singleton dialogs, layout and resources. Tables are opaque identities.
use crate::{
    ui_input_construct::{self, Block},
    ui_interface_chain,
    ui_menu_member_construct::{self, Registry},
    ui_panel_construct,
};

pub trait Services: ui_input_construct::Services {
    fn copied(&mut self, bytes: usize);
    fn free(&mut self, block: Block);
}
pub struct Context<'a> {
    pub identity: u64,
    pub parent: u64,
    pub registry: &'a mut Registry,
    pub chain: &'a mut ui_interface_chain::State,
    /// Stable addresses corresponding to chain.nodes; includes this embedded interface.
    pub addresses: &'a [u64],
    pub interface: usize,
}
fn q(image: &mut [u8], offset: usize, value: u64) {
    image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
struct Extended<'a, S> {
    tail: &'a [u8],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for Extended<'_, S> {
    fn allocate(&mut self, prefix: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let mut full = prefix.to_vec();
        full.extend_from_slice(self.tail);
        self.services.allocate(&full, bytes)
    }
}
struct Frozen<'a, S> {
    image: &'a [u8],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for Frozen<'_, S> {
    fn allocate(&mut self, _: &[u8], bytes: usize) -> Result<Block, &'static str> {
        self.services.allocate(self.image, bytes)
    }
}
impl<S: Services> ui_menu_member_construct::Services for Frozen<'_, S> {
    fn allocate(&mut self, bytes: usize) -> Result<Block, &'static str> {
        self.services.allocate(self.image, bytes)
    }
    fn copied(&mut self, bytes: usize) {
        self.services.copied(bytes);
    }
    fn free(&mut self, block: Block) {
        self.services.free(block);
    }
}
fn inline(text: &[u8]) -> [u8; 32] {
    let mut s = [0; 32];
    s[..text.len()].copy_from_slice(text);
    s[16..24].copy_from_slice(&(text.len() as u64).to_le_bytes());
    s[24..32].copy_from_slice(&15u64.to_le_bytes());
    s
}
fn required(
    image: &[u8],
    bytes: usize,
    services: &mut impl Services,
) -> Result<Block, &'static str> {
    let b = services.allocate(image, bytes)?;
    if b.address == 0 || b.bytes.len() != bytes {
        return Err("successful allocation required");
    }
    Ok(b)
}
/// `localized` is the controlled inline result of lookup `/_lookup_12_/help/main/`.
/// This function returns a prefix snapshot, not a fully constructed/usable menu.
/// Valid nonaliasing chain identities and small parent registry are required.
pub fn construct_prefix(
    image: &mut [u8],
    localized: [u8; 32],
    flag: u8,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Vec<Block>, &'static str> {
    if image.len() != 0x300
        || context.addresses.len() != context.chain.nodes.len()
        || context.addresses.get(context.interface) != Some(&(context.identity + 0x188))
        || context
            .chain
            .head
            .is_some_and(|i| i >= context.chain.nodes.len())
    {
        return Err("invalid menu prefix profile");
    }
    let mut strings = [inline(b""), inline(b"Main Menu"), localized];
    let mut blocks = {
        let (prefix, tail) = image.split_at_mut(0x170);
        ui_panel_construct::construct(
            prefix,
            &mut strings,
            [0, 0, 1],
            &mut Extended { tail, services },
        )?
    };
    q(image, 0x170, 0x1426061e0);
    let mut callback = services.allocate(image, 32)?;
    if callback.address != 0 {
        if callback.bytes.len() != 32 {
            return Err("invalid callback allocation");
        }
        let sentinel = ui_input_construct::construct_callback(
            &mut callback.bytes[..24],
            &mut Frozen { image, services },
        )?;
        q(&mut callback.bytes, 0, 0x14260ca58);
        q(&mut callback.bytes, 24, context.identity + 0x170);
        blocks.push(sentinel);
        q(image, 0x180, callback.address);
        blocks.push(callback);
    } else {
        q(image, 0x180, 0);
    }
    context.chain.construct(context.interface)?;
    let older = context.chain.nodes[context.interface]
        .older
        .map_or(0, |i| context.addresses[i]);
    q(image, 0x188, ui_interface_chain::BASE_TABLE);
    q(image, 0x190, 0);
    q(image, 0x198, older);
    for (off, value) in [
        (0, 0x1426064f0),
        (0x20, 0x142606678),
        (0x28, 0x1426066b0),
        (0x30, 0x142606708),
        (0x38, 0x142606730),
        (0x40, 0x142606750),
        (0x48, 0x142606790),
        (0x170, 0x1426067a8),
        (0x188, 0x1426067c8),
        (0x1a0, 0x1426067e0),
    ] {
        q(image, off, value);
    }
    context.chain.nodes[context.interface].table = 0x1426067c8;
    q(image, 0x1a8, context.parent);
    for (off, variant) in [(0x1b0, false), (0x208, false), (0x260, true)] {
        ui_menu_member_construct::initialize(
            &mut image[off..off + 0x58],
            context.identity + off as u64,
            context.parent,
            variant,
        )?;
        if context.parent != 0 {
            context.registry.append(
                context.identity + off as u64 + 0x10,
                &mut Frozen { image, services },
            )?;
        }
    }
    q(image, 0x2b8, 0);
    q(image, 0x2c0, 0);
    image[0x2c8] = flag;
    image[0x2c9] = 0;
    q(image, 0x2d0, 0);
    q(image, 0x2d8, 0);
    let mut tree = required(image, 32, services)?;
    for off in [0, 8, 16] {
        q(&mut tree.bytes, off, tree.address);
    }
    tree.bytes[24..26].copy_from_slice(&0x101u16.to_le_bytes());
    q(image, 0x2d0, tree.address);
    blocks.push(tree);
    q(image, 0x2e0, u64::MAX);
    image[0x2e8..0x2ec].copy_from_slice(&1f32.to_bits().to_le_bytes());
    Ok(blocks)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Heap {
        next: u64,
        calls: usize,
        null_callback: bool,
    }
    impl ui_input_construct::Services for Heap {
        fn allocate(&mut self, _: &[u8], bytes: usize) -> Result<Block, &'static str> {
            self.calls += 1;
            if self.null_callback && self.calls == 5 {
                return Ok(Block {
                    address: 0,
                    bytes: vec![],
                });
            }
            self.next += 0x100;
            Ok(Block {
                address: self.next,
                bytes: vec![0xa5; bytes],
            })
        }
    }
    impl Services for Heap {
        fn copied(&mut self, _: usize) {}
        fn free(&mut self, _: Block) {}
    }
    #[test]
    fn prefix_links_existing_interface_and_registers_all_members_with_optional_callback() {
        for null_callback in [false, true] {
            let mut heap = Heap {
                next: 0x10000,
                calls: 0,
                null_callback,
            };
            let mut image = vec![0xcc; 0x300];
            let mut registry = Registry::default();
            let mut chain = ui_interface_chain::State {
                head: Some(0),
                nodes: vec![
                    ui_interface_chain::Node {
                        table: 77,
                        ..Default::default()
                    },
                    Default::default(),
                ],
            };
            let blocks = construct_prefix(
                &mut image,
                inline(b"Help"),
                255,
                Context {
                    identity: 0x2000,
                    parent: 0x3000,
                    registry: &mut registry,
                    chain: &mut chain,
                    addresses: &[0x4000, 0x2188],
                    interface: 1,
                },
                &mut heap,
            )
            .unwrap();
            assert_eq!(chain.head, Some(1));
            assert_eq!(chain.nodes[0].newer, Some(1));
            assert_eq!(chain.nodes[0].table, 77);
            assert_eq!(registry.entries, vec![0x21c0, 0x2218, 0x2270]);
            assert_eq!(&image[0x60..0x69], b"Main Menu");
            assert_eq!((image[0x2c8], image[0x2c9]), (255, 0));
            let pointer = u64::from_le_bytes(image[0x180..0x188].try_into().unwrap());
            assert_eq!(pointer == 0, null_callback);
            assert_eq!(blocks.len(), if null_callback { 5 } else { 7 });
            let key = ui_panel_construct::key_state(&image).unwrap();
            assert_eq!((key.flag168, key.flag169), (0, 0));
        }
    }
}
