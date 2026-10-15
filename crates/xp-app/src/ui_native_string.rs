//! Reference MSVC string copy1405dd5c0 and constructor-argument consumption.
//! Stable owned backing blocks; normal successful allocation profile only.
use crate::ui_input_construct::{self, Block};
pub trait Services: ui_input_construct::Services {
    fn release(&mut self, image: &[u8], block: Block) -> Result<(), &'static str>;
}
#[derive(Clone, Debug)]
pub struct NativeString {
    pub raw: [u8; 32],
    pub storage: Option<Block>,
}
fn q(raw: &mut [u8], off: usize, v: u64) {
    raw[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
fn read(raw: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(raw[off..off + 8].try_into().unwrap())
}
impl NativeString {
    pub fn inline(raw: [u8; 32]) -> Result<Self, &'static str> {
        let s = Self { raw, storage: None };
        s.validate()?;
        Ok(s)
    }
    pub fn len(&self) -> usize {
        read(&self.raw, 16) as usize
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn capacity(&self) -> usize {
        read(&self.raw, 24) as usize
    }
    fn data(&self) -> Result<&[u8], &'static str> {
        if self.capacity() <= 15 {
            return Ok(&self.raw[..16]);
        }
        let b = self.storage.as_ref().ok_or("missing string backing")?;
        let off = read(&self.raw, 0)
            .checked_sub(b.address)
            .ok_or("invalid string backing pointer")? as usize;
        b.bytes.get(off..).ok_or("string pointer outside backing")
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.len() > self.capacity() || self.len() > 0x10000 || self.capacity() > 0x1ffff {
            return Err("invalid or oversized string profile");
        }
        if self.capacity() <= 15 && (self.capacity() != 15 || self.storage.is_some()) {
            return Err("invalid inline string profile");
        }
        let data = self.data()?;
        let needed = if self.len() <= 15 { 16 } else { self.len() + 1 };
        if data.len() < needed {
            return Err("short string backing");
        }
        Ok(())
    }
    /// Copy retains all16 data bytes for short strings, even from heap-backed sources.
    pub fn copy(&self, image: &[u8], services: &mut impl Services) -> Result<Self, &'static str> {
        self.validate()?;
        let len = self.len();
        let mut raw = [0; 32];
        q(&mut raw, 16, len as u64);
        if len <= 15 {
            raw[..16].copy_from_slice(&self.data()?[..16]);
            q(&mut raw, 24, 15);
            return Ok(Self { raw, storage: None });
        }
        let cap = (len | 15).max(22);
        let bytes = cap + 1;
        let large = bytes >= 0x1000;
        let mut block = services.allocate(image, bytes + if large { 39 } else { 0 })?;
        if block.address == 0 || block.bytes.len() != bytes + if large { 39 } else { 0 } {
            return Err("successful exact string allocation required");
        }
        let pointer = if large {
            (block.address + 39) & !31
        } else {
            block.address
        };
        let offset = (pointer - block.address) as usize;
        if large {
            q(&mut block.bytes, offset - 8, block.address);
        }
        block.bytes[offset..offset + len + 1].copy_from_slice(&self.data()?[..len + 1]);
        q(&mut raw, 0, pointer);
        q(&mut raw, 24, cap as u64);
        Ok(Self {
            raw,
            storage: Some(block),
        })
    }
    /// Destination is zeroed before any allocation, matching constructor snapshots.
    pub fn copy_into(
        &self,
        image: &mut [u8],
        offset: usize,
        services: &mut impl Services,
    ) -> Result<Self, &'static str> {
        self.validate()?;
        let dst = image
            .get_mut(offset..offset + 32)
            .ok_or("missing destination string")?;
        dst.fill(0);
        let result = self.copy(image, services)?;
        image[offset..offset + 32].copy_from_slice(&result.raw);
        Ok(result)
    }
    /// Frees heap backing then resets metadata and first byte, retaining residual bytes.
    pub fn consume(
        &mut self,
        image: &[u8],
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        self.validate()?;
        if self.capacity() > 15 {
            let block = self.storage.as_ref().ok_or("missing string backing")?;
            let user = read(&self.raw, 0);
            let large = self.capacity() + 1 >= 0x1000;
            let size = self.capacity() + 1 + if large { 39 } else { 0 };
            if size != block.bytes.len() {
                return Err("string sized-free backing mismatch");
            }
            if large {
                let offset = user
                    .checked_sub(block.address)
                    .ok_or("bad aligned string pointer")? as usize;
                if !(8..=39).contains(&offset) || read(&block.bytes, offset - 8) != block.address {
                    return Err("bad aligned string header");
                }
            } else if user != block.address {
                return Err("bad small string pointer");
            }
            services.release(image, block.clone())?;
            self.storage = None;
        }
        q(&mut self.raw, 16, 0);
        q(&mut self.raw, 24, 15);
        self.raw[0] = 0;
        Ok(())
    }
}
/// Preserve the legacy inline-only constructor API without inventing heap frees.
pub(crate) struct InlineServices<'a, S>(pub &'a mut S);
impl<S: ui_input_construct::Services> ui_input_construct::Services for InlineServices<'_, S> {
    fn allocate(&mut self, image: &[u8], bytes: usize) -> Result<Block, &'static str> {
        self.0.allocate(image, bytes)
    }
}
impl<S: ui_input_construct::Services> Services for InlineServices<'_, S> {
    fn release(&mut self, _: &[u8], _: Block) -> Result<(), &'static str> {
        Err("inline constructor cannot release heap strings")
    }
}
