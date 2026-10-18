//! Version-bound query1417fb320: zero-extend byte at receiver+0x525.
//! The producer/lifetime and meaning of this byte are not established here.
use crate::ui_payload_move;
pub const RECEIVER: u64 = 0x142f2d770;
pub const OFFSET: u64 = 0x525;
pub fn query_at(memory: &ui_payload_move::State, receiver: u64) -> Result<u64, &'static str> {
    Ok(memory.read(receiver.wrapping_add(OFFSET), 1)?[0] as u64)
}
pub fn query(memory: &ui_payload_move::State) -> Result<u64, &'static str> {
    query_at(memory, RECEIVER)
}
