//! Whole secondary predicate14113ff10, including recovered primary/membership/
//! resource/equality bodies. `with_dependencies` also composes singleton/mode/header;
//! their settings/catalog producers, allocation/free and CRT remain services.
use crate::{
    ui_indexed_records::{self, Kind},
    ui_payload_move, ui_record_class_membership,
};
pub const INDEX: u64 = 0x142640508;
fn q(m: &ui_payload_move::State, p: u64) -> Result<u64, &'static str> {
    Ok(u64::from_le_bytes(m.read(p, 8)?.try_into().unwrap()))
}
fn d(m: &ui_payload_move::State, p: u64) -> Result<u32, &'static str> {
    Ok(u32::from_le_bytes(m.read(p, 4)?.try_into().unwrap()))
}
fn al(value: u64, byte: u8) -> u64 {
    (value & !255) | byte as u64
}
pub trait Services {
    fn invoke(
        &mut self,
        m: &mut ui_payload_move::State,
        function: u64,
        args: &[u64],
    ) -> Result<u64, &'static str>;
    fn query(&mut self, m: &mut ui_payload_move::State) -> Result<u64, &'static str> {
        let singleton = self.invoke(m, 0x1414cc7f0, &[])?;
        self.invoke(m, 0x1414cedd0, &[singleton])
    }
    fn header(
        &mut self,
        m: &mut ui_payload_move::State,
        receiver: u64,
    ) -> Result<u64, &'static str> {
        self.invoke(m, 0x1411222e0, &[receiver])
    }
    fn primary(
        &mut self,
        m: &mut ui_payload_move::State,
        receiver: u64,
    ) -> Result<u64, &'static str> {
        primary(m, self, receiver)
    }
    fn member(
        &mut self,
        m: &mut ui_payload_move::State,
        record: u64,
        index: u64,
    ) -> Result<u64, &'static str> {
        struct Bridge<'a, H: ?Sized>(&'a mut H);
        impl<H: Services + ?Sized> ui_record_class_membership::Services for Bridge<'_, H> {
            fn invoke(
                &mut self,
                m: &mut ui_payload_move::State,
                f: u64,
                a: &[u64],
            ) -> Result<u64, &'static str> {
                self.0.invoke(m, f, a)
            }
        }
        ui_record_class_membership::contains(m, &mut Bridge(self), record, index)
    }
    fn equal(
        &mut self,
        m: &mut ui_payload_move::State,
        left: u64,
        right: u64,
    ) -> Result<u64, &'static str> {
        equal(m, self, left, right)
    }
}
/// Full RAX of14113fff0, unlike the older canonical-AL gate interface.
pub fn primary<H: Services + ?Sized>(
    m: &mut ui_payload_move::State,
    h: &mut H,
    receiver: u64,
) -> Result<u64, &'static str> {
    let record = ui_indexed_records::lookup(m, Kind::Resource, d(m, 0x142640504)? as u64)?;
    let mut right = record.wrapping_add(0x6178);
    let left = receiver.wrapping_add(0x20);
    let right_capacity = q(m, right.wrapping_add(24))?;
    let right_length = q(m, right.wrapping_add(16))?;
    if right_capacity > 15 {
        right = q(m, right)?;
    }
    let left_capacity = q(m, left.wrapping_add(24))?;
    let left_length = q(m, left.wrapping_add(16))?;
    let left_data = if left_capacity > 15 {
        q(m, left)?
    } else {
        left
    };
    if left_length != right_length {
        return Ok(al(right, 1));
    }
    let result = h.invoke(m, 0x1423c0180, &[left_data, right, left_length])?;
    Ok(if result as u32 != 0 {
        al(result, 1)
    } else {
        result
    })
}
/// Full RAX of string-equality14073ba40; only AL is canonical.
pub fn equal<H: Services + ?Sized>(
    m: &mut ui_payload_move::State,
    h: &mut H,
    left: u64,
    right: u64,
) -> Result<u64, &'static str> {
    let right_capacity = q(m, right.wrapping_add(24))?;
    let length = q(m, right.wrapping_add(16))?;
    let right_data = if right_capacity > 15 {
        q(m, right)?
    } else {
        right
    };
    let left_capacity = q(m, left.wrapping_add(24))?;
    let left_length = q(m, left.wrapping_add(16))?;
    let left_data = if left_capacity > 15 {
        q(m, left)?
    } else {
        left
    };
    if left_length != length {
        return Ok(al(length, 0));
    }
    let result = h.invoke(m, 0x1423c0180, &[left_data, right_data, left_length])?;
    Ok(al(result, u8::from(result as u32 == 0)))
}
pub fn secondary(
    m: &mut ui_payload_move::State,
    h: &mut impl Services,
    receiver: u64,
) -> Result<u64, &'static str> {
    let result = h.primary(m, receiver)?;
    if result as u8 != 0 {
        return Ok(al(result, 0));
    }
    let result = h.member(m, receiver.wrapping_add(0x20), 2)?;
    if result as u8 == 0 {
        return Ok(al(result, 0));
    }
    let result = h.query(m)?;
    if result as u8 != 0 && (d(m, 0x145899fd8)? != 0 || d(m, 0x145899fdc)? != 0) {
        return Ok(al(result, 1));
    }
    let result = h.query(m)?;
    if result as u8 == 0 && (d(m, 0x145899fd0)? != 0 || d(m, 0x145899fd4)? != 0) {
        return Ok(al(result, 1));
    }
    let result = h.query(m)?;
    if result as u8 == 0 {
        return Ok(al(result, 0));
    }
    let record = ui_indexed_records::lookup(m, Kind::Resource, d(m, INDEX)? as u64)?;
    let header = h.header(m, receiver)?;
    let result = h.equal(m, header, record.wrapping_add(0x6178))?;
    Ok(al(result, u8::from(result as u8 == 0)))
}
pub fn with_memory<F>(
    m: &mut ui_payload_move::State,
    receiver: u64,
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
        fn invoke(
            &mut self,
            m: &mut ui_payload_move::State,
            f: u64,
            a: &[u64],
        ) -> Result<u64, &'static str> {
            (self.0)(m, f, a)
        }
    }
    secondary(m, &mut Bridge(invoke), receiver)
}

/// Execute actual singleton/mode/header bodies too. `stack` is the secondary
/// caller's RSP after its prologue; inherited local maps remain observable.
/// Settings and catalog providers, allocation/free and CRT remain explicit.
pub fn with_dependencies<F>(
    m: &mut ui_payload_move::State,
    stack: u64,
    receiver: u64,
    invoke: F,
) -> Result<u64, &'static str>
where
    F: FnMut(&mut ui_payload_move::State, u64, &[u64]) -> Result<u64, &'static str>,
{
    use crate::ui_selection_dependencies as deps;
    struct Bridge<F> {
        invoke: F,
        stack: u64,
    }
    impl<F> deps::Services for Bridge<F>
    where
        F: FnMut(&mut ui_payload_move::State, u64, &[u64]) -> Result<u64, &'static str>,
    {
        fn invoke(&mut self, s: &mut deps::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
            (self.invoke)(&mut s.memory, f, a)
        }
    }
    impl<F> Services for Bridge<F>
    where
        F: FnMut(&mut ui_payload_move::State, u64, &[u64]) -> Result<u64, &'static str>,
    {
        fn invoke(
            &mut self,
            m: &mut ui_payload_move::State,
            f: u64,
            a: &[u64],
        ) -> Result<u64, &'static str> {
            (self.invoke)(m, f, a)
        }
        fn query(&mut self, m: &mut ui_payload_move::State) -> Result<u64, &'static str> {
            let mut s = deps::State {
                memory: std::mem::take(m),
                stack: self.stack - 0x90,
            };
            let result = deps::singleton(&mut s, self).and_then(|_| deps::mode(&mut s, self));
            *m = s.memory;
            result
        }
        fn header(
            &mut self,
            m: &mut ui_payload_move::State,
            receiver: u64,
        ) -> Result<u64, &'static str> {
            let mut s = deps::State {
                memory: std::mem::take(m),
                stack: self.stack - 0x60,
            };
            let result = deps::header(&mut s, self, receiver);
            *m = s.memory;
            result
        }
    }
    let mut created = Vec::new();
    for p in [stack - 0x70, stack - 0x50, stack - 0x30] {
        if m.read(p, 32).is_ok() {
            continue;
        }
        if m.blocks
            .iter()
            .any(|(&base, b)| base < p + 32 && base + b.len() as u64 > p)
        {
            for p in created {
                m.blocks.remove(&p);
            }
            return Err("partially mapped selection dependency scratch unverified");
        }
        m.blocks.insert(p, vec![0; 32]);
        created.push(p);
    }
    let result = secondary(m, &mut Bridge { invoke, stack }, receiver);
    for p in created {
        m.blocks.remove(&p);
    }
    result
}
