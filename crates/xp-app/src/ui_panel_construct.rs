//! Native common panel constructor 0x14129c7a0 through input base constructors.
//! Inline/heap-owned string arguments; reference tables are opaque identities.
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
impl<S: crate::ui_native_string::Services> crate::ui_native_string::Services for Extended<'_, S> {
    fn release(&mut self, prefix: &[u8], block: Block) -> Result<(), &'static str> {
        let mut full = prefix.to_vec();
        full.extend_from_slice(self.tail);
        self.services.release(&full, block)
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
    let mut owned = [
        crate::ui_native_string::NativeString::inline(strings[0])?,
        crate::ui_native_string::NativeString::inline(strings[1])?,
        crate::ui_native_string::NativeString::inline(strings[2])?,
    ];
    let blocks = construct_owned(
        image,
        &mut owned,
        flags,
        &mut crate::ui_native_string::InlineServices(services),
    )?;
    for (raw, string) in strings.iter_mut().zip(owned) {
        *raw = string.raw;
    }
    Ok(blocks)
}
/// Full nested common-panel construction, including intermediate identifier
/// copies and their ownership cleanup. Normal successful allocation profile.
pub fn construct_owned(
    image: &mut [u8],
    strings: &mut [crate::ui_native_string::NativeString; 3],
    flags: [u8; 3],
    services: &mut impl crate::ui_native_string::Services,
) -> Result<Vec<Block>, &'static str> {
    if image.len() != 0x170 {
        return Err("panel image size mismatch");
    }
    for s in strings.iter() {
        s.validate()?;
    }
    let mut first = strings[1].copy(image, services)?;
    let mut second = first.copy(image, services)?;
    let mut third = second.copy(image, services)?;
    let mut blocks = {
        let (prefix, tail) = image.split_at_mut(0x128);
        ui_input_construct::construct_owned(prefix, &mut third, &mut Extended { tail, services })?
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
    second.consume(image, services)?;
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
    first.consume(image, services)?;
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
    let label = strings[0].copy_into(image, 0x128, services)?;
    let auxiliary = strings[2].copy_into(image, 0x148, services)?;
    for name in [label, auxiliary] {
        if let Some(block) = name.storage {
            blocks.push(block);
        }
    }
    image[0x168..0x16b].copy_from_slice(&flags);
    image[0x18..0x1c].copy_from_slice(&0xfffu32.to_le_bytes());
    for s in strings {
        s.consume(image, services)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui_native_string::NativeString;
    struct Heap {
        next: u64,
        freed: Vec<Block>,
    }
    impl Services for Heap {
        fn allocate(&mut self, _: &[u8], bytes: usize) -> Result<Block, &'static str> {
            let address = self.next;
            self.next += (bytes as u64 + 63) & !63;
            Ok(Block {
                address,
                bytes: vec![0xa5; bytes],
            })
        }
    }
    impl crate::ui_native_string::Services for Heap {
        fn release(&mut self, _: &[u8], block: Block) -> Result<(), &'static str> {
            self.freed.push(block);
            Ok(())
        }
    }
    fn inline(text: &[u8]) -> NativeString {
        let mut raw = [0; 32];
        raw[..text.len()].copy_from_slice(text);
        raw[16..24].copy_from_slice(&(text.len() as u64).to_le_bytes());
        raw[24..].copy_from_slice(&15u64.to_le_bytes());
        NativeString::inline(raw).unwrap()
    }
    #[test]
    fn dialog_profile_copies_heap_label_before_releasing_constructor_arguments() {
        let text = b"ABCDEFGHIJKLMNOPQRST";
        let mut raw = [0xcc; 32];
        raw[..8].copy_from_slice(&0x7000u64.to_le_bytes());
        raw[16..24].copy_from_slice(&20u64.to_le_bytes());
        raw[24..].copy_from_slice(&31u64.to_le_bytes());
        let mut backing = vec![0xcc; 32];
        backing[..20].copy_from_slice(text);
        backing[20] = 0;
        let mut strings = [
            NativeString {
                raw,
                storage: Some(Block {
                    address: 0x7000,
                    bytes: backing,
                }),
            },
            inline(b"DialogName"),
            inline(b"Help"),
        ];
        let mut image = vec![0xdd; 0x170];
        let mut heap = Heap {
            next: 0x10000,
            freed: vec![],
        };
        let blocks = construct_owned(&mut image, &mut strings, [1, 0, 1], &mut heap).unwrap();
        let pointer = u64::from_le_bytes(image[0x128..0x130].try_into().unwrap());
        assert_ne!(pointer, 0x7000);
        let label = blocks.iter().find(|b| b.address == pointer).unwrap();
        assert_eq!(&label.bytes[..21], &[text.as_slice(), &[0]].concat());
        assert_eq!(
            heap.freed.iter().map(|b| b.address).collect::<Vec<_>>(),
            vec![0x7000]
        );
        assert!(strings.iter().all(|s| s.is_empty() && s.storage.is_none()));
        assert_eq!(&image[0x60..0x6a], b"DialogName");
        let key = key_state(&image).unwrap();
        assert_eq!((key.flag168, key.flag169), (1, 0));
    }
}
