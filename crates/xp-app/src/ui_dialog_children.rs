//! Complete outer factory1410d6ca0 used at dialog+2d0.
//! Four nested constructors, resource lookup and final refresh are services.
//! Inline fifth-object/container initialization executes in this module.
use crate::ui_input_construct::Block;
use std::collections::BTreeMap;
#[derive(Clone, Debug)]
pub struct State {
    pub bundle: [u8; 40],
    pub blocks: BTreeMap<u64, Vec<u8>>,
}
impl State {
    pub fn new(bundle: [u8; 40]) -> Self {
        Self {
            bundle,
            blocks: BTreeMap::new(),
        }
    }
    fn allocate(
        &mut self,
        bytes: usize,
        required: bool,
        services: &mut impl Services,
    ) -> Result<Option<u64>, &'static str> {
        match services.allocate(self, bytes)? {
            None if !required => Ok(None),
            None => Err("native internal nonnull allocation required"),
            Some(b) => {
                if b.address == 0 || b.bytes.len() != bytes || self.blocks.contains_key(&b.address)
                {
                    return Err("invalid fresh allocation");
                }
                let p = b.address;
                self.blocks.insert(p, b.bytes);
                Ok(Some(p))
            }
        }
    }
    fn q(&mut self, p: u64, off: usize, v: u64) {
        self.blocks.get_mut(&p).unwrap()[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn byte(&mut self, p: u64, off: usize, v: u8) {
        self.blocks.get_mut(&p).unwrap()[off] = v;
    }
    fn slot(&mut self, off: usize, v: u64) {
        self.bundle[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }
    fn value(&self, off: usize) -> u64 {
        u64::from_le_bytes(self.bundle[off..off + 8].try_into().unwrap())
    }
    fn child(
        &mut self,
        kind: u8,
        size: usize,
        args: &[u64],
        services: &mut impl Services,
    ) -> Result<u64, &'static str> {
        if let Some(p) = self.allocate(size, false, services)? {
            services.construct(self, kind, p, args)
        } else {
            Ok(0)
        }
    }
    pub fn construct(&mut self, services: &mut impl Services) -> Result<(), &'static str> {
        let first = self.child(0, 0x3e0, &[1], services)?;
        self.slot(0, first);
        let second = self.child(1, 0x2d0, &[first], services)?;
        self.slot(8, second);
        let third = self.child(2, 0x60, &[second], services)?;
        self.slot(16, third);
        let parent = self.value(8);
        let fourth = self.child(3, 0x3e8, &[parent, third], services)?;
        self.slot(24, fourth);
        let fifth = if let Some(p) = self.allocate(0xb0, false, services)? {
            self.q(p, 0, 0x142642d00);
            self.q(p, 8, 0);
            self.q(p, 16, 0);
            let list = self.allocate(0x78, true, services)?.unwrap();
            self.q(list, 0, list);
            self.q(list, 8, list);
            self.q(p, 8, list);
            self.byte(p, 0x18, 0);
            self.q(p, 0, 0x142642fc0);
            let resource = services.resource(self, 20)?;
            self.q(p, 0x20, resource);
            self.byte(p, 0x28, 0);
            self.byte(p, 0x30, 0);
            for off in [0x38, 0x40, 0x48] {
                self.q(p, off, 0);
            }
            self.blocks.get_mut(&p).unwrap()[0x50..0x54].fill(0);
            self.q(p, 0x58, 0);
            self.q(p, 0x60, 0);
            let sentinel = self.allocate(32, true, services)?.unwrap();
            self.q(sentinel, 0, sentinel);
            self.q(sentinel, 8, sentinel);
            self.q(p, 0x58, sentinel);
            for off in [0x68, 0x70, 0x78] {
                self.q(p, off, 0);
            }
            self.q(p, 0x80, 7);
            self.q(p, 0x88, 8);
            self.blocks.get_mut(&p).unwrap()[0x50..0x54]
                .copy_from_slice(&1f32.to_bits().to_le_bytes());
            // Original empty vector-resize1405de740 allocates16 pointers, commits
            // begin/end/capacity, then fills every entry with the sentinel.
            let buckets = self.allocate(128, true, services)?.unwrap();
            self.q(p, 0x68, buckets);
            self.q(p, 0x70, buckets + 128);
            self.q(p, 0x78, buckets + 128);
            for off in (0..128).step_by(8) {
                self.q(buckets, off, sentinel);
            }
            self.byte(p, 0x94, 0);
            for off in [0x98, 0xa0, 0xa8] {
                self.q(p, off, 0);
            }
            self.byte(p, 0x30, 0);
            services.refresh(self, p)?;
            p
        } else {
            0
        };
        self.slot(32, fifth);
        Ok(())
    }
}
/// Publish logical receiver links/counts into the factory's mapped byte images.
/// Callback RB nodes and receiver wrapper payloads remain logical, not ABI bytes.
/// Call after a child constructor returns its owned base/record blocks.
pub fn publish_callback_links(
    state: &mut State,
    callbacks: &crate::ui_callback_links::State,
    parents: &BTreeMap<u64, crate::ui_callback_list::List>,
) -> Result<(), &'static str> {
    fn put(state: &mut State, p: u64, value: u64) -> Result<(), &'static str> {
        let (base, bytes) = state
            .blocks
            .range_mut(..=p)
            .next_back()
            .ok_or("unmapped callback image")?;
        let offset = (p - base) as usize;
        bytes
            .get_mut(offset..offset + 8)
            .ok_or("short callback image")?
            .copy_from_slice(&value.to_le_bytes());
        Ok(())
    }
    for (parent, list) in parents {
        put(state, parent + 16, list.entries.len() as u64)?;
        for (p, next, previous) in list.links() {
            if list.entries.contains(&p) {
                state.blocks.entry(p).or_insert_with(|| vec![0; 16]);
            }
            put(state, p, next)?;
            put(state, p + 8, previous)?;
        }
    }
    for (p, node) in &callbacks.nodes {
        if node.kind == crate::ui_callback_links::Kind::Container {
            put(state, p + 16, node.peers.len() as u64)?;
        }
    }
    Ok(())
}

pub trait Services {
    fn allocate(&mut self, state: &mut State, bytes: usize) -> Result<Option<Block>, &'static str>;
    /// Kinds0..3 correspond to1410d6a40/1410d6fc0/1410d7490/1410d75a0.
    fn construct(
        &mut self,
        state: &mut State,
        kind: u8,
        allocation: u64,
        args: &[u64],
    ) -> Result<u64, &'static str>;
    /// 141a64260(global146124398,20); result remains opaque.
    fn resource(&mut self, state: &mut State, index: u32) -> Result<u64, &'static str>;
    /// 141139460; recovered by ui_fifth_refresh with caller-supplied mapped
    /// runtime/global state and dependent providers.
    fn refresh(&mut self, state: &mut State, object: u64) -> Result<(), &'static str>;
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Heap {
        next: u64,
        parents: Vec<Vec<u64>>,
        resource_calls: usize,
    }
    impl Services for Heap {
        fn allocate(&mut self, _: &mut State, bytes: usize) -> Result<Option<Block>, &'static str> {
            if bytes == 0xb0 {
                return Ok(None);
            }
            let p = self.next;
            self.next += 0x1000;
            Ok(Some(Block {
                address: p,
                bytes: vec![0xa5; bytes],
            }))
        }
        fn construct(
            &mut self,
            _: &mut State,
            _: u8,
            p: u64,
            args: &[u64],
        ) -> Result<u64, &'static str> {
            self.parents.push(args.to_vec());
            Ok(p + 16)
        }
        fn resource(&mut self, _: &mut State, _: u32) -> Result<u64, &'static str> {
            self.resource_calls += 1;
            Ok(0)
        }
        fn refresh(&mut self, _: &mut State, _: u64) -> Result<(), &'static str> {
            panic!("null optional fifth object must skip refresh")
        }
    }
    #[test]
    fn factory_forwards_constructor_results_and_skips_null_optional_object() {
        let mut s = State::new([0xcc; 40]);
        let mut heap = Heap {
            next: 0x1000,
            parents: vec![],
            resource_calls: 0,
        };
        s.construct(&mut heap).unwrap();
        assert_eq!(
            heap.parents,
            vec![vec![1], vec![0x1010], vec![0x2010], vec![0x2010, 0x3010]]
        );
        assert_eq!(s.value(0), 0x1010);
        assert_eq!(s.value(24), 0x4010);
        assert_eq!(s.value(32), 0);
        assert_eq!(heap.resource_calls, 0);
    }
}
