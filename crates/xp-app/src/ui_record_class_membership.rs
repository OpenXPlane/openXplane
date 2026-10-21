//! Original class-membership141a400f0, including its Favorites tail delegation.
//! Ordered trees are mapped input; registry production and native faults are unverified.
use crate::ui_payload_move;
pub trait Services {
    fn invoke(
        &mut self,
        memory: &mut ui_payload_move::State,
        function: u64,
        args: &[u64],
    ) -> Result<u64, &'static str>;
}
fn q(memory: &ui_payload_move::State, p: u64) -> Result<u64, &'static str> {
    Ok(u64::from_le_bytes(memory.read(p, 8)?.try_into().unwrap()))
}
pub fn contains(
    memory: &mut ui_payload_move::State,
    h: &mut impl Services,
    record: u64,
    index: u64,
) -> Result<u64, &'static str> {
    let key = index as u32 as i32;
    if key == 11 {
        let singleton = h.invoke(memory, 0x1414cc7f0, &[])?;
        return h.invoke(memory, 0x1414cc820, &[singleton, record]);
    }
    let mut candidate = q(memory, record.wrapping_add(0x180))?;
    let mut node = q(memory, candidate.wrapping_add(8))?;
    let mut visits = 0;
    while memory.read(node.wrapping_add(0x19), 1)?[0] == 0 {
        visits += 1;
        if visits > 100000 {
            return Err("cyclic class-membership tree unverified");
        }
        let value =
            i32::from_le_bytes(memory.read(node.wrapping_add(0x1c), 4)?.try_into().unwrap());
        if value < key {
            node = q(memory, node.wrapping_add(0x10))?;
        } else {
            candidate = node;
            node = q(memory, node)?;
        }
    }
    let present = memory.read(candidate.wrapping_add(0x19), 1)?[0] == 0
        && key
            >= i32::from_le_bytes(
                memory
                    .read(candidate.wrapping_add(0x1c), 4)?
                    .try_into()
                    .unwrap(),
            );
    // These paths write AL only; the remaining RAX bits retain the terminal node.
    Ok((node & !255) | u64::from(present))
}
