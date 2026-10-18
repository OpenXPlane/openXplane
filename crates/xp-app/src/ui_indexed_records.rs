//! Original record accessors141a64250/settings and141a64260/resources.
//! These compute logical addresses without bounds checks or host dereferences.
use crate::ui_payload_move;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Resource,
    Settings,
}
impl Kind {
    pub const fn stride(self) -> u64 {
        match self {
            Self::Resource => 0x6668,
            Self::Settings => 0x433c0,
        }
    }
    pub const fn table(self) -> u64 {
        match self {
            Self::Resource => 0x146124398,
            Self::Settings => 0x1461243b0,
        }
    }
    /// Original MOVSLQ EDX ignores high argument bits, then IMUL/ADD wrap64 bits.
    pub fn address(self, base: u64, index: u64) -> u64 {
        let signed = index as u32 as i32 as i64 as u64;
        base.wrapping_add(signed.wrapping_mul(self.stride()))
    }
}
pub fn lookup_at(
    memory: &ui_payload_move::State,
    kind: Kind,
    table: u64,
    index: u64,
) -> Result<u64, &'static str> {
    let base = u64::from_le_bytes(memory.read(table, 8)?.try_into().unwrap());
    Ok(kind.address(base, index))
}
pub fn lookup(
    memory: &ui_payload_move::State,
    kind: Kind,
    index: u64,
) -> Result<u64, &'static str> {
    lookup_at(memory, kind, kind.table(), index)
}
