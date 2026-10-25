//! Selection mode1414cedd0, header1411222e0 and singleton1414cc7f0.
//! Mapped normal-return profile; settings/catalog production and allocation are services.
use crate::{ui_payload_move, ui_record_class_membership};

pub const SINGLETON: u64 = 0x145a12340;
pub struct State {
    pub memory: ui_payload_move::State,
    /// RSP after the selected function's prologue, excluding saved registers.
    pub stack: u64,
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn member(&mut self, s: &mut State, record: u64, index: u64) -> Result<u64, &'static str> {
        struct Bridge<'a, H: ?Sized>(&'a mut H, u64);
        impl<H: Services + ?Sized> ui_record_class_membership::Services for Bridge<'_, H> {
            fn invoke(
                &mut self,
                m: &mut ui_payload_move::State,
                f: u64,
                a: &[u64],
            ) -> Result<u64, &'static str> {
                let mut s = State {
                    memory: std::mem::take(m),
                    stack: self.1,
                };
                let r = self.0.invoke(&mut s, f, a);
                *m = s.memory;
                r
            }
        }
        ui_record_class_membership::contains(
            &mut s.memory,
            &mut Bridge(self, s.stack),
            record,
            index,
        )
    }
    fn singleton(&mut self, s: &mut State) -> Result<u64, &'static str> {
        singleton(s, self)
    }
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
    fn putq(&mut self, p: u64, v: u64) -> Result<(), &'static str> {
        self.memory.write(p, &v.to_le_bytes())
    }
    fn literal(&mut self, p: u64, text: &[u8]) -> Result<(), &'static str> {
        self.memory.write(p, &[0; 16])?;
        self.putq(p + 16, text.len() as u64)?;
        self.putq(p + 24, 15)?;
        self.memory.write(p, text)
    }
}
pub fn singleton<H: Services + ?Sized>(s: &mut State, h: &mut H) -> Result<u64, &'static str> {
    let cached = s.q(SINGLETON)?;
    if cached != 0 {
        return Ok(cached);
    }
    // Native stores even a NULL result; there is no constructor or retry here.
    let value = h.invoke(s, 0x1422e7bf0, &[1])?;
    s.putq(SINGLETON, value)?;
    Ok(value)
}
fn free<H: Services + ?Sized>(s: &mut State, h: &mut H, string: u64) -> Result<(), &'static str> {
    let cap = s.q(string + 24)?;
    if cap > 15 {
        let mut bytes = cap.wrapping_add(1);
        let data = s.q(string)?;
        let mut address = data;
        if bytes >= 4096 {
            bytes = bytes.wrapping_add(39);
            address = s.q(data.wrapping_sub(8))?;
            if data.wrapping_sub(address).wrapping_sub(8) > 31 {
                return Err("native invalid aligned selection temporary unverified");
            }
        }
        h.invoke(s, 0x1422e7c2c, &[address, bytes])?;
    }
    Ok(())
}
fn reset(s: &mut State, string: u64) -> Result<(), &'static str> {
    s.putq(string + 16, 0)?;
    s.putq(string + 24, 15)?;
    s.memory.write(string, &[0])
}
///1414cedd0 ignores its receiver and returns a zero-extended Boolean.
pub fn mode(s: &mut State, h: &mut impl Services) -> Result<u64, &'static str> {
    let settings = h.invoke(s, 0x1414d45e0, &[])?;
    let sp = s.stack;
    s.literal(sp + 0x40, b"1")?;
    s.literal(sp + 0x20, b"_glider_tow")?;
    let result = h.invoke(s, 0x1414d45f0, &[settings, sp + 0x60, sp + 0x20, sp + 0x40])?;
    let length = s.q(result + 16)?;
    let data = if s.q(result + 24)? > 15 {
        s.q(result)?
    } else {
        result
    };
    let yes = length == 1 && h.invoke(s, 0x1423c0180, &[data, 0x1424ff288, 1])? as u32 == 0;
    // The returned descriptor may alias another object. Cleanup still uses +60.
    free(s, h, sp + 0x60)?;
    reset(s, sp + 0x60)?;
    free(s, h, sp + 0x20)?;
    reset(s, sp + 0x20)?;
    free(s, h, sp + 0x40)?;
    Ok(u64::from(yes))
}
///1411222e0 returns the selected catalog string or zero for non-glider records.
pub fn header(s: &mut State, h: &mut impl Services, receiver: u64) -> Result<u64, &'static str> {
    let record = receiver.wrapping_add(0x20);
    if h.member(s, record, 2)? as u8 == 0 {
        return Ok(0);
    }
    let sp = s.stack;
    let single = h.singleton(s)?;
    h.invoke(s, 0x1414ccc80, &[single, sp + 0x30, record])?;
    let catalog = h.invoke(s, 0x1417e9f50, &[])?;
    let mut result = h.invoke(s, 0x1417e8a60, &[catalog, sp + 0x30])?;
    for (text, length) in [(0x142642f40, 21), (0x142642f58, 14)] {
        if result != 0 {
            break;
        }
        let catalog = h.invoke(s, 0x1417e9f50, &[])?;
        s.putq(sp + 0x20, text)?;
        s.putq(sp + 0x28, length)?;
        result = h.invoke(s, 0x1417e89c0, &[catalog, sp + 0x20])?;
    }
    if result == 0 {
        let catalog = h.invoke(s, 0x1417e9f50, &[])?;
        result = h.invoke(s, 0x1417e9a40, &[catalog, 0])?;
    }
    free(s, h, sp + 0x30)?;
    Ok(result)
}
