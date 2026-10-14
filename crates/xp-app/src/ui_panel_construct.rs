//! Native common panel constructor 0x14129c7a0 through input base constructors.
//! Inline strings only; reference table addresses are opaque identities.
use crate::ui_input_construct::{self, Block, Services};
struct Extended<'a, S> {
    tail: &'a [u8],
    services: &'a mut S,
}
impl<S: Services> Services for Extended<'_, S> {
    fn allocate(&mut self, prefix: &[u8], bytes: usize) -> Result<Block, &'static str> {
        let mut full = prefix.to_vec();
        full.extend_from_slice(self.tail);
        self.services.allocate(&full, bytes)
    }
}
fn tables(image: &mut [u8], values: [u64; 7]) {
    for (off, value) in [0, 0x20, 0x28, 0x30, 0x38, 0x40, 0x48]
        .into_iter()
        .zip(values)
    {
        image[off..off + 8].copy_from_slice(&value.to_le_bytes());
    }
}
fn valid(s: &[u8; 32]) -> bool {
    u64::from_le_bytes(s[16..24].try_into().unwrap()) <= 15
        && u64::from_le_bytes(s[24..32].try_into().unwrap()) == 15
}
fn consume(s: &mut [u8; 32]) {
    s[16..24].fill(0);
    s[24..32].copy_from_slice(&15u64.to_le_bytes());
    s[0] = 0;
}
/// Label/identifier/auxiliary arguments correspond to RDX/R8/R9; flags are
/// the low bytes of stack arguments5/6/7. Full concrete main-menu ctor remains separate.
pub fn construct(
    image: &mut [u8],
    strings: &mut [[u8; 32]; 3],
    flags: [u8; 3],
    services: &mut impl Services,
) -> Result<Vec<Block>, &'static str> {
    if image.len() != 0x170 || !strings.iter().all(valid) {
        return Err("valid inline panel construction profile required");
    }
    let mut identifier = strings[1];
    let blocks = {
        let (prefix, tail) = image.split_at_mut(0x128);
        ui_input_construct::construct(prefix, &mut identifier, &mut Extended { tail, services })?
    };
    tables(
        image,
        [
            0x1426748c0,
            0x1426749e0,
            0x142674a18,
            0x142674a70,
            0x142674a98,
            0x142674ab8,
            0x142674af8,
        ],
    );
    tables(
        image,
        [
            0x1426029e0,
            0x142602b08,
            0x142602b40,
            0x142602b98,
            0x142602bc0,
            0x142602be0,
            0x142602c20,
        ],
    );
    tables(
        image,
        [
            0x14267c228,
            0x14267c3b0,
            0x14267c3e8,
            0x14267c440,
            0x14267c468,
            0x14267c488,
            0x14267c4c8,
        ],
    );
    image[0x128..0x148].copy_from_slice(&strings[0]);
    image[0x148..0x168].copy_from_slice(&strings[2]);
    image[0x168..0x16b].copy_from_slice(&flags);
    image[0x18..0x1c].copy_from_slice(&0xfffu32.to_le_bytes());
    for s in strings {
        consume(s);
    }
    Ok(blocks)
}
/// Getter fields used by concrete panel own-key dispatch.
pub fn key_state(image: &[u8]) -> Result<crate::ui_menu_key::State, &'static str> {
    if image.len() < 0x16b {
        return Err("missing panel key fields");
    }
    Ok(crate::ui_menu_key::State {
        flag168: image[0x168],
        flag169: image[0x169],
    })
}
