//! Payload move1410faec0, optional string move1405de290 and small-vector
//! move1410fa780/growth1408b0660. Successful mapped ownership profile.
use crate::ui_input_construct::Block;
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct State {
    pub blocks: BTreeMap<u64, Vec<u8>>,
}
impl State {
    pub fn read(&self, p: u64, n: usize) -> Result<Vec<u8>, &'static str> {
        let (base, b) = self.blocks.range(..=p).next_back().ok_or("unmapped read")?;
        let o = (p - base) as usize;
        Ok(b.get(o..o.checked_add(n).ok_or("range overflow")?)
            .ok_or("short mapped read")?
            .to_vec())
    }
    pub fn write(&mut self, p: u64, data: &[u8]) -> Result<(), &'static str> {
        let (base, b) = self
            .blocks
            .range_mut(..=p)
            .next_back()
            .ok_or("unmapped write")?;
        let o = (p - base) as usize;
        b.get_mut(o..o.checked_add(data.len()).ok_or("range overflow")?)
            .ok_or("short mapped write")?
            .copy_from_slice(data);
        Ok(())
    }
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(self.read(p, 8)?.try_into().unwrap()))
    }
    fn putq(&mut self, p: u64, v: u64) -> Result<(), &'static str> {
        self.write(p, &v.to_le_bytes())
    }
    fn copy(&mut self, d: u64, s: u64, n: usize) -> Result<(), &'static str> {
        if n == 0 {
            return Ok(());
        }
        let b = self.read(s, n)?;
        self.write(d, &b)
    }
}
pub trait Services {
    fn allocate_vector(&mut self, state: &mut State, bytes: usize) -> Result<Block, &'static str>;
    fn free(
        &mut self,
        state: &mut State,
        address: u64,
        bytes: Option<usize>,
    ) -> Result<(), &'static str>;
}
fn free_string(
    state: &mut State,
    p: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let cap = state.q(p + 24)?;
    if cap > 15 {
        let mut address = state.q(p)?;
        let mut bytes = cap.checked_add(1).ok_or("capacity overflow")?;
        if bytes >= 4096 {
            let base = state.q(address.checked_sub(8).ok_or("alignment header overflow")?)?;
            let delta = address.checked_sub(base).ok_or("invalid aligned string")?;
            if !(8..=39).contains(&delta) {
                return Err("invalid aligned string header");
            }
            address = base;
            bytes = bytes.checked_add(39).ok_or("free size overflow")?;
        }
        services.free(
            state,
            address,
            Some(bytes.try_into().map_err(|_| "free size overflow")?),
        )?;
    }
    Ok(())
}
fn reset_string(state: &mut State, p: u64) -> Result<(), &'static str> {
    state.putq(p + 24, 15)?;
    state.putq(p + 16, 0)?;
    state.write(p, &[0])
}
fn move_string(
    state: &mut State,
    d: u64,
    s: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if d == s {
        return Ok(());
    }
    free_string(state, d, services)?;
    reset_string(state, d)?;
    state.copy(d, s, 32)?;
    reset_string(state, s)
}
fn count(start: u64, end: u64) -> Result<usize, &'static str> {
    let n = end.checked_sub(start).ok_or("invalid vector range")?;
    if n % 4 != 0 || n > 0x40000 {
        return Err("bounded aligned vector required");
    }
    Ok((n / 4) as usize)
}
fn grow(
    state: &mut State,
    d: u64,
    n: usize,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let begin = state.q(d)?;
    let cap = count(begin, state.q(d + 16)?)?;
    let old = count(begin, state.q(d + 8)?)?;
    let mut mask = cap.checked_add(2).ok_or("capacity overflow")?;
    for shift in [1, 2, 4, 8, 16, 32] {
        mask |= mask >> shift;
    }
    let bytes = mask
        .checked_add(1)
        .ok_or("capacity overflow")?
        .max(n)
        .checked_mul(4)
        .ok_or("capacity overflow")?;
    let b = services.allocate_vector(state, bytes)?;
    if b.address == 0 || b.bytes.len() != bytes || state.blocks.contains_key(&b.address) {
        return Err("fresh successful vector allocation required");
    }
    let allocated = b.address;
    state.blocks.insert(b.address, b.bytes);
    let previous = state.q(d)?;
    let live_bytes = count(previous, state.q(d + 8)?)? * 4;
    state.copy(allocated, previous, live_bytes)?;
    if previous != d + 24 {
        services.free(state, previous, None)?;
    }
    state.putq(d + 8, allocated + old as u64 * 4)?;
    state.putq(d, allocated)?;
    state.putq(d + 16, allocated + bytes as u64)
}
fn move_vector(
    state: &mut State,
    d: u64,
    s: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if d == s {
        return Ok(());
    }
    let source = state.q(s)?;
    if source != s + 24 {
        let old = state.q(d)?;
        if old != d + 24 {
            services.free(state, old, None)?;
        }
        for o in [0, 8, 16] {
            let value = state.q(s + o)?;
            state.putq(d + o, value)?;
        }
        for o in [16, 8, 0] {
            state.putq(s + o, s + 24)?;
        }
        return Ok(());
    }
    let begin = state.q(d)?;
    let mut old = count(begin, state.q(d + 8)?)?;
    let new = count(source, state.q(s + 8)?)?;
    if old >= new {
        if new != 0 {
            state.copy(begin, source, new * 4)?;
        }
        state.putq(d + 8, begin + new as u64 * 4)?;
    } else {
        let cap = count(begin, state.q(d + 16)?)?;
        if cap < new {
            old = 0;
            state.putq(d + 8, begin)?;
            grow(state, d, new, services)?;
        } else if old != 0 {
            state.copy(begin, source, old * 4)?;
        }
        let target = state.q(d)?;
        let source = state.q(s)?;
        let end = state.q(s + 8)?;
        state.copy(
            target + old as u64 * 4,
            source + old as u64 * 4,
            count(source + old as u64 * 4, end)? * 4,
        )?;
        let target = state.q(d)?;
        state.putq(d + 8, target + new as u64 * 4)?;
    }
    let source = state.q(s)?;
    state.putq(s + 8, source)
}
pub fn assign(
    state: &mut State,
    destination: u64,
    source: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    state.read(destination, 0xe0)?;
    state.read(source, 0xe0)?;
    state.copy(destination, source, 0x2c)?;
    let present = state.read(source + 0x58, 1)?[0] != 0;
    let old = state.read(destination + 0x58, 1)?[0] != 0;
    if present {
        if old {
            move_string(state, destination + 0x30, source + 0x30, services)?;
        } else {
            state.write(destination + 0x30, &[0; 32])?;
            state.copy(destination + 0x30, source + 0x30, 32)?;
            reset_string(state, source + 0x30)?;
            state.write(destination + 0x58, &[1])?;
        }
        state.copy(destination + 0x50, source + 0x50, 4)?;
    } else if old {
        free_string(state, destination + 0x30, services)?;
        reset_string(state, destination + 0x30)?;
        state.write(destination + 0x58, &[0])?;
    }
    state.copy(destination + 0x60, source + 0x60, 0x44)?;
    move_vector(state, destination + 0xa8, source + 0xa8, services)
}

/// Adapter for ui_nested_refresh::Services::assign_payload. Keeps raw object,
/// temporary and backing blocks synchronized, including partial state on errors.
/// Backing blocks model storage identity; providers own actual free bookkeeping.
pub fn assign_nested(
    state: &mut crate::ui_nested_refresh::State,
    identity: u64,
    temporary: &mut crate::ui_nested_refresh::Temporary,
    backing: &mut BTreeMap<u64, Vec<u8>>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if state.image.len() != 0x158
        || temporary.raw.len() != 0xe0
        || backing.contains_key(&identity)
        || backing.contains_key(&temporary.identity)
        || identity == temporary.identity
    {
        return Err("distinct valid nested payload mappings required");
    }
    let mut memory = State {
        blocks: std::mem::take(backing),
    };
    memory.blocks.insert(identity, state.image.clone());
    memory
        .blocks
        .insert(temporary.identity, temporary.raw.clone());
    let result = assign(&mut memory, identity + 0x40, temporary.identity, services);
    state.image = memory
        .blocks
        .remove(&identity)
        .ok_or("lost nested mapping")?;
    temporary.raw = memory
        .blocks
        .remove(&temporary.identity)
        .ok_or("lost temporary mapping")?;
    *backing = memory.blocks;
    result
}
