//! Ignition key of the stock Cessna 172 SP, following its `c172_starter` xlua script (see
//! `research/C172_XLUA_SYSTEMS.md`). The script turns one pair of commands into key steps: down steps the
//! key back (never from START), up steps it forward and, from BOTH or START, holds the starter. The native
//! behaviour that the key snaps from START back to BOTH on release is inferred from the script's
//! `STOPPING COMMAND` branch, not measured. The engine does not use the key yet.

/// `sim/cockpit2/engine/actuators/ignition_key[0]` values.
pub const OFF: u8 = 0;
pub const RIGHT: u8 = 1;
pub const LEFT: u8 = 2;
pub const BOTH: u8 = 3;
pub const START: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ignition {
    pub key: u8,
}

impl Default for Ignition {
    /// The documented cold-and-dark position; the original's value is not recorded.
    fn default() -> Self {
        Self { key: OFF }
    }
}

impl Ignition {
    /// `laminar/c172/ignition_down` on press.
    pub fn down(&mut self) {
        if self.key != START && self.key > OFF {
            self.key -= 1;
        }
    }

    /// `laminar/c172/ignition_up` on press: steps up below BOTH, otherwise engages the starter.
    pub fn up_press(&mut self) {
        if self.key < BOTH {
            self.key += 1;
        } else {
            self.key = START;
        }
    }

    /// `laminar/c172/ignition_up` on release: the key springs back from START.
    pub fn up_release(&mut self) {
        if self.key == START {
            self.key = BOTH;
        }
    }

    pub fn starter_engaged(&self) -> bool {
        self.key == START
    }

    pub fn dataref(&self, name: &str) -> Option<f32> {
        (name == "sim/cockpit2/engine/actuators/ignition_key[0]").then(|| f32::from(self.key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_steps_up_to_both_then_starts_and_springs_back() {
        let mut i = Ignition::default();
        for expected in [RIGHT, LEFT, BOTH, START] {
            i.up_press();
            i.up_release();
            if expected == START {
                assert_eq!(i.key, BOTH, "released after START");
            } else {
                assert_eq!(i.key, expected);
            }
        }
        i.up_press();
        assert!(i.starter_engaged());
        i.down();
        assert_eq!(i.key, START, "down does nothing from START");
        i.up_release();
        i.down();
        assert_eq!(i.key, LEFT);
    }
}
