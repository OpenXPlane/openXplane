//! Root refresh141138cf0. Version-bound leaf identities are data, not host calls.
use crate::{ui_callback_links, ui_callback_list, ui_payload_move};
use std::collections::BTreeMap;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
    pub stack: u64,
    pub active_global: u64,
    pub resource_index: u32,
}
pub struct Context<'a> {
    pub callbacks: &'a mut ui_callback_links::State,
    pub parents: &'a mut BTreeMap<u64, ui_callback_list::List>,
}
/// Arguments use recovered logical widths;141141de0 third argument is R8B.
pub trait Services: ui_callback_list::Services {
    fn invoke(
        &mut self,
        state: &mut State,
        function: u64,
        arguments: &[u64],
    ) -> Result<u64, &'static str>;
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
    fn byte(&self, p: u64) -> Result<u8, &'static str> {
        Ok(self.memory.read(p, 1)?[0])
    }
    fn set(&mut self, p: u64, v: u8) -> Result<(), &'static str> {
        self.memory.write(p, &[v])
    }
    fn copy(&mut self, d: u64, s: u64, n: usize) -> Result<(), &'static str> {
        let b = self.memory.read(s, n)?;
        self.memory.write(d, &b)
    }
}
fn invoke(s: &mut State, h: &mut impl Services, f: u64, a: &[u64]) -> Result<u64, &'static str> {
    h.invoke(s, f, a)
}
fn resource(s: &mut State, h: &mut impl Services) -> Result<u64, &'static str> {
    let index = s.resource_index;
    invoke(s, h, 0x141a64260, &[0x146124398, index as u64])
}
fn manager(s: &mut State, h: &mut impl Services) -> Result<u64, &'static str> {
    invoke(s, h, 0x1414cc7f0, &[])
}
fn registry(s: &mut State, h: &mut impl Services) -> Result<u64, &'static str> {
    invoke(s, h, 0x1417e9f50, &[])
}
fn has(s: &mut State, h: &mut impl Services, key: u64) -> Result<bool, &'static str> {
    Ok(invoke(s, h, 0x140d776a0, &[0x142f02ba8, key])? as u8 != 0)
}
fn release_string(s: &mut State, h: &mut impl Services, p: u64) -> Result<(), &'static str> {
    let cap = s.q(p + 24)?;
    if cap > 15 {
        let mut n = cap.checked_add(1).ok_or("string capacity overflow")?;
        let mut base = s.q(p)?;
        if n >= 4096 {
            let original = s.q(base.checked_sub(8).ok_or("bad string pointer")?)?;
            let delta = base.checked_sub(original).ok_or("bad string alignment")?;
            if !(8..=39).contains(&delta) {
                return Err("invalid aligned string header");
            }
            base = original;
            n = n.checked_add(39).ok_or("string capacity overflow")?;
        }
        invoke(s, h, 0x1422e7c2c, &[base, n])?;
    }
    Ok(())
}
fn empty_string(s: &mut State, p: u64) -> Result<(), &'static str> {
    s.memory.write(p, &[0; 16])?;
    s.putq(p + 16, 0)?;
    s.putq(p + 24, 15)
}
fn move_local(s: &mut State, h: &mut impl Services, d: u64, src: u64) -> Result<(), &'static str> {
    if d != src {
        release_string(s, h, d)?;
        s.putq(d + 16, 0)?;
        s.putq(d + 24, 15)?;
        s.set(d, 0)?;
        s.copy(d, src, 32)?;
        s.putq(src + 16, 0)?;
        s.putq(src + 24, 15)?;
        s.set(src, 0)?;
    }
    Ok(())
}
fn copy_text(s: &mut State, h: &mut impl Services, d: u64, src: u64) -> Result<(), &'static str> {
    if d != src {
        let n = s.q(src + 16)?;
        let data = if s.q(src + 24)? > 15 { s.q(src)? } else { src };
        invoke(s, h, 0x1405df240, &[d, data, n])?;
    }
    Ok(())
}
fn range(begin: u64, end: u64, stride: u64) -> Result<usize, &'static str> {
    let bytes = end.checked_sub(begin).ok_or("invalid live range")?;
    if bytes % stride != 0 || bytes / stride > 100000 {
        return Err("bounded aligned live range required");
    }
    Ok((bytes / stride) as usize)
}
pub fn refresh(
    s: &mut State,
    context: Context<'_>,
    h: &mut impl Services,
) -> Result<(), &'static str> {
    let o = s.identity;
    let a = s.stack + 0x20;
    let b = s.stack + 0x40;
    let record = s.stack + 0x110;
    s.memory.read(o, 0x3e0)?;
    s.memory.write(s.stack + 0x2e8, &0u32.to_le_bytes())?;
    let engine = invoke(s, h, 0x1417fb320, &[0x142f2d770])? as u8;
    let alternate = if engine != 0 && s.active_global == 0 {
        let r = resource(s, h)?;
        s.q(r + 0x6188)? != 0
    } else {
        false
    };
    if alternate {
        s.set(o + 0x3c0, 0)?;
        let r = resource(s, h)?;
        let text = invoke(s, h, 0x1405dd5c0, &[b, r + 0x6178])?;
        let result = invoke(s, h, 0x141a3b600, &[record, text])?;
        invoke(s, h, 0x141141de0, &[o, result, 0])?;
        invoke(s, h, 0x1405ddde0, &[record])?;
        if s.byte(o + 0x1d8)? != 0 {
            if invoke(s, h, 0x141100c40, &[o])? as u8 != 0 {
                let value = invoke(s, h, 0x1417f6dc0, &[b])?;
                invoke(s, h, 0x1410fa920, &[o + 0x1c0, value])?;
                invoke(s, h, 0x1408acd90, &[b])?;
                let m = manager(s, h)?;
                invoke(s, h, 0x1414d0900, &[m, o + 0x1c0])?;
            } else {
                let m = manager(s, h)?;
                let value = invoke(s, h, 0x1414ce750, &[m, b])?;
                invoke(s, h, 0x1410fa920, &[o + 0x1c0, value])?;
                invoke(s, h, 0x1408acd90, &[b])?;
            }
        } else {
            invoke(s, h, 0x141110050, &[o + 0x1c0])?;
        }
    } else {
        empty_string(s, a)?;
        let mut found = 0;
        if has(s, h, 0x142642dd0)? {
            let r = registry(s, h)?;
            let option = invoke(s, h, 0x140d77340, &[0x142f02ba8, b, 0x142642dd0])?;
            found = invoke(s, h, 0x1417e8460, &[r, option])?;
            release_string(s, h, b)?;
        }
        if found != 0 {
            copy_text(s, h, a, found)?;
        } else {
            let m = manager(s, h)?;
            let value = invoke(s, h, 0x1414ccf50, &[m, b])?;
            move_local(s, h, a, value)?;
            release_string(s, h, b)?;
        }
        if invoke(s, h, 0x1415404e0, &[a])? as u32 == 0 {
            let value = invoke(s, h, 0x1411490b0, &[0x145a120c0, b])?;
            move_local(s, h, a, value)?;
            release_string(s, h, b)?;
            if invoke(s, h, 0x1415404e0, &[a])? as u32 == 0 {
                let r = registry(s, h)?;
                let value = invoke(s, h, 0x1417e9a40, &[r, 0])?;
                copy_text(s, h, a, value)?;
            }
        }
        let text = invoke(s, h, 0x1405dd5c0, &[s.stack + 0x80, a])?;
        invoke(s, h, 0x141a3b600, &[record, text])?;
        invoke(s, h, 0x141141de0, &[o, record, 1])?;
        let mut cursor = s.q(o + 0x1c0)?;
        let end = s.q(o + 0x1c8)?;
        let n = range(cursor, end, 0x50)?;
        for _ in 0..n {
            if s.byte(cursor + 0x48)? != 0 {
                invoke(s, h, 0x1405de1d0, &[cursor + 8])?;
            }
            cursor += 0x50;
        }
        if n != 0 {
            let begin = s.q(o + 0x1c0)?;
            s.putq(o + 0x1c8, begin)?;
        }
        if !has(s, h, 0x1425de020)? && !has(s, h, 0x1425de030)? {
            let m = manager(s, h)?;
            let src = invoke(s, h, 0x1414ce750, &[m, b])?;
            if src != o + 0x1c0 {
                invoke(s, h, 0x1408acd90, &[o + 0x1c0])?;
                for off in [0, 8, 16] {
                    let v = s.q(src + off)?;
                    s.putq(o + 0x1c0 + off, v)?;
                }
                for off in [0, 8, 16] {
                    s.putq(src + off, 0)?;
                }
            }
            invoke(s, h, 0x1408acd90, &[b])?;
        } else {
            let m = manager(s, h)?;
            invoke(s, h, 0x1414d0900, &[m, o + 0x1c0])?;
        }
        s.memory.write(s.stack + 0x2e8, &0u32.to_le_bytes())?;
        s.putq(b, 0x14264fab0)?;
        s.putq(b + 8, s.stack + 0x2e8)?;
        s.putq(b + 0x38, b)?;
        invoke(s, h, 0x141138650, &[o + 0x268, b])?;
        let begin = s.q(o + 0x350)?;
        let count = (s.q(o + 0x358)?.wrapping_sub(begin) as i64) >> 2;
        let expected =
            i32::from_le_bytes(s.memory.read(s.stack + 0x2e8, 4)?.try_into().unwrap()) as i64;
        if count != expected {
            s.putq(o + 0x358, begin)?;
            s.putq(b, 0x14264fa78)?;
            s.putq(b + 8, o + 0x268)?;
            s.putq(b + 0x38, b)?;
            invoke(s, h, 0x141138650, &[o + 0x268, b])?;
        }
        invoke(s, h, 0x1405ddde0, &[record])?;
        release_string(s, h, a)?;
    }
    if s.byte(o + 0x1d9)? == 0 {
        let temp = s.stack + 0x60;
        context
            .callbacks
            .construct(temp, Some(o + 0x288), 0, 0, h)?;
        context
            .parents
            .get_mut(&(o + 0x1e0))
            .ok_or("missing nested receiver")?
            .append(temp, context.callbacks, h)?;
        context.callbacks.destroy(temp, h)?;
        // Publish only receiver links when a mapped list header is supplied.
        // Internal callback RB storage remains owned by the logical graph.
        let list = &context.parents[&(o + 0x1e0)];
        if s.memory.read(list.sentinel, 16).is_ok() {
            for (p, next, previous) in list.links() {
                if s.memory.read(p, 16).is_err() {
                    s.memory.blocks.insert(p, vec![0; 16]);
                }
                s.putq(p, next)?;
                s.putq(p + 8, previous)?;
            }
        }

        s.putq(
            o + 0x1f0,
            context.parents[&(o + 0x1e0)].entries.len() as u64,
        )?;
        s.putq(
            o + 0x298,
            context.callbacks.nodes[&(o + 0x288)].peers.len() as u64,
        )?;
        s.set(o + 0x1d9, 1)?;
    }
    if s.q(o + 0x210)? != 0 {
        invoke(s, h, 0x1411399c0, &[o + 0x1e0])?;
    }
    let mut cursor = s.q(o + 0x3c8)?;
    let end = s.q(o + 0x3d0)?;
    let n = range(cursor, end, 8)?;
    for _ in 0..n {
        let p = s.q(cursor)?;
        if p != 0 {
            invoke(s, h, 0x140f59550, &[p])?;
        }
        cursor += 8;
    }
    if n != 0 {
        let begin = s.q(o + 0x3c8)?;
        s.putq(o + 0x3d0, begin)?;
    }
    invoke(s, h, 0x14113a300, &[o + 0x268])?;
    s.set(o + 0x18, 1)?;
    let mut cursor = s.q(s.q(o + 8)?)?;
    let mut visits = 0;
    while cursor != s.q(o + 8)? {
        visits += 1;
        if visits > 100000 {
            return Err("bounded listener traversal required");
        }
        let old = cursor;
        invoke(s, h, 0x1410c9d10, &[cursor + 16])?;
        cursor = s.q(old)?;
    }
    s.set(o + 0x18, 0)
}

/// Bridge for the first child's final refresh service. Runtime memory supplies
/// live vector/list backing and scratch storage; reference identities are data.
/// Raw callback RB nodes remain represented by the logical callback/list states.
pub fn refresh_image(
    image: &mut [u8],
    runtime: &mut State,
    context: Context<'_>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0x3e0 {
        return Err("root image size mismatch");
    }
    runtime
        .memory
        .blocks
        .insert(runtime.identity, image.to_vec());
    let result = refresh(runtime, context, services);
    let current = runtime.memory.read(runtime.identity, 0x3e0)?;
    image.copy_from_slice(&current);
    result
}
