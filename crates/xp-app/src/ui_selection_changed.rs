//! Original selection change14113ba10. Set dirty and notify live listeners;
//! deliberately no final dirty store. Callback effects remain a service.
use crate::ui_payload_move;
pub struct State {
    pub memory: ui_payload_move::State,
    pub identity: u64,
}
pub trait Services {
    fn notify(&mut self, state: &mut State, wrapper: u64) -> Result<(), &'static str>;
}
impl State {
    fn q(&self, p: u64) -> Result<u64, &'static str> {
        Ok(u64::from_le_bytes(
            self.memory.read(p, 8)?.try_into().unwrap(),
        ))
    }
}
pub fn changed(s: &mut State, h: &mut impl Services) -> Result<(), &'static str> {
    s.memory.write(s.identity + 0x18, &[1])?;
    let head = s.q(s.identity + 8)?;
    let mut current = s.q(head)?;
    let mut visits = 0;
    while current != s.q(s.identity + 8)? {
        visits += 1;
        if visits > 100000 {
            return Err("unverified nonterminating selection listeners");
        }
        h.notify(s, current + 16)?;
        current = s.q(current)?;
    }
    Ok(())
}
pub fn changed_image(
    image: &mut [u8],
    s: &mut State,
    h: &mut impl Services,
) -> Result<(), &'static str> {
    if image.len() != 0x2d0 {
        return Err("selection image must be0x2d0 bytes");
    }
    s.memory.blocks.insert(s.identity, image.to_vec());
    let result = changed(s, h);
    image.copy_from_slice(&s.memory.read(s.identity, 0x2d0)?);
    result
}
