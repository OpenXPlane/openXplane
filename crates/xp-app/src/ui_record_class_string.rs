//! Whole class-string141a3df30 and class-label141a3cd30. Successful allocation,
//! append/growth/free remain services; reached resize uses its recovered shrink branch.
use crate::ui_payload_move;
pub struct State {
    pub memory: ui_payload_move::State,
    pub stack: u64,
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn label(&mut self, s: &mut State, out: u64, index: u64) -> Result<u64, &'static str> {
        label(s, self, out, index)
    }
    fn shrink(&mut self, s: &mut State, out: u64, length: u64) -> Result<(), &'static str> {
        shrink(&mut s.memory, out, length)
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
    fn flag(&mut self, v: u32) -> Result<(), &'static str> {
        self.memory.write(self.stack + 0x30, &v.to_le_bytes())
    }
}
pub fn label<H: Services + ?Sized>(
    s: &mut State,
    h: &mut H,
    out: u64,
    index: u64,
) -> Result<u64, &'static str> {
    let text: &[u8] = match index as u32 {
        0 => b"Airliner",
        1 => b"General Aviation",
        2 => b"Glider",
        3 => b"Cargo",
        4 => b"Experimental",
        5 => b"Helicopter",
        6 => b"Military",
        7 => b"Seaplane",
        8 => b"Science Fiction",
        9 => b"VTOL",
        10 => b"Ultralight",
        11 => b"Favorites",
        _ => b"Unclassified",
    };
    s.memory.write(out, &[0; 16])?;
    let data = if text.len() > 15 {
        s.putq(out + 16, 0)?;
        s.putq(out + 24, 0)?;
        let p = h.invoke(s, 0x1422e7bf0, &[32])?;
        s.putq(out, p)?;
        s.putq(out + 16, 16)?;
        s.putq(out + 24, 31)?;
        p
    } else {
        s.putq(out + 16, text.len() as u64)?;
        s.putq(out + 24, 15)?;
        out
    };
    s.memory.write(data, text)?;
    s.memory.write(data + text.len() as u64, &[0])?;
    Ok(out)
}
/// Only the non-growing branch reached by the final backward slash search.
pub fn shrink(
    memory: &mut ui_payload_move::State,
    out: u64,
    length: u64,
) -> Result<(), &'static str> {
    let previous = u64::from_le_bytes(memory.read(out + 16, 8)?.try_into().unwrap());
    if length > previous {
        return Err("growing class-string resize unverified");
    }
    memory.write(out + 16, &length.to_le_bytes())?;
    let cap = u64::from_le_bytes(memory.read(out + 24, 8)?.try_into().unwrap());
    let data = if cap > 15 {
        u64::from_le_bytes(memory.read(out, 8)?.try_into().unwrap())
    } else {
        out
    };
    memory.write(data.wrapping_add(length), &[0])
}
fn free(s: &mut State, h: &mut impl Services, string: u64) -> Result<(), &'static str> {
    let cap = s.q(string + 24)?;
    if cap > 15 {
        let mut bytes = cap.wrapping_add(1);
        let data = s.q(string)?;
        let mut address = data;
        if bytes >= 4096 {
            bytes = bytes.wrapping_add(39);
            address = s.q(data.wrapping_sub(8))?;
            if data.wrapping_sub(address).wrapping_sub(8) > 31 {
                return Err("invalid class-string aligned temporary unverified");
            }
        }
        h.invoke(s, 0x1422e7c2c, &[address, bytes])?;
    }
    Ok(())
}
pub fn build(
    s: &mut State,
    h: &mut impl Services,
    record: u64,
    out: u64,
) -> Result<u64, &'static str> {
    let sp = s.stack;
    // The original saves RDX here before initializing the output, then consumes
    // the low byte of that live home slot as a cached growth argument.
    s.putq(sp + 0xa8, out)?;
    s.flag(0)?;
    s.memory.write(out, &[0; 16])?;
    s.putq(out + 16, 0)?;
    s.putq(out + 24, 15)?;
    s.memory.write(out, &[0])?;
    s.flag(1)?;
    let selector = s.byte(sp + 0xa8)?;
    let mut flags = 1u32;
    for index in 0..12i32 {
        let mut candidate = s.q(record + 0x180)?;
        let mut node = s.q(candidate + 8)?;
        let mut visits = 0;
        while s.byte(node + 0x19)? == 0 {
            visits += 1;
            if visits > 100000 {
                return Err("cyclic class-string tree unverified");
            }
            if (s.d(node + 0x1c)? as i32) < index {
                node = s.q(node + 0x10)?;
            } else {
                candidate = node;
                node = s.q(node)?;
            }
        }
        if s.byte(candidate + 0x19)? != 0 || index < (s.d(candidate + 0x1c)? as i32) || index == 11
        {
            continue;
        }
        let mut p = h.label(s, sp + 0x58, index as u64)?;
        let length = s.q(p + 16)?;
        let capacity = s.q(p + 24)?;
        if capacity.wrapping_sub(length) >= 1 {
            s.putq(p + 16, length.wrapping_add(1))?;
            let data = if capacity > 15 { s.q(p)? } else { p };
            s.memory.write(data.wrapping_add(length), &[b'/', 0])?;
        } else {
            s.putq(sp + 0x20, 1)?;
            p = h.invoke(s, 0x1405eca90, &[p, 1, selector as u64, 0x142505110, 1])?;
        }
        // Preserve the two native XMM loads/stores in order, including overlap
        // between a provider-returned descriptor and the physical destination.
        let first = s.memory.read(p, 16)?;
        s.memory.write(sp + 0x38, &first)?;
        let second = s.memory.read(p + 16, 16)?;
        s.memory.write(sp + 0x48, &second)?;
        let mut raw = first;
        raw.extend_from_slice(&second);
        s.putq(p + 16, 0)?;
        s.putq(p + 24, 15)?;
        s.memory.write(p, &[0])?;
        flags |= 2;
        s.flag(flags)?;
        let capacity = u64::from_le_bytes(raw[24..32].try_into().unwrap());
        let length = u64::from_le_bytes(raw[16..24].try_into().unwrap());
        let data = if capacity > 15 {
            u64::from_le_bytes(raw[..8].try_into().unwrap())
        } else {
            sp + 0x38
        };
        h.invoke(s, 0x140624040, &[out, data, length])?;
        free(s, h, sp + 0x38)?;
        free(s, h, sp + 0x58)?;
    }
    let length = s.q(out + 16)?;
    let data = if s.q(out + 24)? > 15 { s.q(out)? } else { out };
    if length != 0 {
        let mut current = data.wrapping_add(length - 1);
        let mut visits = 0;
        loop {
            visits += 1;
            if visits > 0x100000 {
                return Err("unbounded class-string slash search unverified");
            }
            if s.byte(current)? == b'/' {
                h.shrink(s, out, current.wrapping_sub(data))?;
                break;
            }
            if current == data {
                break;
            }
            current = current.wrapping_sub(1);
        }
    }
    Ok(out)
}
/// Introduce only missing scratch; keep inherited maps and live dependency effects.
pub fn build_with<F>(
    memory: &mut ui_payload_move::State,
    stack: u64,
    record: u64,
    out: u64,
    invoke: F,
) -> Result<u64, &'static str>
where
    F: FnMut(&mut ui_payload_move::State, u64, &[u64]) -> Result<u64, &'static str>,
{
    struct Bridge<F>(F);
    impl<F> Services for Bridge<F>
    where
        F: FnMut(&mut ui_payload_move::State, u64, &[u64]) -> Result<u64, &'static str>,
    {
        fn invoke(&mut self, s: &mut State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
            (self.0)(&mut s.memory, f, a)
        }
    }
    let mut created = Vec::new();
    for (p, n) in [(stack + 0x20, 0x58usize), (stack + 0xa8, 8)] {
        if memory.read(p, n).is_ok() {
            continue;
        }
        if memory
            .blocks
            .iter()
            .any(|(b, v)| *b < p + n as u64 && p < *b + v.len() as u64)
        {
            for p in created {
                memory.blocks.remove(&p);
            }
            return Err("partially mapped class-string scratch unverified");
        }
        memory.blocks.insert(p, vec![0; n]);
        created.push(p);
    }
    let mut s = State {
        memory: std::mem::take(memory),
        stack,
    };
    let result = build(&mut s, &mut Bridge(invoke), record, out);
    *memory = s.memory;
    for p in created {
        memory.blocks.remove(&p);
    }
    result
}
