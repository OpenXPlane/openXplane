//! Fuel tank selector and cutoff of the stock Cessna 172 SP, following its `c172_fuel_selector` xlua
//! script (see `research/C172_XLUA_SYSTEMS.md`). The start position of `fuel_tank_selector` comes from
//! the native model, which has no reference yet; "all" is assumed. Fuel flow and quantities are not
//! modelled, and the commands are not bound to keys.

/// `sim/cockpit2/fuel/fuel_tank_selector` values.
pub const NONE: u8 = 0;
pub const LEFT: u8 = 1;
pub const RIGHT: u8 = 3;
pub const ALL: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuelSelector {
    /// Where the handle is set, ignoring the cutoff (1 left, 3 right, 4 all).
    pub handle: u8,
    pub cutoff: bool,
}

impl Default for FuelSelector {
    fn default() -> Self {
        Self {
            handle: ALL,
            cutoff: false,
        }
    }
}

impl FuelSelector {
    /// `sim/cockpit2/fuel/fuel_tank_selector`: none while the cutoff is pulled.
    pub fn tank_selector(&self) -> u8 {
        if self.cutoff { NONE } else { self.handle }
    }

    /// `laminar/c172/fuel/fuel_tank_selector` at rest: 1 left, 2 all, 3 right.
    pub fn handle_position(&self) -> f32 {
        if self.handle == ALL {
            2.0
        } else {
            f32::from(self.handle)
        }
    }

    /// `laminar/c172/fuel_selector_up`: left -> all -> right.
    pub fn up(&mut self) {
        self.handle = if self.handle == RIGHT {
            RIGHT
        } else if self.handle == LEFT {
            ALL
        } else if self.handle == ALL {
            RIGHT
        } else {
            ALL
        };
    }

    /// `laminar/c172/fuel_selector_dwn`: right -> all -> left.
    pub fn down(&mut self) {
        self.handle = if self.handle == RIGHT { ALL } else { LEFT };
    }

    /// `sim/starters/shut_down` (replaced by the script): toggles the fuel cutoff.
    pub fn toggle_cutoff(&mut self) {
        self.cutoff = !self.cutoff;
    }

    pub fn dataref(&self, name: &str) -> Option<f32> {
        Some(match name {
            "sim/cockpit2/fuel/fuel_tank_selector" => f32::from(self.tank_selector()),
            "laminar/c172/fuel/fuel_tank_selector" => self.handle_position(),
            "laminar/c172/fuel/fuel_cutoff_selector" => f32::from(u8::from(self.cutoff)),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_steps_through_left_all_right_and_clamps() {
        let mut f = FuelSelector::default();
        assert_eq!(f.tank_selector(), ALL);
        f.up();
        assert_eq!(f.handle, RIGHT);
        f.up();
        assert_eq!(f.handle, RIGHT);
        f.down();
        assert_eq!(f.handle, ALL);
        f.down();
        assert_eq!(f.handle, LEFT);
        f.down();
        assert_eq!(f.handle, LEFT);
        f.up();
        assert_eq!(f.handle, ALL);
    }

    #[test]
    fn cutoff_closes_the_selector_and_restores_the_handle() {
        let mut f = FuelSelector::default();
        f.down();
        f.toggle_cutoff();
        assert_eq!(f.dataref("sim/cockpit2/fuel/fuel_tank_selector"), Some(0.0));
        assert_eq!(
            f.dataref("laminar/c172/fuel/fuel_cutoff_selector"),
            Some(1.0)
        );
        assert_eq!(f.dataref("laminar/c172/fuel/fuel_tank_selector"), Some(1.0));
        f.toggle_cutoff();
        assert_eq!(f.dataref("sim/cockpit2/fuel/fuel_tank_selector"), Some(1.0));
    }
}
