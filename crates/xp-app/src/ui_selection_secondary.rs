//! Whole secondary predicate14113ff10, including recovered primary/membership/
//! resource/equality bodies. Singleton, mode, header and CRT leaves remain services.
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
fn query<H: Services + ?Sized>(
    m: &mut ui_payload_move::State,
    h: &mut H,
) -> Result<u64, &'static str> {
    let singleton = h.invoke(m, 0x1414cc7f0, &[])?;
    h.invoke(m, 0x1414cedd0, &[singleton])
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
    let result = query(m, h)?;
    if result as u8 != 0 && (d(m, 0x145899fd8)? != 0 || d(m, 0x145899fdc)? != 0) {
        return Ok(al(result, 1));
    }
    let result = query(m, h)?;
    if result as u8 == 0 && (d(m, 0x145899fd0)? != 0 || d(m, 0x145899fd4)? != 0) {
        return Ok(al(result, 1));
    }
    let result = query(m, h)?;
    if result as u8 == 0 {
        return Ok(al(result, 0));
    }
    let record = ui_indexed_records::lookup(m, Kind::Resource, d(m, INDEX)? as u64)?;
    let header = h.invoke(m, 0x1411222e0, &[receiver])?;
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
