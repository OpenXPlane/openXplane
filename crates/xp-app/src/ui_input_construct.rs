//! Native input-node base constructor 0x1412a70a0 and callback container ctor.
//! Raw addresses are reference identities, not callable host vtables.
#[derive(Clone, Debug)]
pub struct Block {
    pub address: u64,
    pub bytes: Vec<u8>,
}
pub trait Services {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str>;
}
fn q(image: &mut [u8], offset: usize, value: u64) {
    image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
fn d(image: &mut [u8], offset: usize, value: u32) {
    image[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
fn allocate(
    image: &[u8],
    bytes: usize,
    services: &mut impl Services,
) -> Result<Block, &'static str> {
    let block = services.allocate(image, bytes)?;
    if block.address == 0 || block.bytes.len() != bytes {
        return Err("native successful allocation profile required");
    }
    Ok(block)
}
/// Verified inline-name profile (capacity15, length0..15), valid 0x128-byte object.
/// Preserves padding/uninitialized sentinel payloads, source string residual bytes,
/// transient interface stores and allocator snapshot order.
pub fn construct(
    image: &mut [u8],
    name: &mut [u8; 32],
    services: &mut impl Services,
) -> Result<Vec<Block>, &'static str> {
    if image.len() != 0x128 {
        return Err("input node image size mismatch");
    }
    let len = u64::from_le_bytes(name[16..24].try_into().unwrap());
    let cap = u64::from_le_bytes(name[24..32].try_into().unwrap());
    if len > 15 || cap != 15 {
        return Err("only valid inline name profile verified");
    }
    q(image, 0, 0x142674610);
    q(image, 8, 0);
    q(image, 16, 0);
    d(image, 24, 0xfff);
    for (off, value) in [
        (0x20, 0x142674248),
        (0x28, 0x142616f00),
        (0x30, 0x1426742c0),
        (0x38, 0x142674280),
        (0x40, 0x142616f58),
    ] {
        q(image, off, value);
    }
    q(image, 0x48, 0x1426401e0);
    q(image, 0x50, 0);
    q(image, 0x58, 0);
    let mut callback = allocate(image, 40, services)?;
    for off in [0, 8, 16] {
        q(&mut callback.bytes, off, callback.address);
    }
    callback.bytes[24..26].copy_from_slice(&0x101u16.to_le_bytes());
    q(image, 0x50, callback.address);
    for (off, value) in [
        (0, 0x142674670),
        (0x20, 0x142674790),
        (0x28, 0x1426747c8),
        (0x30, 0x142674820),
        (0x38, 0x142674848),
        (0x40, 0x142674868),
        (0x48, 0x1426748a8),
    ] {
        q(image, off, value);
    }
    image[0x60..0x70].copy_from_slice(&name[..16]);
    q(image, 0x70, len);
    q(image, 0x78, 15);
    for off in [0x80, 0x88, 0x90, 0x98] {
        q(image, off, 0);
    }
    let mut first = allocate(image, 24, services)?;
    for off in [0, 8] {
        q(&mut first.bytes, off, first.address);
    }
    q(image, 0x90, first.address);
    for off in [
        0xa0, 0xa8, 0xb0, 0xb8, 0xc0, 0xc8, 0xd0, 0xd8, 0xe0, 0xf4, 0xe8,
    ] {
        q(image, off, 0);
    }
    d(image, 0xf0, 0);
    q(image, 0x100, 0);
    q(image, 0x108, 0);
    let mut second = allocate(image, 40, services)?;
    for off in [0, 8] {
        q(&mut second.bytes, off, second.address);
    }
    q(image, 0x100, second.address);
    q(image, 0x110, 0);
    q(image, 0x118, 0);
    let mut third = allocate(image, 48, services)?;
    for off in [0, 8] {
        q(&mut third.bytes, off, third.address);
    }
    q(image, 0x110, third.address);
    d(image, 0x120, 0);
    name[16..24].fill(0);
    name[24..32].copy_from_slice(&15u64.to_le_bytes());
    name[0] = 0;
    Ok(vec![callback, first, second, third])
}
