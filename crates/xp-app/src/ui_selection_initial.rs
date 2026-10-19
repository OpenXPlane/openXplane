//! Whole selection callback1410fe0f0 for key0x1e1b9. Compare/float/finish,
//! cleanup, secondary predicate and recursive-reset bodies remain services.
use crate::ui_payload_move;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    pub stack: u64,
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
    fn float(&self, p: u64) -> Result<f32, &'static str> {
        Ok(f32::from_bits(u32::from_le_bytes(
            self.memory.read(p, 4)?.try_into().unwrap(),
        )))
    }
}
/// Original1417fb370: its callers consume AL only; upper RAX is unspecified here.
pub fn tag_is_three(memory: &ui_payload_move::State, receiver: u64) -> Result<bool, &'static str> {
    Ok(memory.read(receiver.wrapping_add(0x68), 1)?[0] == 3)
}
/// Original1417f8940: return the receiver for tag3, otherwise full RAX zero.
pub fn payload_if_three(
    memory: &ui_payload_move::State,
    receiver: u64,
) -> Result<u64, &'static str> {
    Ok(if tag_is_three(memory, receiver)? {
        receiver
    } else {
        0
    })
}
pub trait Services {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str>;
    fn tag_is_three(&mut self, s: &mut State, receiver: u64) -> Result<bool, &'static str> {
        tag_is_three(&s.memory, receiver)
    }
    fn payload(&mut self, s: &mut State, receiver: u64) -> Result<u64, &'static str> {
        payload_if_three(&s.memory, receiver)
    }
}
pub fn callback(s: &mut State, h: &mut impl Services, key: u64) -> Result<(), &'static str> {
    if key != 0x1e1b9 {
        return Ok(());
    }
    let r = s.identity + 0x38;
    let sp = s.stack;
    let parent = s.q(r + 0x90)?;
    if parent != 0 {
        let different = h.invoke(s, 0x141a3cbc0, &[r + 0xf8, parent + 0x20])? as u8 != 0;
        let changed = if different {
            true
        } else {
            let live_parent = s.q(r + 0x90)?;
            let value = h.invoke(
                s,
                0x1411363f0,
                &[live_parent.wrapping_add(0x268), sp + 0xa8],
            )?;
            // UCOMISS unordered also takes the change path; signed zero compares equal.
            s.float(r + 0x290)? != s.float(value)?
        };
        if changed {
            let begin = s.q(r + 0xd0)?;
            if begin != s.q(r + 0xd8)? {
                s.memory.write(r + 0xd8, &begin.to_le_bytes())?;
            }
            h.invoke(s, 0x14113d660, &[s.identity])?;
        }
    }
    let parent = s.q(r + 0x90)?;
    if parent != 0
        && h.invoke(s, 0x141a3cbc0, &[r + 0xf8, parent + 0x20])? as u8 != 0
        && h.tag_is_three(s, r + 0x18)?
    {
        let p = h.payload(s, r + 0x18)?;
        let third = s.memory.read(p + 0x20, 16)?;
        let mut tail = s.memory.read(p + 0x30, 8)?;
        let first = s.memory.read(p, 16)?;
        let second = s.memory.read(p + 0x10, 16)?;
        s.memory.write(sp + 0x40, &third)?;
        s.memory.write(sp + 0x48, &2u32.to_le_bytes())?;
        s.memory.write(sp + 0x50, &tail)?;
        s.memory.write(sp + 0x40, &0u64.to_le_bytes())?;
        if tail[0] != 1 {
            tail[0] = 1;
            s.memory.write(sp + 0x50, &[1])?;
        }
        let tag = s.byte(r + 0x80)?;
        if tag != 3 {
            if tag > 4 && tag != 255 {
                return Err("invalid native variant jump tag unverified");
            }
            if tag == 0 && s.byte(r + 0x78)? != 0 {
                h.invoke(s, 0x1405de1d0, &[r + 0x38])?;
            }
        }
        // Nonvolatile XMM7/8/6 retain first/second/tail across cleanup.
        // Native reloads the third word from local scratch after that dependency.
        let third = s.memory.read(sp + 0x40, 16)?;
        s.memory.write(r + 0x18, &first)?;
        s.memory.write(r + 0x28, &second)?;
        s.memory.write(r + 0x38, &third)?;
        s.memory.write(r + 0x48, &tail)?;
        if tag != 3 {
            s.memory.write(r + 0x80, &[3])?;
        }
    }
    if s.byte(r + 0x88)? != 0 {
        let parent = s.q(r + 0x90)?;
        if h.invoke(s, 0x14113ff10, &[parent])? as u8 != 0 {
            s.memory.write(s.identity + 0xc0, &[0])?;
            h.invoke(s, 0x14113a460, &[s.identity, 1])?;
        }
    }
    Ok(())
}
