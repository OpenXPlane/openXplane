//! Switch state behind the electrical datarefs of the stock Cessna 172 SP. The start values and the
//! master-switch interlock follow its xlua scripts (see `research/C172_XLUA_SYSTEMS.md`). Bus volts and
//! amps are not modelled: they come from the native electrical model, which has no reference yet.
//! `battery_on[0]` and `generator_on[0]` are assumed off at a cold start; the scripts do not set them.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Start {
    ColdAndDark,
    Running,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Electrical {
    /// `battery_on[0..2]`.
    pub battery_on: [bool; 2],
    pub generator_on: bool,
    pub cross_tie: bool,
    pub fuel_pump_on: bool,
    /// `instrument_brightness_ratio[0..4]`.
    pub instrument_brightness: [f32; 4],
    /// `panel_brightness_ratio[3]`.
    pub glareshield_brightness: f32,
}

impl Electrical {
    /// State after `flight_start`; `batteries` is `sim/aircraft/electrical/num_batteries` (1 analog,
    /// 2 G1000).
    pub fn at_start(start: Start, batteries: u32) -> Self {
        let mut e = Self {
            battery_on: [false; 2],
            generator_on: false,
            cross_tie: false,
            fuel_pump_on: false,
            instrument_brightness: [0.4, 0.75, 0.75, 0.2],
            glareshield_brightness: 0.3,
        };
        if start == Start::Running {
            if batteries == 1 {
                e.cross_tie = true;
            } else {
                e.battery_on[1] = true;
            }
        }
        e
    }

    /// Flips the battery master and applies the interlock: switching it off drops the alternator.
    pub fn set_battery(&mut self, on: bool) {
        self.battery_on[0] = on;
        if !on {
            self.generator_on = false;
        }
    }

    /// Flips the alternator switch; switching it on closes the battery master.
    pub fn set_generator(&mut self, on: bool) {
        self.generator_on = on;
        if on {
            self.battery_on[0] = true;
        }
    }

    pub fn dataref(&self, name: &str) -> Option<f32> {
        let flag = |b: bool| Some(if b { 1.0 } else { 0.0 });
        if name == "sim/cockpit2/electrical/cross_tie" {
            return flag(self.cross_tie);
        }
        let (array, index) = name.strip_suffix(']')?.rsplit_once('[')?;
        let index: usize = index.parse().ok()?;
        match array {
            "sim/cockpit2/electrical/battery_on" => flag(*self.battery_on.get(index)?),
            "sim/cockpit2/electrical/generator_on" if index == 0 => flag(self.generator_on),
            "sim/cockpit2/engine/actuators/fuel_pump_on" if index == 0 => flag(self.fuel_pump_on),
            "sim/cockpit2/switches/instrument_brightness_ratio" => {
                self.instrument_brightness.get(index).copied()
            }
            "sim/cockpit2/switches/panel_brightness_ratio" if index == 3 => {
                Some(self.glareshield_brightness)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_state_follows_the_stock_init_script() {
        let cold = Electrical::at_start(Start::ColdAndDark, 1);
        assert_eq!(cold.dataref("sim/cockpit2/electrical/cross_tie"), Some(0.0));
        assert_eq!(
            cold.dataref("sim/cockpit2/electrical/battery_on[1]"),
            Some(0.0)
        );
        let analog = Electrical::at_start(Start::Running, 1);
        assert_eq!(
            analog.dataref("sim/cockpit2/electrical/cross_tie"),
            Some(1.0)
        );
        let g1000 = Electrical::at_start(Start::Running, 2);
        assert_eq!(
            g1000.dataref("sim/cockpit2/electrical/cross_tie"),
            Some(0.0)
        );
        assert_eq!(
            g1000.dataref("sim/cockpit2/electrical/battery_on[1]"),
            Some(1.0)
        );
        assert_eq!(
            cold.dataref("sim/cockpit2/switches/instrument_brightness_ratio[2]"),
            Some(0.75)
        );
        assert_eq!(
            cold.dataref("sim/cockpit2/switches/panel_brightness_ratio[3]"),
            Some(0.3)
        );
        assert!(
            cold.dataref("sim/cockpit2/electrical/battery_on[2]")
                .is_none()
        );
    }

    #[test]
    fn master_switches_are_interlocked() {
        let mut e = Electrical::at_start(Start::ColdAndDark, 1);
        e.set_generator(true);
        assert!(e.battery_on[0] && e.generator_on);
        e.set_battery(false);
        assert!(!e.generator_on);
    }
}
