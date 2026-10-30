# Reference takeoff of the stock Cessna 172 SP (X-Plane 12.4.3, Data Output)

Recorded 2026-10-30: new flight, "Cessna Skyhawk" (`Cessna_172SP.acf` per `Log.txt`) at UHWW runway 07L, clear
weather, engines started from the Flight menu, parking brake released with `V`, full throttle with `F3`, the
mouse yoke pulled back later. `_Bdisk0,1,3,4,17,18,20` enabled for the run and restored afterwards. The excerpt
`research/reference/c172_takeoff_uhww_20261030.csv` keeps every tenth sample (about 0.2 s) from 2 s before the
roll to 45 s after it: indicated airspeed, VVI, alpha, pitch, roll, heading, height above ground, position.
Column mapping of the original table: 14 Vind kias, 22 VVI, 27 alpha, 32 pitch, 33 roll, 34 heading,
41 height above ground (46 columns per sample, the file wraps each sample over several lines).

## Roll comparison with the port (`fly-test`, full throttle from standstill)
| Speed | Reference, s after the roll starts | Port, s |
|---:|---:|---:|
| 10 kt | 1.7 | 2.0 |
| 20 kt | 4.2 | 4.2 |
| 30 kt | 7.5 | 6.4 |
| 40 kt | 10.9 | 8.7 |
| 50 kt | 14.6 | 11.2 |
| 55 kt | 16.5 | 12.6 |
| 60 kt | 18.6 | 14.2 |
| 70 kt | 23.1 | 18.2 |

The port matches the first 20 kt and then accelerates faster: it reaches 70 kt in 18.2 s against 23.1 s
(about 21 % sooner). This is the assumed thrust law (`PROP_EFFICIENCY`, static cap) not falling off with speed
as the original does, or rolling drag that is too low. The reference liftoff (height above ground over 3 ft)
came 35.5 s after the roll started at 88 kias, but that depends on when the yoke was pulled and says nothing
about the model. The run is one flight with unrepeated pilot inputs, so the numbers are indicative, not a
tolerance. The reference aircraft left the runway sideways in the roll (no rudder input), which the port's
straight run does not show.

## After calibrating the thrust law (same day)
`PROP_SPEED_OFFSET` raised from 22 to 38 m/s. Roll times, reference vs port: 10 kt 1.7/2.0 s, 20 kt 4.2/4.3, 30 kt 7.5/7.0, 40 kt 10.9/10.1, 50 kt 14.6/13.7, 55 kt 16.5/15.7, 60 kt 18.6/18.0, 70 kt 23.1/24.1: within 1.0 s everywhere (mean 0.62 s), down from 5 s at 70 kt. This is a one-parameter fit to a single recording, not the original propeller model; the regression test `full_throttle_roll_follows_the_recorded_original` allows 2 s. Liftoff, climb and landing are not compared.

## Second recording: roll, liftoff and the first minute of climb (same day, 21:43 UTC+3 session)
`Resume last flight` (same C172 at UHWW 07L, night), engines started from the Flight menu, `V`, `F3`, mouse yoke
pulled gently at 22 s and returned to the centre at 45 s. Data Output flags as before, prefs restored (sha256
`1E75406E…` verified). 6540 samples (20 Hz); excerpt at 1 s steps in `research/reference/c172_climb_uhww_20261030.csv`
(mission time, IAS, VVI, pitch, alpha, gear height above ground, pressure altitude).

- **The roll repeats.** IAS 20/40/55/60/70 kt came 4.1/10.9/16.3/18.5/23.1 s after the roll started, against
  4.2/10.9/16.5/18.6/23.1 s in the first recording: the original's roll is reproducible to 0.2 s even at night and
  with different pilot timing, so the roll reference is sound and the calibration above holds against both runs.
- **Liftoff** (gear height above 0.5 ft) 33.9 s after the roll started at 86 kias, pitch 2.9°.
- **Climb** with a mouse yoke is a phugoid driven by the pilot (pitch 3°→18°→3°, IAS 68–90 kt, VVI 0–2000 fpm over
  60 s, 836 ft at 60 s), so this stretch is **not** a trajectory that the port can be compared with sample by
  sample: the inputs were not recorded. It becomes usable only when the controls are logged with it or replaced by
  a fixed trim/step input. Cruise, stall and landing are still not recorded; Phase 1's exit check stays open.
