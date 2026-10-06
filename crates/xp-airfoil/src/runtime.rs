//! Recovered storage and float dataref access for total running time.
//! Clock advancement, pause and replay behavior have not been recovered.

pub const RUNNING_TIME_DATAREF: &str = "sim/time/total_running_time_sec";

#[derive(Clone, Copy, Debug)]
pub struct RunningTime {
    seconds: f64,
}

impl RunningTime {
    /// Snapshot of the internal double at 0x142f01918; no float32 round trip.
    pub fn from_snapshot(seconds: f64) -> Result<Self, String> {
        if !seconds.is_finite() {
            return Err("running time must be finite".into());
        }
        Ok(Self { seconds })
    }

    pub fn seconds(self) -> f64 {
        self.seconds
    }

    /// Getter 0x1418b6bb0 converts the stored double to float32.
    pub fn read_dataref(self) -> f32 {
        self.seconds as f32
    }

    /// Setter 0x1418b6d70 widens an incoming float32 to double.
    /// Non-finite inputs are rejected by our diagnostic API.
    pub fn write_dataref(&mut self, seconds: f32) -> Result<(), String> {
        if !seconds.is_finite() {
            return Err("running time dataref must be finite".into());
        }
        self.seconds = f64::from(seconds);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataref_round_trip_loses_precision_but_internal_snapshot_does_not() {
        let mut time = RunningTime::from_snapshot(16_777_217.0).unwrap();
        assert_eq!(time.seconds(), 16_777_217.0);
        let exposed = time.read_dataref();
        assert_eq!(exposed, 16_777_216.0);
        assert_eq!(time.seconds(), 16_777_217.0);
        time.write_dataref(exposed).unwrap();
        assert_eq!(time.seconds(), 16_777_216.0);
        assert!(time.write_dataref(f32::NAN).is_err());
        assert_eq!(time.seconds(), 16_777_216.0);
        assert!(RunningTime::from_snapshot(f64::INFINITY).is_err());
    }
}
