//! Whole reset14113a460 caller, including the tail request to finish14113d660.
//! Queries and reset gate use recovered defaults; provider/effect bodies remain services.
use crate::{ui_engine_query, ui_payload_move, ui_selection_reset_gate as gate};
pub const CATALOG_BEGIN: u64 = 0x145e4dab8;
pub const CATALOG_END: u64 = 0x145e4dac0;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    pub stack: u64,
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn secondary(&mut self, s: &mut State, receiver: u64) -> Result<u64, &'static str> {
        let mut memory = std::mem::take(&mut s.memory);
        let result = crate::ui_selection_secondary::with_dependencies(
            &mut memory,
            s.stack - 0x60,
            receiver,
            |memory, f, args| {
                std::mem::swap(&mut s.memory, memory);
                let result = self.invoke(s, f, args);
                std::mem::swap(&mut s.memory, memory);
                result
            },
        );
        s.memory = memory;
        result
    }

    fn initial_callback(&mut self, s: &mut State) -> Result<(), &'static str> {
        self.invoke(s, 0x1410fe0f0, &[s.identity + 0x38, 0x1e1b9, 0])?;
        Ok(())
    }
    fn skip_reset(&mut self, s: &mut State) -> Result<bool, &'static str> {
        let mut g = gate::State {
            identity: s.identity,
            memory: std::mem::take(&mut s.memory),
        };
        let mut bridge = GateBridge {
            host: self,
            stack: s.stack,
        };
        let result = gate::skip_reset(&mut g, &mut bridge);
        s.memory = g.memory;
        result
    }
}
struct GateBridge<'a, H: ?Sized> {
    host: &'a mut H,
    stack: u64,
}
impl<H: Services + ?Sized> GateBridge<'_, H> {
    fn call(
        &mut self,
        g: &mut gate::State,
        function: u64,
        args: &[u64],
    ) -> Result<u64, &'static str> {
        let mut s = State {
            identity: g.identity,
            memory: std::mem::take(&mut g.memory),
            stack: self.stack,
        };
        let result = self.host.invoke(&mut s, function, args);
        g.memory = s.memory;
        result
    }
}
impl<H: Services + ?Sized> gate::Services for GateBridge<'_, H> {
    fn compare_bytes(
        &mut self,
        g: &mut gate::State,
        left: u64,
        right: u64,
        bytes: u64,
    ) -> Result<i32, &'static str> {
        Ok(self.call(g, 0x1423c0180, &[left, right, bytes])? as i32)
    }
    fn secondary(&mut self, g: &mut gate::State, receiver: u64) -> Result<u64, &'static str> {
        let mut caller = State {
            memory: std::mem::take(&mut g.memory),
            identity: g.identity,
            stack: self.stack,
        };
        let result = self.host.secondary(&mut caller, receiver);
        g.memory = caller.memory;
        result
    }
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
    fn byte(&self, p: u64) -> Result<u8, &'static str> {
        Ok(self.memory.read(p, 1)?[0])
    }
    fn putq(&mut self, p: u64, v: u64) -> Result<(), &'static str> {
        self.memory.write(p, &v.to_le_bytes())
    }
}
fn free_string(s: &mut State, h: &mut impl Services, p: u64) -> Result<(), &'static str> {
    let cap = s.q(p + 24)?;
    if cap > 15 {
        let mut bytes = cap.checked_add(1).ok_or("invalid string capacity")?;
        let mut address = s.q(p)?;
        if bytes >= 4096 {
            let base = s.q(address.checked_sub(8).ok_or("invalid aligned pointer")?)?;
            let delta = address.checked_sub(base).ok_or("invalid aligned pointer")?;
            if !(8..=39).contains(&delta) {
                return Err("invalid aligned header");
            }
            address = base;
            bytes = bytes.checked_add(39).ok_or("invalid string capacity")?;
        }
        h.invoke(s, 0x1422e7c2c, &[address, bytes])?;
    }
    Ok(())
}
pub fn reset(s: &mut State, h: &mut impl Services, argument: u32) -> Result<(), &'static str> {
    let o = s.identity;
    let sp = s.stack;
    h.initial_callback(s)?;
    if !h.skip_reset(s)? {
        if ui_engine_query::query(&s.memory)? as u8 == 0
            || gate::variant_is_two_or_three(&s.memory, o + 0x50)? != 0
        {
            let a = sp + 0x20;
            let b = sp + 0x40;
            let v = sp + 0x60;
            s.memory.write(a, &[0; 16])?;
            s.putq(a + 16, 0)?;
            s.putq(a + 24, 15)?;
            s.memory.write(a, &[0])?;
            let present = h.invoke(s, 0x140d776a0, &[0x142f02ba8, 0x142643120])? as u8 != 0;
            let returned = if present {
                h.invoke(s, 0x140d77340, &[0x142f02ba8, b, 0x142643120])?
            } else {
                let manager = h.invoke(s, 0x1414cf5e0, &[])?;
                h.invoke(s, 0x1414cce40, &[manager, b])?
            };
            if returned != a {
                // Native uses two sequential16-byte transfers, not one atomic32-byte copy.
                for offset in [0, 16] {
                    let raw = s.memory.read(returned + offset, 16)?;
                    s.memory.write(a + offset, &raw)?;
                }
                s.putq(returned + 16, 0)?;
                s.putq(returned + 24, 15)?;
                s.memory.write(returned, &[0])?;
            }
            free_string(s, h, b)?;
            let copied = h.invoke(s, 0x1405dd5c0, &[b, a])?;
            let mut index = h.invoke(s, 0x14163b530, &[copied])? as u32 as i32;
            let end = s.q(CATALOG_END)?;
            let begin = s.q(CATALOG_BEGIN)?;
            // Original signed division truncates toward zero; its comparison
            // consumes low32 of count-1, without a range/alignment validation.
            let distance = end.wrapping_sub(begin) as i64;
            let last = ((distance / 168) as u32).wrapping_sub(1) as i32;
            if index < 0 || index > last {
                index = 0;
                let name = begin + 0x58;
                let mut n = 0;
                while s.byte(name + n)? != 0 {
                    n += 1;
                    if n > 0x100000 {
                        return Err("bounded catalog name required");
                    }
                }
                h.invoke(s, 0x1405df240, &[a, name, n])?;
            }
            let manager = h.invoke(s, 0x1414cf5e0, &[])?;
            let returned = h.invoke(s, 0x1414cd660, &[manager, v])?;
            s.putq(sp + 0xe0, o + 0x50)?;
            let selector = (s.byte(returned + 0x68)? as i8 as i64).wrapping_add(1) as u64;
            h.invoke(s, 0x1410d3690, &[selector, sp + 0xe0, returned])?;
            let tag = s.byte(v + 0x68)?;
            if tag > 4 && tag != 255 {
                return Err("invalid native variant tag unverified");
            }
            if tag == 0 && s.byte(v + 0x60)? != 0 {
                h.invoke(s, 0x1405de1d0, &[v + 0x20])?;
            }
            let record = s
                .q(CATALOG_BEGIN)?
                .wrapping_add((index as i64 as u64).wrapping_mul(168));
            h.invoke(s, 0x141146740, &[o, argument as u64, record])?;
            free_string(s, h, a)?;
        }
        s.memory.write(o + 0x18, &[1])?;
        let head = s.q(o + 8)?;
        let mut current = s.q(head)?;
        let mut visits = 0;
        while current != s.q(o + 8)? {
            visits += 1;
            if visits > 100000 {
                return Err("unverified nonterminating reset listeners");
            }
            h.invoke(s, 0x1410c9d10, &[current + 16])?;
            current = s.q(current)?;
        }
    }
    s.memory.write(o + 0x18, &[0])?;
    h.invoke(s, 0x14113d660, &[o])?;
    Ok(())
}
