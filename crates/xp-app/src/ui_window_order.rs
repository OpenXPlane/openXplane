//! Native registry extraction (0x1412eecc0) and classified insertion (0x1412e4220).
//! Stable entry indices model pointer identity. Allocation/lifetime and widget effects
//! are outside this module; malformed native lists are rejected rather than emulated.
use crate::ui_state::RegisteredWindow;

#[derive(Clone, Debug, Default)]
pub struct Registry {
    pub entries: Vec<RegisteredWindow>,
    pub order: Vec<usize>,
}
impl Registry {
    /// Removes the first token match, as the native helper does before reinsertion.
    /// The native missing-token path unlinks the sentinel and is unsupported here.
    pub fn extract(&mut self, token: u64) -> Result<usize, &'static str> {
        for position in 0..self.order.len() {
            let index = self.order[position];
            let entry = self.entries.get(index).ok_or("invalid registry entry")?;
            if entry.token == token {
                self.order.remove(position);
                return Ok(index);
            }
        }
        Err("native extraction requires a registered token")
    }

    /// Nonzero low byte inserts before the first class <= target; zero inserts
    /// before the first class < target. Signed comparisons and ties are significant.
    /// No sorting/deduplication is performed on existing list contents.
    pub fn insert(&mut self, index: usize, flag: u8) -> Result<(), &'static str> {
        let class = self
            .entries
            .get(index)
            .ok_or("invalid insertion entry")?
            .classification;
        let mut position = self.order.len();
        for (i, &other) in self.order.iter().enumerate() {
            let other_class = self
                .entries
                .get(other)
                .ok_or("invalid registry entry")?
                .classification;
            if if flag != 0 {
                other_class <= class
            } else {
                other_class < class
            } {
                position = i;
                break;
            }
        }
        self.order.insert(position, index);
        Ok(())
    }
}
