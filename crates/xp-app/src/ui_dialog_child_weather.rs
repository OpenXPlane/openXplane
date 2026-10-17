//! Fourth dialog child1410d75a0. Raw initialization/copying and retained
//! callbacks are recovered; weather/provider/refresh bodies remain services.
use crate::{
    ui_callback_links, ui_callback_list,
    ui_input_construct::{self, Block},
};
use std::collections::BTreeMap;
pub struct Context<'a> {
    pub identity: u64,
    pub temporaries: [u64; 2],
    pub callbacks: &'a mut ui_callback_links::State,
    pub parents: &'a mut BTreeMap<u64, ui_callback_list::List>,
}
pub trait Services: ui_input_construct::Services + ui_callback_list::Services {
    fn weather(&mut self, image: &mut [u8], offset: usize, index: i32, altitude_bits: u32);
    fn altitude(&mut self, image: &mut [u8], parent: u64) -> u32;
    fn selection(&mut self, image: &mut [u8], parent: u64) -> [u8; 32];
    fn refresh(&mut self, image: &mut [u8]);
    fn special(&mut self, image: &mut [u8]);
}
pub struct Initialization {
    pub base_sentinel: Block,
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
fn initial_weather(image: &mut [u8], base: usize, services: &mut impl Services) {
    for (o, n) in [
        (0, 12),
        (0x14, 20),
        (0x2c, 4),
        (0x40, 8),
        (0x48, 0x4c),
        (0xc8, 0xe0),
    ] {
        image[base + o..base + o + n].fill(0);
    }
    for (o, v) in [(0xc, 3), (0x28, 0xc479c000), (0x30, 2), (0x34, 2)] {
        d(image, base + o, v);
    }
    image[base + 0x10] = 1;
    q(image, base + 0x38, 2);
    for o in (0x94..=0xc4).step_by(4) {
        d(image, base + o, 0xbf800000);
    }
    services.weather(image, base, 0, 0);
    d(image, base + 0x28, 0xc479c000);
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
pub fn construct(
    image: &mut [u8],
    first: u64,
    second: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    if image.len() != 0x3e8
        || context.parents.contains_key(&context.identity)
        || context
            .callbacks
            .nodes
            .contains_key(&(context.identity + 0x20))
    {
        return Err("fresh weather child required");
    }
    q(image, 0, 0x142642d00);
    q(image, 8, 0);
    q(image, 16, 0);
    let mut base = ui_input_construct::Services::allocate(services, image, 120)?;
    if base.address == 0 || base.bytes.len() != 120 {
        return Err("successful receiver allocation required");
    }
    q(&mut base.bytes, 0, base.address);
    q(&mut base.bytes, 8, base.address);
    q(image, 8, base.address);
    image[0x18] = 0;
    context
        .parents
        .insert(context.identity, ui_callback_list::List::new(base.address)?);
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
    context.callbacks.adopt_container(context.identity + 0x20)?;
    q(image, 0x38, 0x142674f30);
    d(image, 0x40, 0);
    for (o, v) in [
        (0, 0x142643038),
        (0x20, 0x142643068),
        (0x38, 0x142643080),
        (0x48, 0x4200c388d0000000),
        (0x50, 0x4200c388d0000000),
    ] {
        q(image, o, v);
    }
    initial_weather(image, 0x58, services);
    for o in [0x200, 0x208, 0x210, 0x220, 0x228] {
        q(image, o, 0);
    }
    image[0x218] = 1;
    initial_weather(image, 0x230, services);
    q(image, 0x3d8, first);
    q(image, 0x3e0, second);
    let altitude = services.altitude(image, first);
    services.weather(image, 0x230, 0, altitude);
    let source = image[0x230..0x3d8].to_vec();
    image[0x58..0x200].copy_from_slice(&source);
    let selected = services.selection(image, getq(image, 0x3e0));
    image[0x200..0x220].copy_from_slice(&selected);
    services.refresh(image);
    let targets = [first, getq(image, 0x3e0)];
    for (i, parent) in targets.into_iter().enumerate() {
        context.callbacks.construct(
            context.temporaries[i],
            Some(context.identity + 0x20),
            0,
            0,
            services,
        )?;
        context
            .parents
            .get_mut(&parent)
            .ok_or("missing live parent receiver")?
            .append(context.temporaries[i], context.callbacks, services)?;
        context
            .callbacks
            .destroy(context.temporaries[i], services)?;
    }
    q(
        image,
        0x30,
        context.callbacks.nodes[&(context.identity + 0x20)]
            .peers
            .len() as u64,
    );
    if u32::from_le_bytes(image[0x23c..0x240].try_into().unwrap()) == 7 {
        services.special(image);
    }
    Ok(Initialization {
        base_sentinel: base,
        callback_head: head.address,
    })
}
pub fn construct_factory_child(
    factory: &mut crate::ui_dialog_children::State,
    allocation: u64,
    first: u64,
    second: u64,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<Initialization, &'static str> {
    if allocation != context.identity {
        return Err("factory allocation identity mismatch");
    }
    let mut image = factory
        .blocks
        .get(&allocation)
        .ok_or("missing weather child")?
        .clone();
    let init = construct(&mut image, first, second, context, services)?;
    factory.blocks.insert(allocation, image);
    Ok(init)
}
