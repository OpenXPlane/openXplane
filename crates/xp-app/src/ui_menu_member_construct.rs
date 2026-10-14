//! Main-menu member constructors 0x140ec3280/0x140ec3330 and parent registration.
//! Reference tables are identities; allocator/CRT endpoints are explicit services.
use crate::ui_input_construct::Block;
#[derive(Clone, Debug, Default)]
pub struct Registry {
    pub entries: Vec<u64>,
    pub capacity: usize,
    pub storage: Option<Block>,
}
pub trait Services {
    fn allocate(&mut self, bytes: usize) -> Result<Block, &'static str>;
    fn copied(&mut self, bytes: usize);
    fn free(&mut self, block: Block);
}
fn q(image: &mut [u8], offset: usize, value: u64) {
    image[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}
impl Registry {
    pub fn append(&mut self, value: u64, services: &mut impl Services) -> Result<(), &'static str> {
        let len = self.entries.len();
        if len > self.capacity {
            return Err("invalid registry length");
        }
        if len == self.capacity {
            let new_capacity = (self.capacity + self.capacity / 2).max(len + 1);
            let bytes = new_capacity
                .checked_mul(8)
                .ok_or("registry size overflow")?;
            if bytes >= 0x1000 {
                return Err("large aligned registry allocation unverified");
            }
            let mut block = services.allocate(bytes)?;
            if block.address == 0 || block.bytes.len() != bytes {
                return Err("native nonnull allocation profile required");
            }
            q(&mut block.bytes, len * 8, value);
            if let Some(old) = &self.storage {
                block.bytes[..len * 8].copy_from_slice(&old.bytes[..len * 8]);
            }
            services.copied(len * 8);
            if let Some(old) = self.storage.take() {
                services.free(old);
            }
            self.storage = Some(block);
            self.capacity = new_capacity;
        } else {
            let block = self.storage.as_mut().ok_or("missing registry storage")?;
            q(&mut block.bytes, len * 8, value);
        }
        self.entries.push(value);
        Ok(())
    }
}
/// Variant false is used at panel+1b0/+208; true at +260. No high-level
/// callback meaning is inferred. Input image0x58 and captured identity must be stable.
pub fn construct(
    image: &mut [u8],
    identity: u64,
    parent: u64,
    variant: bool,
    registry: &mut Registry,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0x58 {
        return Err("member image size mismatch");
    }
    q(image, 0, 0);
    q(image, 8, parent);
    q(image, 0x10, 0x142604a38);
    q(image, 0x50, 0);
    q(image, 0x18, if variant { 0x14260d050 } else { 0x14260d018 });
    q(image, 0x20, identity);
    q(image, 0x50, identity + 0x18);
    if parent != 0 {
        registry.append(identity + 0x10, services)?;
    }
    Ok(())
}
