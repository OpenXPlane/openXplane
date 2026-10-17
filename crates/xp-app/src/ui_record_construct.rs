//! Fresh data initializer141a3c750 and empty-tree clear141a40ad0.
//! General nonempty clear/failure paths remain unverified.
use crate::ui_input_construct::{Block, Services};
fn q(b: &mut [u8], o: usize, v: u64) {
    b[o..o + 8].copy_from_slice(&v.to_le_bytes());
}
fn d(b: &mut [u8], o: usize, v: u32) {
    b[o..o + 4].copy_from_slice(&v.to_le_bytes());
}
pub fn construct(
    image: &mut [u8],
    base: usize,
    services: &mut impl Services,
) -> Result<Block, &'static str> {
    if base.checked_add(0x198).is_none_or(|end| end > image.len()) {
        return Err("missing fresh record image");
    }
    for off in [0, 0x20, 0x40, 0x60, 0x80, 0xa8, 0xc8, 0xe8, 0x108] {
        image[base + off..base + off + 16].fill(0);
        q(image, base + off + 16, 0);
        q(image, base + off + 24, 15);
    }
    d(image, base + 0xa0, u32::MAX);
    image[base + 0xa4] = 0;
    d(image, base + 0x128, 0x1000000);
    image[base + 0x12c] = 0;
    for off in (0x130..=0x178).step_by(8) {
        q(image, base + off, 0);
    }
    q(image, base + 0x180, 0);
    q(image, base + 0x188, 0);
    let mut tree = services.allocate(image, 32)?;
    if tree.address == 0 || tree.bytes.len() != 32 {
        return Err("successful record sentinel allocation required");
    }
    for off in [0, 8, 16] {
        q(&mut tree.bytes, off, tree.address);
    }
    tree.bytes[24..26].copy_from_slice(&0x101u16.to_le_bytes());
    q(image, base + 0x180, tree.address);
    image[base + 0x190] = 0;
    image[base + 0x192..base + 0x194].copy_from_slice(&0xfcu16.to_le_bytes());
    image[base + 0x194] = 255;
    // Original141a40ad0 visits no nodes in this freshly initialized tree.
    Ok(tree)
}
