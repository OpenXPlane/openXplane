//! Whole finish14113d660 caller. Record/config/math/output/growth bodies remain
//! services; indexed resource access and selection gate use recovered defaults.
use crate::{
    ui_indexed_records::{self, Kind},
    ui_payload_move, ui_selection_reset_gate as gate,
};
pub const DENOMINATOR: u64 = 0x142650450;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    pub stack: u64,
}
#[derive(Clone, Copy, Debug)]
pub enum ScalarOp {
    Multiply,
    Divide,
    Add,
}
pub trait Services {
    /// Float operands/results are raw low32 words, not converted host doubles.
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn scalar(&mut self, a: u32, b: u32, op: ScalarOp) -> u32 {
        scalar_sse(a, b, op)
    }
    fn resource(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        ui_indexed_records::lookup(&s.memory, Kind::Resource, index as u64)
    }
    fn gate(&mut self, s: &mut State) -> Result<bool, &'static str> {
        let mut g = gate::State {
            memory: std::mem::take(&mut s.memory),
            identity: s.identity,
        };
        let result = gate::skip_reset(
            &mut g,
            &mut GateBridge {
                host: self,
                stack: s.stack,
            },
        );
        s.memory = g.memory;
        result
    }
}
struct GateBridge<'a, H: ?Sized> {
    host: &'a mut H,
    stack: u64,
}
impl<H: Services + ?Sized> GateBridge<'_, H> {
    fn call(&mut self, g: &mut gate::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        let mut s = State {
            memory: std::mem::take(&mut g.memory),
            identity: g.identity,
            stack: self.stack,
        };
        let result = self.host.invoke(&mut s, f, a);
        g.memory = s.memory;
        result
    }
}
impl<H: Services + ?Sized> gate::Services for GateBridge<'_, H> {
    fn compare_bytes(
        &mut self,
        g: &mut gate::State,
        l: u64,
        r: u64,
        n: u64,
    ) -> Result<i32, &'static str> {
        Ok(self.call(g, 0x1423c0180, &[l, r, n])? as i32)
    }
    fn secondary(&mut self, g: &mut gate::State, p: u64) -> Result<u64, &'static str> {
        self.call(g, 0x14113ff10, &[p])
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
    fn putq(&mut self, p: u64, v: u64) -> Result<(), &'static str> {
        self.memory.write(p, &v.to_le_bytes())
    }
    fn putd(&mut self, p: u64, v: u32) -> Result<(), &'static str> {
        self.memory.write(p, &v.to_le_bytes())
    }
}
fn nan(a: u32) -> bool {
    a & 0x7f800000 == 0x7f800000 && a & 0x7fffff != 0
}
/// Legacy SSE operand priority. FP exception flags and non-default MXCSR modes
/// are outside this value-only model; native x86 tests are an independent oracle.
pub fn scalar_sse(a: u32, b: u32, op: ScalarOp) -> u32 {
    if nan(a) {
        return a | 0x400000;
    }
    if nan(b) {
        return b | 0x400000;
    }
    let (a, b) = (f32::from_bits(a), f32::from_bits(b));
    let r = match op {
        ScalarOp::Multiply => a * b,
        ScalarOp::Divide => a / b,
        ScalarOp::Add => a + b,
    };
    if r.is_nan() { 0xffc00000 } else { r.to_bits() }
}
fn clear_end(s: &mut State, p: u64) -> Result<(), &'static str> {
    let begin = s.q(p)?;
    if begin != s.q(p + 8)? {
        s.putq(p + 8, begin)?;
    }
    Ok(())
}
fn parent_record(s: &mut State, h: &mut impl Services, d: u64) -> Result<(), &'static str> {
    let parent = s.q(s.identity + 0xc8)?;
    if parent != 0 {
        h.invoke(s, 0x1410d6520, &[d, parent + 0x20])?;
    } else {
        h.invoke(s, 0x141a3c750, &[d])?;
    }
    Ok(())
}
pub fn finish(s: &mut State, h: &mut impl Services) -> Result<(), &'static str> {
    let o = s.identity;
    let sp = s.stack;
    let primary = sp + 0x1f0;
    let temporary = sp + 0x50;
    let work = sp + 0x390;
    let token = h.invoke(s, 0x1417f9060, &[o + 0x50])?;
    s.putq(sp + 0x658, token)?;
    parent_record(s, h, primary)?;
    clear_end(s, o + 0xd8)?;
    clear_end(s, o + 0xf0)?;
    s.putd(o + 0x128, 0)?;
    let manager = h.invoke(s, 0x1414cf5e0, &[])?;
    let ready = h.invoke(s, 0x1406d4ba0, &[manager])? as u8 != 0;
    if ready {
        let different = h.invoke(s, 0x141a3cbc0, &[primary, o + 0x130])? as u8 != 0;
        let update = if different {
            true
        } else {
            let parent = s.q(o + 0xc8)?;
            let p = h.invoke(s, 0x1411363f0, &[parent.wrapping_add(0x268), sp + 0x650])?;
            f32::from_bits(s.d(p)?) != f32::from_bits(s.d(o + 0x2c8)?)
        };
        if update {
            clear_end(s, o + 0x108)?;
            h.invoke(s, 0x1410fa970, &[o + 0x130, primary])?;
            let parent = s.q(o + 0xc8)?;
            let p = h.invoke(s, 0x1411363f0, &[parent.wrapping_add(0x268), sp + 0x650])?;
            let bits = s.d(p)?;
            s.putd(o + 0x2c8, bits)?;
        }
        if s.q(o + 0x108)? == s.q(o + 0x110)? {
            let property = s.q(o + 0x120)?;
            h.invoke(s, 0x140c8ef70, &[property, 0xffffffff])?;
            parent_record(s, h, temporary)?;
            h.invoke(s, 0x141100d80, &[o, temporary])?;
        }
    } else {
        clear_end(s, o + 0x108)?;
        let initialized = h.invoke(s, 0x141a3c750, &[temporary])?;
        h.invoke(s, 0x1405de3d0, &[o + 0x130, initialized])?;
        h.invoke(s, 0x1405ddde0, &[temporary])?;
        s.putd(o + 0x2c8, 0)?;
        let vector = o + 0x108;
        let mut insertion = s.q(vector)?;
        s.putq(sp + 0x40, vector)?;
        s.putq(sp + 0x48, insertion)?;
        let end = s.q(0x145e4dac0)?;
        let mut record = s.q(0x145e4dab8)?;
        let cached_vector = s.q(sp + 0x40)?;
        let mut visits = 0;
        while record != end {
            visits += 1;
            if visits > 100000 {
                return Err("unverified nonterminating finish catalog");
            }
            let checksum = s.d(o + 0x128)? ^ s.d(record + 0x88)?;
            s.putd(o + 0x128, checksum)?;
            s.putq(sp + 0x650, record)?;
            let live_end = s.q(vector + 8)?;
            let capacity = s.q(vector + 16)?;
            let returned = if live_end != capacity {
                if insertion == live_end {
                    s.putq(live_end, record)?;
                    let live = s.q(vector + 8)?;
                    s.putq(vector + 8, live.wrapping_add(8))?;
                } else {
                    let last = live_end.wrapping_sub(8);
                    let item = s.q(last)?;
                    s.putq(live_end, item)?;
                    let live = s.q(vector + 8)?;
                    s.putq(vector + 8, live.wrapping_add(8))?;
                    let bytes = last.wrapping_sub(insertion);
                    let destination = live_end.wrapping_sub(bytes);
                    h.invoke(s, 0x1423c0280, &[destination, insertion, bytes])?;
                    s.putq(insertion, record)?;
                }
                insertion
            } else {
                h.invoke(s, 0x1405e6a10, &[cached_vector, insertion, sp + 0x650])?
            };
            insertion = returned.wrapping_add(8);
            record = record.wrapping_add(168);
        }
    }
    // The home slot is live until this load; later output calls reuse it as a float.
    let token = s.q(sp + 0x658)?;
    if token != 0 {
        h.invoke(s, 0x1423c0920, &[work, 0, 0x258])?;
        h.invoke(s, 0x141bae6d0, &[work])?;
        let parent = s.q(o + 0xd0)?;
        if parent != 0 {
            h.invoke(s, 0x141b9e840, &[work, parent + 0x230])?;
        }
        let mut final_bits = 0;
        let manager = h.invoke(s, 0x1414cf5e0, &[])?;
        if h.invoke(s, 0x1406d4ba0, &[manager])? as u8 != 0 {
            let parent = s.q(o + 0xc8)?;
            h.invoke(s, 0x1411363f0, &[parent.wrapping_add(0x268), sp + 0x660])?;
            let index = if h.gate(s)? { 0 } else { 20 };
            let base = s.d(token + 0x48)?;
            let delta = h.invoke(s, 0x141ba6df0, &[work, base as u64])? as u32;
            let value = h.scalar(delta, 0x41400000, ScalarOp::Multiply);
            let value = h.scalar(value, 0x437e0000, ScalarOp::Multiply);
            let value = h.scalar(value, 0x42c80000, ScalarOp::Divide);
            let value = h.scalar(value, 0x42c80000, ScalarOp::Divide);
            let value = h.scalar(value, base, ScalarOp::Add);
            let adjusted = h.invoke(s, 0x141ba63a0, &[work, value as u64])? as u32;
            let resource = h.resource(s, index)?;
            let amount = s.d(sp + 0x660)?;
            let converted = h.scalar(amount, 0x4b189680, ScalarOp::Multiply);
            let converted = h.scalar(converted, s.d(DENOMINATOR)?, ScalarOp::Divide);
            s.putd(sp + 0x20, adjusted)?;
            h.invoke(
                s,
                0x141a66550,
                &[
                    resource,
                    sp + 0x658,
                    base as u64,
                    converted as u64,
                    adjusted as u64,
                ],
            )?;
            let resource = h.resource(s, index)?;
            h.invoke(s, 0x141a664d0, &[resource, sp + 0x650, base as u64])?;
            let a = s.d(sp + 0x650)?;
            let b = s.d(sp + 0x658)?;
            final_bits = if f32::from_bits(a) > f32::from_bits(b) {
                a
            } else {
                b
            };
        }
        parent_record(s, h, temporary)?;
        s.putq(sp + 0x38, o + 0xf0)?;
        s.putq(sp + 0x30, o + 0xd8)?;
        s.putq(sp + 0x28, work)?;
        s.putq(sp + 0x20, token)?;
        h.invoke(
            s,
            0x14111da80,
            &[
                o,
                temporary,
                final_bits as u64,
                1,
                token,
                work,
                o + 0xd8,
                o + 0xf0,
            ],
        )?;
        h.invoke(s, 0x1405ddde0, &[temporary])?;
    }
    h.invoke(s, 0x1405ddde0, &[primary])?;
    Ok(())
}

#[cfg(all(test, target_arch = "x86_64"))]
mod native_scalar_tests {
    use super::{ScalarOp, scalar_sse};
    use core::arch::asm;
    struct Mxcsr(u32);
    impl Drop for Mxcsr {
        fn drop(&mut self) {
            unsafe {
                asm!("ldmxcsr [{p}]",p=in(reg)&self.0,options(nostack,preserves_flags));
            }
        }
    }
    fn oracle(a: u32, b: u32, op: ScalarOp) -> u32 {
        let result: u32;
        macro_rules! evaluate {($opcode:literal)=>{unsafe{asm!("movd xmm0, {a:e}","movd xmm1, {b:e}",concat!($opcode," xmm0, xmm1"),"movd {r:e}, xmm0",a=in(reg)a,b=in(reg)b,r=lateout(reg)result,out("xmm0")_,out("xmm1")_,options(nostack,preserves_flags));}};}
        match op {
            ScalarOp::Multiply => evaluate!("mulss"),
            ScalarOp::Divide => evaluate!("divss"),
            ScalarOp::Add => evaluate!("addss"),
        };
        result
    }
    #[test]
    fn raw_scalar_values_match_native_sse_with_masked_nearest_mxcsr() {
        let mut previous = 0u32;
        let nearest = 0x1f80u32;
        unsafe {
            asm!("stmxcsr [{p}]",p=in(reg)&mut previous,options(nostack,preserves_flags));
            asm!("ldmxcsr [{p}]",p=in(reg)&nearest,options(nostack,preserves_flags));
        }
        let _restore = Mxcsr(previous);
        let values: [u32; 22] = [
            0, 0x80000000, 0x3f800000, 0xbf800000, 0x7f800000, 0xff800000, 0x7fc12345, 0xffc54321,
            0x7fa00001, 0xffa12345, 1, 0x7f7fffff, 0x41400000, 0x437e0000, 0x42c80000, 0x4b189680,
            0x4ba83308, 0x3f9e0652, 0xc2f6e979, 0x3eaaaaab, 0x00800001, 0x4b000001,
        ];
        for op in [ScalarOp::Multiply, ScalarOp::Divide, ScalarOp::Add] {
            for a in values {
                for b in values {
                    assert_eq!(
                        scalar_sse(a, b, op),
                        oracle(a, b, op),
                        "{op:?} {a:08x} {b:08x}"
                    );
                }
            }
        }
    }
}
