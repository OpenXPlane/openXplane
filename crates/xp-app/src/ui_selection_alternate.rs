//! Whole selection alternate14111bb90. Provider operands retain native widths.
//! Settings address lookup is recovered; search/data/cleanup effects stay services.
use crate::{
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
    /// Floating operands are raw low XMM words; fifth/sixth operands are logical
    /// caller stack values. Function identities are data, never host pointers.
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
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
    fn d(&self, p: u64) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.memory.read(p, 4)?.try_into().unwrap(),
        ))
    }
    fn byte(&self, p: u64) -> Result<u8, &'static str> {
        Ok(self.memory.read(p, 1)?[0])
    }
    fn putq(&mut self, p: u64, v: u64) -> Result<(), &'static str> {
        self.memory.write(p, &v.to_le_bytes())
    }
    fn set(&mut self, p: u64, v: u8) -> Result<(), &'static str> {
        self.memory.write(p, &[v])
    }
}
fn call(s: &mut State, h: &mut impl Services, f: u64, a: &[u64]) -> Result<u64, &'static str> {
    h.invoke(s, f, a)
}
fn tag(s: &State) -> Result<u8, &'static str> {
    let v = s.byte(s.identity + 0xb8)?;
    if v <= 4 || v == 255 {
        Ok(v)
    } else {
        Err("native invalid variant jump unverified")
    }
}
fn metadata(s: &mut State, first: u64, second: u64, flag: u8) -> Result<(), &'static str> {
    let o = s.identity;
    s.putq(o + 0x50, first)?;
    s.putq(o + 0x58, second)?;
    s.set(o + 0x60, flag)?;
    let raw = s.memory.read(s.stack + 0x84, 4)?;
    s.memory.write(o + 0x64, &raw)?;
    s.memory.write(o + 0x68, &0u32.to_le_bytes())
}
/// Original140e20d50 tail-calls optional cleanup only when its trailing byte is set.
fn destroy_temp(s: &mut State, h: &mut impl Services, p: u64) -> Result<(), &'static str> {
    if s.byte(p + 0x60)? != 0 {
        call(s, h, 0x1405de1d0, &[p + 0x20])?;
    }
    Ok(())
}
pub fn alternate(s: &mut State, h: &mut impl Services) -> Result<(), &'static str> {
    let o = s.identity;
    let sp = s.stack;
    let t = sp + 0xe0;
    let settings = h.settings(s, s.resource_index)?;
    if s.d(settings + 0x24c)? != 0 {
        s.putq(sp + 0x630, 0)?;
        s.putq(sp + 0x638, 0)?;
        s.putq(sp + 0x48, 0)?;
        call(s, h, 0x1408aaf40, &[t, 0x142f2d7a8])?;
        if call(s, h, 0x1417fb230, &[t])? as u8 != 0 {
            let record = h.settings(s, s.resource_index)?;
            let zero = call(s, h, 0x1417f12c0, &[0x1424f5648])? as u8 != 0;
            let second = if zero { 0 } else { s.q(record + 0x398)? };
            let record = h.settings(s, s.resource_index)?;
            let zero = call(s, h, 0x1417f12c0, &[0x1424f5648])? as u8 != 0;
            let first = if zero { 0 } else { s.q(record + 0x390)? };
            call(s, h, 0x14163ffa0, &[sp + 0x30, first, second, 0x41f00000])?;
            let end = s.q(sp + 0x38)?;
            let begin = s.q(sp + 0x30)?;
            let count = (end.wrapping_sub(begin) as i64 >> 3) as u64;
            let live_begin = if count > 1 {
                let flag = s.byte(sp + 0x628)?;
                let filtered = call(s, h, 0x1410d4490, &[sp + 0x50, begin, end, flag as u64])?;
                let first = s.q(filtered)?;
                call(s, h, 0x1408ad0f0, &[sp + 0x30, sp + 0x58, first, end])?;
                let token = call(s, h, 0x1417f9060, &[t])?;
                s.putq(sp + 0x50, token)?;
                let begin = s.q(sp + 0x30)?;
                let end = s.q(sp + 0x38)?;
                let found = call(s, h, 0x140915da0, &[sp + 0x58, begin, end, sp + 0x50])?;
                let live_end = s.q(sp + 0x38)?;
                if s.q(found)? != live_end {
                    let token = call(s, h, 0x1417f9060, &[t])?;
                    s.putq(sp + 0x630, token)?;
                    let record = h.settings(s, s.resource_index)?;
                    s.set(sp + 0x28, 0)?;
                    s.putq(sp + 0x20, sp + 0x48)?;
                    call(
                        s,
                        h,
                        0x14163f830,
                        &[record, token, sp + 0x638, sp + 0x628, sp + 0x48, 0],
                    )?;
                }
                s.q(sp + 0x30)?
            } else {
                begin
            };
            if live_begin != 0 {
                let cap = s.q(sp + 0x40)?;
                let mut bytes = ((cap.wrapping_sub(live_begin) as i64 >> 3) as u64).wrapping_mul(8);
                let mut base = live_begin;
                if bytes >= 4096 {
                    base = s.q(live_begin.checked_sub(8).ok_or("bad vector header")?)?;
                    let delta = live_begin
                        .checked_sub(base)
                        .and_then(|d| d.checked_sub(8))
                        .ok_or("invalid vector alignment")?;
                    if delta > 31 {
                        return Err("invalid vector alignment");
                    }
                    bytes = bytes.wrapping_add(39);
                }
                call(s, h, 0x1422e7c2c, &[base, bytes])?;
            }
        }
        let mut first = s.q(sp + 0x630)?;
        if first == 0 {
            let record = h.settings(s, s.resource_index)?;
            s.putq(sp + 0x20, sp + 0x48)?;
            call(
                s,
                h,
                0x141640d80,
                &[record, sp + 0x630, sp + 0x638, sp + 0x628, sp + 0x48],
            )?;
            first = s.q(sp + 0x630)?;
        }
        let second = s.q(sp + 0x638)?;
        if second != 0 {
            s.putq(sp + 0x70, first)?;
            s.putq(sp + 0x78, second)?;
            let flag = u8::from(s.d(sp + 0x628)? != 0);
            s.set(sp + 0x80, flag)?;
            s.putq(sp + 0x84, 0)?;
            s.set(sp + 0xd0, 0)?;
            let old = tag(s)?;
            metadata(s, first, second, flag)?;
            if old == 0 {
                if s.byte(o + 0xb0)? != 0 {
                    call(s, h, 0x1405de1d0, &[o + 0x70])?;
                    s.set(o + 0xb0, 0)?;
                }
            } else {
                // All valid nonzero tags (including signed-1) jump directly to14111beb6.
                s.set(o + 0xb0, 0)?;
                s.set(o + 0xb8, 0)?;
            }
            destroy_temp(s, h, sp + 0x70)?;
        } else {
            let secondary = s.q(sp + 0x48)?;
            if secondary != 0 {
                s.putq(sp + 0x58, first)?;
                s.putq(sp + 0x60, secondary)?;
                call(s, h, 0x1410ca190, &[o + 0x50, sp + 0x58])?;
            } else {
                let record = call(s, h, 0x1417f63d0, &[sp + 0x58, s.resource_index as u64])?;
                let raw = s.memory.read(record, 16)?;
                s.memory.write(sp + 0x30, &raw)?;
                let raw = s.memory.read(record + 16, 8)?;
                s.memory.write(sp + 0x40, &raw)?;
                call(s, h, 0x1410ca0f0, &[o + 0x50, sp + 0x30])?;
            }
        }
        s.set(o + 0xc3, 0)?;
        call(s, h, 0x1408abc10, &[t])?;
    } else {
        let record = call(s, h, 0x1417f6240, &[sp + 0x70, s.resource_index as u64])?;
        let raw = s.memory.read(record, 56)?;
        let old = tag(s)?;
        if old != 3 {
            if old == 0 && s.byte(o + 0xb0)? != 0 {
                call(s, h, 0x1405de1d0, &[o + 0x70])?;
            }
            s.memory.write(o + 0x50, &raw)?;
            s.set(o + 0xb8, 3)?;
        } else {
            s.memory.write(o + 0x50, &raw)?;
        }
    }
    s.memory.write(o + 0xc0, &[1, 0, 0])
}
pub fn alternate_image(
    image: &mut [u8],
    s: &mut State,
    h: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0x2d0 {
        return Err("selection image must be0x2d0 bytes");
    }
    s.memory.blocks.insert(s.identity, image.to_vec());
    let result = alternate(s, h);
    image.copy_from_slice(&s.memory.read(s.identity, 0x2d0)?);
    result
}
