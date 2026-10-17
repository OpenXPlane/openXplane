//! First dialog child ctor1410d6a40: raw initialization and fresh receiver/
//! container construction. Two refresh bodies remain explicit services.
use crate::{
    ui_callback_links, ui_callback_list,
    ui_input_construct::{self, Block},
    ui_record_construct,
};
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct State {
    pub image: Vec<u8>,
    pub blocks: Vec<Block>,
}
impl State {
    pub fn new(image: Vec<u8>) -> Self {
        Self {
            image,
            blocks: vec![],
        }
    }
}
pub struct Context<'a> {
    pub identity: u64,
    pub callbacks: &'a mut ui_callback_links::State,
    pub parents: &'a mut BTreeMap<u64, ui_callback_list::List>,
}
pub trait Services {
    fn allocate(
        &mut self,
        image: &[u8],
        blocks: &[Block],
        bytes: usize,
    ) -> Result<Block, &'static str>;
    fn refresh_nested(
        &mut self,
        image: &mut [u8],
        blocks: &mut Vec<Block>,
        identity: u64,
    ) -> Result<(), &'static str>;
    fn refresh_root(
        &mut self,
        image: &mut [u8],
        blocks: &mut Vec<Block>,
        identity: u64,
    ) -> Result<(), &'static str>;
    /// Final refresh may retain callbacks in the receiver lists just constructed.
    /// Legacy effect providers keep their existing context-free entry point.
    fn refresh_root_with_context(
        &mut self,
        image: &mut [u8],
        blocks: &mut Vec<Block>,
        identity: u64,
        _callbacks: &mut ui_callback_links::State,
        _parents: &mut BTreeMap<u64, ui_callback_list::List>,
    ) -> Result<(), &'static str> {
        self.refresh_root(image, blocks, identity)
    }
}
fn q(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn d(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
fn block(
    state: &mut State,
    bytes: usize,
    services: &mut impl Services,
) -> Result<Block, &'static str> {
    let b = services.allocate(&state.image, &state.blocks, bytes)?;
    if b.address == 0 || b.bytes.len() != bytes {
        return Err("successful exact allocation required");
    }
    Ok(b)
}
fn receiver(
    state: &mut State,
    off: usize,
    table: u64,
    identity: u64,
    parents: &mut BTreeMap<u64, ui_callback_list::List>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    q(&mut state.image, off, 0x142642d00);
    q(&mut state.image, off + 8, 0);
    q(&mut state.image, off + 16, 0);
    let mut b = block(state, 120, services)?;
    for o in [0, 8] {
        q(&mut b.bytes, o, b.address);
    }
    q(&mut state.image, off + 8, b.address);
    state.image[off + 0x18] = 0;
    parents.insert(
        identity + off as u64,
        ui_callback_list::List::new(b.address)?,
    );
    state.blocks.push(b);
    q(&mut state.image, off, table);
    Ok(())
}
struct Record<'a, S> {
    blocks: &'a [Block],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for Record<'_, S> {
    fn allocate(&mut self, image: &[u8], n: usize) -> Result<Block, &'static str> {
        self.services.allocate(image, self.blocks, n)
    }
}
struct Callback<'a, S> {
    prefix: &'a [u8],
    suffix: &'a [u8],
    blocks: &'a [Block],
    services: &'a mut S,
}
impl<S: Services> ui_input_construct::Services for Callback<'_, S> {
    fn allocate(&mut self, raw: &[u8], n: usize) -> Result<Block, &'static str> {
        let mut image = self.prefix.to_vec();
        image.extend_from_slice(raw);
        image.extend_from_slice(self.suffix);
        self.services.allocate(&image, self.blocks, n)
    }
}
pub fn construct(
    state: &mut State,
    flag: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if state.image.len() != 0x3e0
        || !state.blocks.is_empty()
        || [0, 0x1e0, 0x268]
            .into_iter()
            .any(|o| context.parents.contains_key(&(context.identity + o)))
        || context
            .callbacks
            .nodes
            .contains_key(&(context.identity + 0x288))
    {
        return Err("fresh valid root child required");
    }
    receiver(
        state,
        0,
        0x142642da8,
        context.identity,
        context.parents,
        services,
    )?;
    let record = ui_record_construct::construct(
        &mut state.image,
        0x20,
        &mut Record {
            blocks: &state.blocks,
            services,
        },
    )?;
    state.blocks.push(record);
    for o in [0x1b8, 0x1c0, 0x1c8, 0x1d0] {
        q(&mut state.image, o, 0);
    }
    state.image[0x1d8] = flag as u8;
    state.image[0x1d9] = 0;
    receiver(
        state,
        0x1e0,
        0x142642d30,
        context.identity,
        context.parents,
        services,
    )?;
    d(&mut state.image, 0x200, 0);
    for off in [0x208, 0x218] {
        q(&mut state.image, off, 0);
        q(&mut state.image, off + 8, 0);
        let mut tree = block(state, 72, services)?;
        for o in [0, 8, 16] {
            q(&mut tree.bytes, o, tree.address);
        }
        tree.bytes[24..26].copy_from_slice(&0x101u16.to_le_bytes());
        q(&mut state.image, off, tree.address);
        state.blocks.push(tree);
    }
    for o in [0x228, 0x230, 0x238] {
        q(&mut state.image, o, 0);
    }
    state.image[0x240..0x250].fill(0);
    q(&mut state.image, 0x250, 0);
    q(&mut state.image, 0x258, 15);
    d(&mut state.image, 0x260, 0);
    receiver(
        state,
        0x268,
        0x142642d00,
        context.identity,
        context.parents,
        services,
    )?;
    let head = {
        let (prefix, tail) = state.image.split_at_mut(0x288);
        let (raw, suffix) = tail.split_at_mut(24);
        ui_input_construct::construct_callback(
            raw,
            &mut Callback {
                prefix,
                suffix,
                blocks: &state.blocks,
                services,
            },
        )?
    };
    state.blocks.push(head);
    context
        .callbacks
        .adopt_container(context.identity + 0x288)?;
    q(&mut state.image, 0x268, 0x142642d60);
    q(&mut state.image, 0x288, 0x142642d90);
    q(&mut state.image, 0x2a0, 0);
    q(&mut state.image, 0x2a8, 0);
    state.image[0x2cc] = 0;
    d(&mut state.image, 0x2d0, 0);
    state.image[0x300] = 0;
    q(&mut state.image, 0x308, 0);
    state.image[0x310..0x330].fill(0);
    d(&mut state.image, 0x330, 0);
    for o in [0x338, 0x340, 0x348] {
        state.image[o] = 0;
    }
    q(&mut state.image, 0x350, context.identity + 0x368);
    q(&mut state.image, 0x358, context.identity + 0x368);
    q(&mut state.image, 0x360, context.identity + 0x388);
    q(&mut state.image, 0x390, 0);
    state.image[0x398] = 0;
    q(&mut state.image, 0x3a0, context.identity + 0x218);
    for o in [0x3a8, 0x3b0, 0x3b8] {
        q(&mut state.image, o, 0);
    }
    services.refresh_nested(
        &mut state.image,
        &mut state.blocks,
        context.identity + 0x268,
    )?;
    state.image[0x3c0] = 0;
    for o in [0x3c8, 0x3d0, 0x3d8] {
        q(&mut state.image, o, 0);
    }
    services.refresh_root_with_context(
        &mut state.image,
        &mut state.blocks,
        context.identity,
        context.callbacks,
        context.parents,
    )?;
    Ok(())
}
/// Kind0 adapter. Internal allocations remain owned in State. Constructor
/// images and logical lists are not executable native ABI memory mirrors.
pub fn construct_factory_child(
    factory: &mut crate::ui_dialog_children::State,
    allocation: u64,
    flag: u64,
    state: &mut State,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if context.identity != allocation {
        return Err("factory allocation identity mismatch");
    }
    state.image = factory
        .blocks
        .get(&allocation)
        .ok_or("missing factory root child")?
        .clone();
    construct(state, flag, context, services)?;
    factory.blocks.insert(allocation, state.image.clone());
    Ok(())
}
