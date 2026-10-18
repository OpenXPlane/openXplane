//! Original weather child refresh141139fe0. Logical memory, bounded listeners.
//! Weather computation and context providers remain explicit services.
use crate::{
    ui_engine_query,
    ui_indexed_records::{self, Kind},
    ui_payload_move,
};
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    pub stack: u64,
    pub active_global: u64,
    pub resource_index: u32,
}
pub trait Services {
    /// Floating provider operands/results are raw XMM low-word bits in args/RAX.
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn engine(&mut self, s: &mut State) -> Result<u64, &'static str> {
        ui_engine_query::query(&s.memory)
    }
    fn resource(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        ui_indexed_records::lookup(&s.memory, Kind::Resource, index as u64)
    }
    fn settings(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        ui_indexed_records::lookup(&s.memory, Kind::Settings, index as u64)
    }
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
    fn putq(&mut self, p: u64, x: u64) -> Result<(), &'static str> {
        self.memory.write(p, &x.to_le_bytes())
    }
    fn set(&mut self, p: u64, x: u8) -> Result<(), &'static str> {
        self.memory.write(p, &[x])
    }
    fn mode(&mut self, x: u32) -> Result<(), &'static str> {
        self.memory.write(self.identity + 0x23c, &x.to_le_bytes())
    }
}
fn call(s: &mut State, h: &mut impl Services, f: u64, a: &[u64]) -> Result<u64, &'static str> {
    h.invoke(s, f, a)
}
fn has(s: &mut State, h: &mut impl Services, key: u64) -> Result<bool, &'static str> {
    Ok(call(s, h, 0x140d776a0, &[0x142f02ba8, key])? as u8 != 0)
}
/// Preserve forward sixteen-byte loads/stores for a potentially overlapping
/// provider result. The final in-object display copy is always disjoint.
fn copy_record(s: &mut State, dst: u64, src: u64) -> Result<(), &'static str> {
    for off in (0..0x1a0).step_by(16) {
        let b = s.memory.read(src + off, 16)?;
        s.memory.write(dst + off, &b)?;
    }
    let b = s.memory.read(src + 0x1a0, 8)?;
    s.memory.write(dst + 0x1a0, &b)
}
pub fn refresh(s: &mut State, h: &mut impl Services) -> Result<(), &'static str> {
    let o = s.identity;
    let sp = s.stack;
    let work = o + 0x230;
    if has(s, h, 0x1425de020)? || has(s, h, 0x1425de030)? {
        s.mode(3)?;
        call(s, h, 0x141b77b60, &[work, 0, 0])?;
    } else {
        let engine = h.engine(s)? as u8 != 0;
        let available = if engine && s.active_global == 0 {
            let resource = h.resource(s, s.resource_index)?;
            s.q(resource + 0x6188)? != 0
        } else {
            false
        };
        if available {
            call(s, h, 0x140e206c0, &[sp + 0x30])?;
            call(s, h, 0x141b75880, &[sp + 0x30])?;
            call(s, h, 0x1410d3a20, &[o, work, sp + 0x30])?;
            let record = h.settings(s, s.resource_index)?;
            let zero = call(s, h, 0x1417f12c0, &[0x1424f5648])? as u8 != 0;
            let value = if zero { 0 } else { s.q(record + 0x390)? };
            s.putq(o + 0x48, value)?;
            let record = h.settings(s, s.resource_index)?;
            let zero = call(s, h, 0x1417f12c0, &[0x1424f5648])? as u8 != 0;
            let value = if zero { 0 } else { s.q(record + 0x398)? };
            s.putq(o + 0x50, value)?;
        } else {
            let manager = call(s, h, 0x1414d16d0, &[])?;
            let parent = s.q(o + 0x3d8)?;
            let altitude = call(s, h, 0x14111d6a0, &[parent])? as u32;
            let record = call(s, h, 0x1414cdc80, &[manager, sp + 0x30, altitude as u64])?;
            copy_record(s, work, record)?;
            if has(s, h, 0x1426469a8)? {
                s.mode(7)?;
                s.set(o + 0x240, 0)?;
            } else if has(s, h, 0x1426469b8)? {
                s.mode(3)?;
            }
            let parent = s.q(o + 0x3d8)?;
            let context = call(s, h, 0x141126f60, &[parent, sp + 0x20])?;
            let b = s.memory.read(context, 16)?;
            s.memory.write(o + 0x48, &b)?;
        }
    }
    copy_record(s, o + 0x58, work)?;
    s.set(o + 0x18, 1)?;
    let head = s.q(o + 8)?;
    let mut current = s.q(head)?;
    let mut visits = 0;
    while current != s.q(o + 8)? {
        visits += 1;
        if visits > 100000 {
            return Err("unverified nonterminating weather listeners");
        }
        call(s, h, 0x1410c9d10, &[current + 16])?;
        current = s.q(current)?;
    }
    s.set(o + 0x18, 0)
}
pub fn refresh_image(
    image: &mut [u8],
    s: &mut State,
    h: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0x3e8 {
        return Err("weather image must be0x3e8 bytes");
    }
    s.memory.blocks.insert(s.identity, image.to_vec());
    let result = refresh(s, h);
    image.copy_from_slice(&s.memory.read(s.identity, 0x3e8)?);
    result
}
