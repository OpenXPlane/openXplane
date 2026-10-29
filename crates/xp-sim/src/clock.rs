//! Scenario clock behind the time datarefs (`sim/time/zulu_time_sec`, `sim/cockpit2/clock_timer/*`).
//! The semantics follow the public dataref documentation; none of them has been compared with the
//! original game. In particular the local time here is the mean solar offset (longitude / 15 hours),
//! not a timezone lookup.

const DAY_SECONDS: f64 = 86_400.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimClock {
    /// Calendar month (1..=12) and day (1..=31) of the scenario start.
    pub month: u32,
    pub day: u32,
    /// Seconds after zulu midnight at simulation time zero.
    pub zulu_start_sec: f64,
    /// Longitude used for the local-time offset, degrees east.
    pub longitude_deg: f64,
}

impl Default for SimClock {
    fn default() -> Self {
        // No scenario time source exists yet; noon zulu on 1 January at the prime meridian.
        Self {
            month: 1,
            day: 1,
            zulu_start_sec: 12.0 * 3600.0,
            longitude_deg: 0.0,
        }
    }
}

impl SimClock {
    /// Seconds after zulu midnight at simulation time `t` (wraps at 24 h).
    pub fn zulu_sec(&self, t: f64) -> f64 {
        (self.zulu_start_sec + t).rem_euclid(DAY_SECONDS)
    }

    /// Seconds after local midnight at simulation time `t`.
    pub fn local_sec(&self, t: f64) -> f64 {
        (self.zulu_start_sec + t + self.longitude_deg / 15.0 * 3600.0).rem_euclid(DAY_SECONDS)
    }

    /// The datarefs this clock answers, or `None` for other names. Integer datarefs are returned as
    /// exact integers in the float.
    pub fn dataref(&self, name: &str, t: f64) -> Option<f32> {
        let (z, l) = (self.zulu_sec(t), self.local_sec(t));
        let hms = |s: f64| {
            (
                (s / 3600.0).floor(),
                (s / 60.0).floor() % 60.0,
                s.floor() % 60.0,
            )
        };
        let (zh, zm, zs) = hms(z);
        let (lh, lm, ls) = hms(l);
        Some(match name {
            "sim/time/zulu_time_sec" => z as f32,
            "sim/time/local_time_sec" => l as f32,
            "sim/cockpit2/clock_timer/zulu_time_hours" => zh as f32,
            "sim/cockpit2/clock_timer/zulu_time_minutes" => zm as f32,
            "sim/cockpit2/clock_timer/zulu_time_seconds" => zs as f32,
            "sim/cockpit2/clock_timer/local_time_hours" => lh as f32,
            "sim/cockpit2/clock_timer/local_time_minutes" => lm as f32,
            "sim/cockpit2/clock_timer/local_time_seconds" => ls as f32,
            "sim/cockpit2/clock_timer/current_month" => self.month as f32,
            "sim/cockpit2/clock_timer/current_day" => self.day as f32,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zulu_and_local_wrap_and_split_into_fields() {
        let c = SimClock {
            month: 10,
            day: 29,
            zulu_start_sec: 23.0 * 3600.0 + 59.0 * 60.0 + 30.0,
            longitude_deg: 132.0,
        };
        // Vladivostok-like longitude: +8.8 h from zulu.
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_hours", 0.0),
            Some(23.0)
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_minutes", 0.0),
            Some(59.0)
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_seconds", 0.0),
            Some(30.0)
        );
        // 45 s later the zulu day has rolled over.
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_hours", 45.0),
            Some(0.0)
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_minutes", 45.0),
            Some(0.0)
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/zulu_time_seconds", 45.0),
            Some(15.0)
        );
        let local = c.local_sec(0.0);
        assert!(
            (local - ((23.0 * 3600.0 + 59.0 * 60.0 + 30.0 + 8.8 * 3600.0) % 86_400.0)).abs() < 1e-6
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/current_month", 0.0),
            Some(10.0)
        );
        assert_eq!(
            c.dataref("sim/cockpit2/clock_timer/current_day", 0.0),
            Some(29.0)
        );
        assert!(c.dataref("sim/time/not_a_clock", 0.0).is_none());
    }

    #[test]
    fn negative_longitude_offsets_local_time_backwards() {
        let c = SimClock {
            zulu_start_sec: 3600.0,
            longitude_deg: -75.0, // -5 h
            ..SimClock::default()
        };
        assert!((c.local_sec(0.0) - 20.0 * 3600.0).abs() < 1e-6);
    }
}
