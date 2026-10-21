//! Original append140624040 and growth1405eca90 normal callers. Allocation,
//! overlap-safe move and sized free remain services; exceptional exits unverified.
use crate::ui_payload_move;
pub struct State {
    pub memory: ui_payload_move::State,
    pub stack: u64,
}
#[derive(Clone, Copy, Debug)]
pub enum Operation {
    Append {
        object: u64,
        source: u64,
        count: u64,
    },
    Grow {
        object: u64,
        amount: u64,
        opaque: u64,
        source: u64,
        count: u64,
    },
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn grow(
        &mut self,
        s: &mut State,
        object: u64,
        amount: u64,
        opaque: u64,
        source: u64,
        count: u64,
    ) -> Result<u64, &'static str> {
        grow(s, self, object, amount, opaque, source, count)
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
}
pub fn append(
    s: &mut State,
    h: &mut impl Services,
    object: u64,
    source: u64,
    count: u64,
) -> Result<u64, &'static str> {
    let old_length = s.q(object + 16)?;
    let capacity = s.q(object + 24)?;
    if count <= capacity.wrapping_sub(old_length) {
        s.putq(object + 16, old_length.wrapping_add(count))?;
        let data = if capacity > 15 { s.q(object)? } else { object };
        let destination = data.wrapping_add(old_length);
        h.invoke(s, 0x1423c0280, &[destination, source, count])?;
        s.memory.write(destination.wrapping_add(count), &[0])?;
        Ok(object)
    } else {
        let opaque = s.memory.read(s.stack + 0x40, 1)?[0];
        s.putq(s.stack + 0x20, count)?;
        h.grow(s, object, count, opaque as u64, source, count)
    }
}
pub fn grow<H: Services + ?Sized>(
    s: &mut State,
    h: &mut H,
    object: u64,
    amount: u64,
    _opaque: u64,
    source: u64,
    count: u64,
) -> Result<u64, &'static str> {
    let old_length = s.q(object + 16)?;
    let maximum = i64::MAX as u64;
    if maximum.wrapping_sub(old_length) < amount {
        return Err("native string length error branch unverified");
    }
    let old_capacity = s.q(object + 24)?;
    let new_length = old_length.wrapping_add(amount);
    let rounded = new_length | 15;
    let half = old_capacity >> 1;
    let capacity = if rounded > maximum || old_capacity > maximum.wrapping_sub(half) {
        maximum
    } else {
        rounded.max(old_capacity.wrapping_add(half))
    };
    let bytes = capacity.wrapping_add(1);
    let data = if bytes >= 4096 {
        let requested = bytes.wrapping_add(39);
        if requested <= bytes {
            return Err("native string allocation overflow branch unverified");
        }
        let base = h.invoke(s, 0x1422e7bf0, &[requested])?;
        if base == 0 {
            return Err("native large string null allocation branch unverified");
        }
        let aligned = base.wrapping_add(39) & !31;
        s.putq(aligned.wrapping_sub(8), base)?;
        aligned
    } else if bytes != 0 {
        h.invoke(s, 0x1422e7bf0, &[bytes])?
    } else {
        0
    };
    // Pointer publication is deliberately last. Old-capacity/length are cached,
    // but a heap pointer is loaded after the allocation and metadata stores.
    s.putq(object + 16, new_length)?;
    let tail = data.wrapping_add(old_length);
    s.putq(object + 24, capacity)?;
    if old_capacity > 15 {
        let old = s.q(object)?;
        h.invoke(s, 0x1423c0280, &[data, old, old_length])?;
        h.invoke(s, 0x1423c0280, &[tail, source, count])?;
        s.memory.write(tail.wrapping_add(count), &[0])?;
        let mut address = old;
        let mut released = old_capacity.wrapping_add(1);
        if released >= 4096 {
            address = s.q(old.wrapping_sub(8))?;
            released = released.wrapping_add(39);
            if old.wrapping_sub(address).wrapping_sub(8) > 31 {
                return Err("native string aligned-free failure unverified");
            }
        }
        h.invoke(s, 0x1422e7c2c, &[address, released])?;
    } else {
        h.invoke(s, 0x1423c0280, &[data, object, old_length])?;
        h.invoke(s, 0x1423c0280, &[tail, source, count])?;
        s.memory.write(tail.wrapping_add(count), &[0])?;
    }
    s.putq(object, data)?;
    Ok(object)
}
pub fn with_memory<F>(
    memory: &mut ui_payload_move::State,
    stack: u64,
    operation: Operation,
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
    if matches!(operation, Operation::Append { .. }) {
        for (p, n) in [(stack + 0x20, 8usize), (stack + 0x40, 1)] {
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
                return Err("partially mapped append scratch unverified");
            }
            memory.blocks.insert(p, vec![0; n]);
            created.push(p);
        }
    }
    let mut s = State {
        memory: std::mem::take(memory),
        stack,
    };
    let mut h = Bridge(invoke);
    let result = match operation {
        Operation::Append {
            object,
            source,
            count,
        } => append(&mut s, &mut h, object, source, count),
        Operation::Grow {
            object,
            amount,
            opaque,
            source,
            count,
        } => grow(&mut s, &mut h, object, amount, opaque, source, count),
    };
    *memory = s.memory;
    for p in created {
        memory.blocks.remove(&p);
    }
    result
}
