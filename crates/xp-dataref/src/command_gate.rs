//! Native command blocking mask query, 0x140c63f00.
//! Mask assignment/lifecycle and callback effects remain outside this query.
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct CommandGate {
    pub default_mask: u32,
    pub by_command: BTreeMap<i32, u32>,
}

/// Exact native exemptions. Range comparisons use wrapping unsigned subtraction.
pub fn exempt(command: i32) -> bool {
    let id = command as u32;
    (id <= 36 && (0x11f08a000e_u64 & (1_u64 << id)) != 0)
        || (id.wrapping_sub(0xb44) <= 8 && (0x141_u32 & (1_u32 << id.wrapping_sub(0xb44))) != 0)
        || id.wrapping_sub(0xb8a) <= 0x86
}

impl CommandGate {
    /// Source mask 0 tests any block bit. A nonzero source mask removes only
    /// its own bits from the effective block mask, not all blocking.
    pub fn is_blocked(&self, command: i32, source_mask: u32) -> bool {
        if exempt(command) {
            return false;
        }
        let mask = self
            .by_command
            .get(&command)
            .copied()
            .unwrap_or(self.default_mask);
        if source_mask == 0 {
            mask != 0
        } else {
            mask & !source_mask != 0
        }
    }
}
