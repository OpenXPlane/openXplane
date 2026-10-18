//! Selection reset predicate14114d330, primary string comparison14113fff0
//! and variant-tag query1417fb390. Version-bound mapped state; secondary
//! predicate14113ff10 remains a dependency. This does not implement reset itself.
use crate::{ui_engine_query, ui_indexed_records, ui_payload_move};

pub const RESOURCE_INDEX: u64 = 0x142640504;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
}
pub trait Services {
    fn engine(&mut self, s: &mut State) -> Result<u64, &'static str> {
        ui_engine_query::query(&s.memory)
    }
    fn primary(&mut self, s: &mut State, receiver: u64) -> Result<u64, &'static str> {
        primary_differs(s, receiver, self)
    }
    fn compare_bytes(
        &mut self,
        s: &mut State,
        left: u64,
        right: u64,
        bytes: u64,
    ) -> Result<i32, &'static str> {
        compare_bytes(s, left, right, bytes)
    }
    fn secondary(&mut self, s: &mut State, receiver: u64) -> Result<u64, &'static str>;
}
/// Original CRT equality operand; zero count never dereferences either pointer.
/// Only zero/nonzero matters to14113fff0, not memcmp's negative/positive magnitude.
pub fn compare_bytes(s: &State, left: u64, right: u64, bytes: u64) -> Result<i32, &'static str> {
    if bytes == 0 {
        return Ok(0);
    }
    let n = usize::try_from(bytes).map_err(|_| "comparison size overflow")?;
    Ok(i32::from(
        s.memory.read(left, n)? != s.memory.read(right, n)?,
    ))
}
/// Canonical boolean result; native callers inspect AL, not the stale upper RAX bits.
pub fn primary_differs(
    s: &mut State,
    receiver: u64,
    h: &mut (impl Services + ?Sized),
) -> Result<u64, &'static str> {
    let index = u32::from_le_bytes(s.memory.read(RESOURCE_INDEX, 4)?.try_into().unwrap());
    let record =
        ui_indexed_records::lookup(&s.memory, ui_indexed_records::Kind::Resource, index as u64)?;
    let mut right = record.wrapping_add(0x6178);
    let mut left = receiver.wrapping_add(0x20);
    let right_cap = s.q(right.wrapping_add(24))?;
    let right_len = s.q(right.wrapping_add(16))?;
    if right_cap > 15 {
        right = s.q(right)?;
    }
    let left_cap = s.q(left.wrapping_add(24))?;
    let left_len = s.q(left.wrapping_add(16))?;
    if left_cap > 15 {
        left = s.q(left)?;
    }
    if left_len != right_len {
        return Ok(1);
    }
    Ok(u64::from(h.compare_bytes(s, left, right, left_len)? != 0))
}
pub fn skip_reset(s: &mut State, h: &mut impl Services) -> Result<bool, &'static str> {
    let receiver = s.q(s.identity.wrapping_add(0xc8))?;
    let blocked = receiver != 0
        && (h.engine(s)? as u8 == 0
            || h.primary(s, receiver)? as u8 != 0
            || h.secondary(s, receiver)? as u8 != 0);
    // Native reads this byte after every reached dependency; receiver remains cached.
    Ok(s.memory.read(s.identity.wrapping_add(0xc0), 1)?[0] != 0 && !blocked)
}
pub fn variant_is_two_or_three(
    memory: &ui_payload_move::State,
    receiver: u64,
) -> Result<u64, &'static str> {
    let tag = memory.read(receiver.wrapping_add(0x68), 1)?[0];
    Ok(u64::from(tag.wrapping_sub(2) <= 1))
}
