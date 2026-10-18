//! Fifth dialog child refresh141139460 and hash clear14062b900.
//! Version-bound provider identities are data, never executable host pointers.
use crate::{
    ui_engine_query,
    ui_indexed_records::{self, Kind},
    ui_payload_move,
};
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    /// Scratch frame identity after the original refresh prologue.
    pub stack: u64,
    pub active_global: u64,
    pub resource_index: u32,
    pub override_enabled: u32,
    pub override_bits: u32,
}
pub trait Services {
    fn invoke(
        &mut self,
        state: &mut State,
        function: u64,
        args: &[u64],
    ) -> Result<u64, &'static str>;
    /// Read-only original1417fb320. Legacy traces override the controlled endpoint.
    fn engine_query(&mut self, state: &mut State) -> Result<u64, &'static str> {
        ui_engine_query::query(&state.memory)
    }
    /// Original indexed table access. Callers supply mapped table descriptors.
    fn resource_record(&mut self, state: &mut State, index: u32) -> Result<u64, &'static str> {
        ui_indexed_records::lookup(&state.memory, Kind::Resource, index as u64)
    }
    fn settings_record(&mut self, state: &mut State, index: u32) -> Result<u64, &'static str> {
        ui_indexed_records::lookup(&state.memory, Kind::Settings, index as u64)
    }
    /// Default executes recovered14113b9a0. Controlled legacy adapters may
    /// override the opaque endpoint while comparing only its caller.
    fn payload_available(&mut self, state: &mut State) -> Result<bool, &'static str>
    where
        Self: Sized,
    {
        payload_available(state, self)
    }
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
    fn d(&self, p: u64) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.memory.read(p, 4)?.try_into().unwrap(),
        ))
    }
    fn putq(&mut self, p: u64, n: u64) -> Result<(), &'static str> {
        self.memory.write(p, &n.to_le_bytes())
    }
    fn byte(&self, p: u64) -> Result<u8, &'static str> {
        Ok(self.memory.read(p, 1)?[0])
    }
    fn set(&mut self, p: u64, n: u8) -> Result<(), &'static str> {
        self.memory.write(p, &[n])
    }
}
fn call(s: &mut State, h: &mut impl Services, f: u64, a: &[u64]) -> Result<u64, &'static str> {
    h.invoke(s, f, a)
}
fn bounded(begin: u64, end: u64) -> Result<usize, &'static str> {
    let n = end.checked_sub(begin).ok_or("invalid live pointer range")?;
    if n % 8 != 0 || n / 8 > 100000 {
        return Err("bounded aligned pointer range required");
    }
    Ok((n / 8) as usize)
}
/// Nonempty sparse tables delegate original range-erase1406215a0; bulk clear
/// follows cached next pointers, reloads the sentinel and fills valid buckets.
fn clear_hash(s: &mut State, h: &mut impl Services, table: u64) -> Result<(), &'static str> {
    let count = s.q(table + 16)?;
    if count == 0 {
        return Ok(());
    }
    if s.q(table + 56)? >> 3 > count {
        let head = s.q(table + 8)?;
        let first = s.q(head)?;
        call(s, h, 0x1406215a0, &[table, first, head])?;
        return Ok(());
    }
    let head = s.q(table + 8)?;
    let last = s.q(head + 8)?;
    s.putq(last, 0)?;
    let mut current = s.q(head)?;
    let mut visits = 0;
    while current != 0 {
        visits += 1;
        if visits > 100000 {
            return Err("unverified nonterminating hash clear");
        }
        let next = s.q(current)?;
        call(s, h, 0x1422e7c2c, &[current, 32])?;
        current = next;
    }
    let head = s.q(table + 8)?;
    s.putq(head, head)?;
    let head = s.q(table + 8)?;
    s.putq(head + 8, head)?;
    s.putq(table + 16, 0)?;
    let head = s.q(table + 8)?;
    let begin = s.q(table + 24)?;
    let end = s.q(table + 32)?;
    s.putq(s.stack, head)?;
    for i in 0..bounded(begin, end)? {
        s.putq(begin + i as u64 * 8, head)?;
    }
    Ok(())
}
/// Predicate14113b9a0 reloads index after each provider call; AL controls engine
/// acceptance and the settings mode is compared as a full32-bit zero test.
pub fn payload_available(s: &mut State, h: &mut impl Services) -> Result<bool, &'static str> {
    if h.engine_query(s)? as u8 == 0 || s.active_global != 0 {
        return Ok(false);
    }
    let resource = h.resource_record(s, s.resource_index)?;
    if s.q(resource + 0x6188)? == 0 {
        return Ok(false);
    }
    let settings = h.settings_record(s, s.resource_index)?;
    Ok(s.d(settings + 0xdac)? == 0)
}
pub fn refresh(s: &mut State, h: &mut impl Services) -> Result<(), &'static str> {
    let o = s.identity;
    let sp = s.stack;
    let vector = o + 0x98;
    s.set(o + 0x30, 0)?;
    s.set(o + 0x28, 0)?;
    clear_hash(s, h, o + 0x50)?;
    let begin = s.q(vector)?;
    if begin != s.q(vector + 8)? {
        s.putq(vector + 8, begin)?;
    }
    if h.engine_query(s)? as u8 != 0 && s.active_global == 0 {
        let resource = h.resource_record(s, s.resource_index)?;
        if s.q(resource + 0x6188)? != 0 {
            let manager = call(s, h, 0x14193ae40, &[])?;
            call(s, h, 0x141660c60, &[sp + 0x20, manager])?;
            let mut visits = 0;
            while s.q(sp + 0x20)? != 0 {
                visits += 1;
                if visits > 100000 {
                    return Err("unverified nonterminating iterator");
                }
                let current = s.q(sp + 0x20)?;
                let index = s.d(sp + 0x28)? as i32 as i64;
                let item = current.wrapping_add(((index + 5) * 8) as u64);
                let end = s.q(vector + 8)?;
                if end != s.q(vector + 16)? {
                    let value = s.q(item)?;
                    s.putq(end, value)?;
                    let live = s.q(vector + 8)?;
                    s.putq(vector + 8, live + 8)?;
                } else {
                    call(s, h, 0x1405dcc70, &[vector, end, item])?;
                }
                call(s, h, 0x1406999d0, &[sp + 0x20])?;
            }
            let head = s.q(sp + 0x50)?;
            let last = s.q(head + 8)?;
            s.putq(last, 0)?;
            let mut current = s.q(head)?;
            let mut visits = 0;
            while current != 0 {
                visits += 1;
                if visits > 100000 {
                    return Err("unverified nonterminating temporary teardown");
                }
                let next = s.q(current)?;
                call(s, h, 0x1422e7c2c, &[current, 32])?;
                current = next;
            }
            let head = s.q(sp + 0x50)?;
            call(s, h, 0x1422e7c2c, &[head, 32])?;
            let end = s.q(vector + 8)?;
            let begin = s.q(vector)?;
            let count = (end.wrapping_sub(begin) as i64 >> 3) as u64;
            call(s, h, 0x1407925b0, &[begin, end, count, 0])?;
        }
    }
    if h.payload_available(s)? {
        let manager = call(s, h, 0x14193ae40, &[])?;
        let payload = call(s, h, 0x1417f71d0, &[sp + 0x20, 0, manager])?;
        call(s, h, 0x1410fac50, &[o + 0x30, payload])?;
        call(s, h, 0x1405ddae0, &[sp + 0x40])?;
        call(s, h, 0x1408ace50, &[sp + 0x28])?;
    } else {
        let begin = s.q(o + 0x38)?;
        if begin != s.q(o + 0x40)? {
            s.putq(o + 0x40, begin)?;
        }
        let begin = s.q(vector)?;
        let end = s.q(vector + 8)?;
        for i in 0..bounded(begin, end)? {
            let value = s.q(begin + i as u64 * 8)?;
            s.putq(sp + 0xc0, value)?;
            let result = call(s, h, 0x1410d2f10, &[o + 0x50, sp + 0x90, sp + 0xc0])?;
            let entry = s.q(result)?;
            s.set(entry + 0x18, 0)?;
        }
        if s.override_enabled != 0 {
            s.memory.write(o + 0x90, &s.override_bits.to_le_bytes())?;
            if s.byte(o + 0x94)? == 0 {
                s.set(o + 0x94, 1)?;
            }
        }
    }
    s.set(o + 0x18, 1)?;
    let head = s.q(o + 8)?;
    let mut current = s.q(head)?;
    let mut visits = 0;
    while current != s.q(o + 8)? {
        visits += 1;
        if visits > 100000 {
            return Err("unverified nonterminating listener list");
        }
        call(s, h, 0x1410c9d10, &[current + 16])?;
        current = s.q(current)?;
    }
    s.set(o + 0x18, 0)
}
pub fn refresh_image(
    image: &mut [u8],
    runtime: &mut State,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0xb0 {
        return Err("fifth child image must be0xb0 bytes");
    }
    runtime
        .memory
        .blocks
        .insert(runtime.identity, image.to_vec());
    let result = refresh(runtime, services);
    image.copy_from_slice(&runtime.memory.read(runtime.identity, 0xb0)?);
    result
}
