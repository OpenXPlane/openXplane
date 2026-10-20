//! Whole equality141a3c910 and negated wrapper141a3cbc0. Derived-string uses
//! a recovered default; memcmp and deallocation remain explicit services.
use crate::ui_payload_move;
pub struct State {
    pub memory: ui_payload_move::State,
    pub stack: u64,
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn derived(&mut self, s: &mut State, record: u64, out: u64) -> Result<u64, &'static str> {
        let stack = s.stack;
        let mut memory = std::mem::take(&mut s.memory);
        let result = crate::ui_record_class_string::build_with(
            &mut memory,
            stack - 0xa0,
            record,
            out,
            |memory, f, args| {
                let mut caller = State {
                    memory: std::mem::take(memory),
                    stack,
                };
                let result = self.invoke(&mut caller, f, args);
                *memory = caller.memory;
                result
            },
        );
        s.memory = memory;
        result
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
    fn d(&self, p: u64) -> Result<u32, &'static str> {
        Ok(u32::from_le_bytes(
            self.memory.read(p, 4)?.try_into().unwrap(),
        ))
    }
}
fn strings(
    s: &mut State,
    h: &mut impl Services,
    left: u64,
    right: u64,
    first: bool,
) -> Result<bool, &'static str> {
    // The first comparison loads left length last. Later comparisons cache it
    // before reading either capacity; calls can change later fields.
    let length = if first { None } else { Some(s.q(left + 16)?) };
    let left_data = if s.q(left + 24)? > 15 {
        s.q(left)?
    } else {
        left
    };
    let right_length = s.q(right + 16)?;
    let right_data = if s.q(right + 24)? > 15 {
        s.q(right)?
    } else {
        right
    };
    let left_length = match length {
        Some(n) => n,
        None => s.q(left + 16)?,
    };
    if right_length != left_length {
        return Ok(false);
    }
    Ok(h.invoke(s, 0x1423c0180, &[right_data, left_data, right_length])? as u32 == 0)
}
fn free(s: &mut State, h: &mut impl Services, string: u64) -> Result<(), &'static str> {
    let capacity = s.q(string + 24)?;
    if capacity > 15 {
        let mut bytes = capacity.wrapping_add(1);
        let data = s.q(string)?;
        let mut address = data;
        if bytes >= 4096 {
            bytes = bytes.wrapping_add(39);
            address = s.q(data.wrapping_sub(8))?;
            if data.wrapping_sub(address).wrapping_sub(8) > 31 {
                return Err("native invalid aligned comparison temporary unverified");
            }
        }
        h.invoke(s, 0x1422e7c2c, &[address, bytes])?;
    }
    Ok(())
}
pub fn equal(
    s: &mut State,
    h: &mut impl Services,
    left: u64,
    right: u64,
) -> Result<bool, &'static str> {
    let sp = s.stack;
    s.memory.write(sp + 0x70, &0u32.to_le_bytes())?;
    for off in [0, 0x20, 0x60] {
        if !strings(s, h, left + off, right + off, off == 0)? {
            return Ok(false);
        }
    }
    let a = h.derived(s, left, sp + 0x40)?;
    s.memory.write(sp + 0x70, &1u32.to_le_bytes())?;
    let b = h.derived(s, right, sp + 0x20)?;
    // Both return pointers may alias either physical temporary or caller data.
    let mut result = strings(s, h, a, b, false)?;
    if result {
        result = s.d(left + 0xa0)? == s.d(right + 0xa0)?;
    }
    if result {
        for off in [0xa8, 0x80, 0x108] {
            if !strings(s, h, left + off, right + off, false)? {
                result = false;
                break;
            }
        }
    }
    // The right physical temporary is reset even when it was inline. The left
    // physical temporary is only freed. Cache the result across both requests.
    free(s, h, sp + 0x20)?;
    s.putq(sp + 0x30, 0)?;
    s.putq(sp + 0x38, 15)?;
    s.memory.write(sp + 0x20, &[0])?;
    free(s, h, sp + 0x40)?;
    Ok(result)
}
pub fn different(
    s: &mut State,
    h: &mut impl Services,
    left: u64,
    right: u64,
) -> Result<bool, &'static str> {
    Ok(!equal(s, h, left, right)?)
}

/// Share live caller memory without copying recorded child results into the body.
pub fn different_with<F>(
    memory: &mut ui_payload_move::State,
    stack: u64,
    left: u64,
    right: u64,
    invoke: F,
) -> Result<bool, &'static str>
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
    for (p, n) in [(stack + 0x20, 64usize), (stack + 0x70, 4)] {
        if memory.read(p, n).is_ok() {
            continue;
        }
        if memory
            .blocks
            .iter()
            .any(|(base, bytes)| *base < p + n as u64 && p < *base + bytes.len() as u64)
        {
            for p in created {
                memory.blocks.remove(&p);
            }
            return Err("partially mapped comparison scratch unverified");
        }
        memory.blocks.insert(p, vec![0; n]);
        created.push(p);
    }
    let mut state = State {
        memory: std::mem::take(memory),
        stack,
    };
    let result = different(&mut state, &mut Bridge(invoke), left, right);
    *memory = state.memory;
    for p in created {
        memory.blocks.remove(&p);
    }
    result
}

#[cfg(test)]
mod caller_defaults {
    use super::*;
    use crate::{ui_selection_finish as finish, ui_selection_initial as initial};
    const LEFT: u64 = 0x20000;
    const RIGHT: u64 = 0x30000;
    fn input() -> ui_payload_move::State {
        let mut memory = ui_payload_move::State::default();
        let mut record = vec![0; 0x198];
        for off in [0, 0x20, 0x40, 0x60, 0x80, 0xa8, 0xc8, 0xe8, 0x108] {
            record[off + 24..off + 32].copy_from_slice(&15u64.to_le_bytes());
        }
        let sentinel = 0x40000u64;
        record[0x180..0x188].copy_from_slice(&sentinel.to_le_bytes());
        let mut head = vec![0; 40];
        head[8..16].copy_from_slice(&sentinel.to_le_bytes());
        head[0x19] = 1;
        memory.blocks.insert(sentinel, head);
        memory.blocks.insert(LEFT, record.clone());
        memory.blocks.insert(RIGHT, record);
        memory
    }
    struct Host;
    fn provider(m: &mut ui_payload_move::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        match f {
            0x1423c0180 => {
                let result =
                    u64::from(m.read(a[0], a[2] as usize)? != m.read(a[1], a[2] as usize)?);
                m.write(LEFT + 0xa4, &[0x7e])?;
                Ok(result)
            }
            _ => Err("unexpected comparison service"),
        }
    }
    impl finish::Services for Host {
        fn invoke(
            &mut self,
            s: &mut finish::State,
            f: u64,
            a: &[u64],
        ) -> Result<u64, &'static str> {
            provider(&mut s.memory, f, a)
        }
    }
    impl initial::Services for Host {
        fn invoke(
            &mut self,
            s: &mut initial::State,
            f: u64,
            a: &[u64],
        ) -> Result<u64, &'static str> {
            provider(&mut s.memory, f, a)
        }
    }
    #[test]
    fn finish_default_runs_comparison_and_preserves_live_effects_after_scratch_expires() {
        let mut s = finish::State {
            memory: input(),
            identity: 0x10000,
            stack: 0x100000,
        };
        assert!(!finish::Services::different(&mut Host, &mut s, LEFT, RIGHT).unwrap());
        assert_eq!(s.memory.blocks.len(), 3);
        assert_eq!(s.memory.read(LEFT + 0xa4, 1).unwrap(), [0x7e]);
    }
    #[test]
    fn initial_default_runs_comparison_and_preserves_live_effects_after_scratch_expires() {
        let mut s = initial::State {
            memory: input(),
            identity: 0x10000,
            stack: 0x100000,
        };
        assert!(!initial::Services::different(&mut Host, &mut s, LEFT, RIGHT).unwrap());
        assert_eq!(s.memory.blocks.len(), 3);
        assert_eq!(s.memory.read(LEFT + 0xa4, 1).unwrap(), [0x7e]);
    }
}
