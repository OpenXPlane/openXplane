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
- **Climb** (pitch 3°→18°→3°, IAS 68–90 kt, VVI 0–2000 fpm over 60 s, 836 ft at 60 s): see the correction below,
  the mouse yoke did not act, so this is the free response of the aircraft.

## Third recording: no control input at all, and the first real mismatch of the port
The same flight again with **no** yoke movement (the script only starts the engines, releases the brake, `F3`).
The result equals the second recording within 0.2 s and 1–2 ft per 5 s (roll 4.2/10.9/16.4/18.5/23.1 s, liftoff
33.8 s at 86 kias, 783 ft at 60 s after liftoff against 770 ft), so the mouse yoke of the second run never acted:
the climb above is the aircraft's own, reproducible response to trim and full power with a free stick. That makes
it a usable reference (`research/reference/c172_noinput_uhww_20261030.csv`, 1 s steps).

The port (`fly-test`, same procedure, 140 s) does **not** reproduce it:

| | Reference | Port |
|---|---|---|
| liftoff | 33.8 s after the roll starts, 86 kias, pitch 2.9° | 24.1 s, 67 kt |
| after liftoff | pitch 11° → 18° → 3° → 12° → 15°, IAS 68–90 kt, VVI 0–2000 fpm (a damped phugoid-like swing, period about 25 s) | monotone: pitch 7.2°, IAS 86 kt, VVI 490 fpm from about 60 s |
| altitude 60 s after liftoff | 770–857 ft | about 850 ft at 114 s |

Two differences stand out: the original holds the nose wheel down until 86 kias (the port lifts at 67 kt), so the
pitch moment at the start (trim, elevator, ground handling) is not that of the original; and the port has far
too little pitch dynamics (no oscillation), so its longitudinal damping/stability differs. Neither is fitted. The
Phase 1 exit check stays open; cruise, stall and landing are not recorded.

## Why the first comparison was wrong, and the trim fit
`fly-test` drove the port with `takeoff_pilot` (rotation at 55 kt, pitch held), so the table above compared a
scripted pilot with a hands-off original. With the new `OPENXPLANE_HANDS_OFF=1` (full power after 2 s, no other
input; `OPENXPLANE_ELEVATOR_TRIM=<x>` sets the trim) and zero trim the port never leaves the ground (116 kt on the
runway), while the original lifts at 86 kias: the original starts with nose-up trim. Port hands-off liftoff
against trim (since the roll starts): 0.20 → 91 kt, 0.22 → 88 kt after 35.7 s, 0.25 → 85 kt after 33.0 s,
0.30 → 80 kt after 29.5 s; original 86 kt after 33.8 s, so the fit is trim ≈ 0.24. With it the port also swings
in pitch like the original (VVI peaks about 1050 fpm, 13° at 10 s and 30 s after liftoff, period 20 s; original
2000 fpm, 18°, period about 25 s), so the shape is right and the damping/amplitude is not: the port's swing is
about half the original's. The trim of 0.24 is fitted, not read from the original; where the original takes its
start trim from is open. Test: `hands_off_takeoff_lifts_off_near_the_recorded_original` (liftoff within 4 s and 5 kt,
pitch swing above 6°).

## The original's start trim is read: 0.07, not 0.24
A fourth hands-off run with Data Output groups 8 and 13 enabled (stick positions, trim, flaps, speedbrakes; 57
columns, `Data_c172_trim.txt` in the scratch folder, not in the repo) gives, for the whole run from the cold start,
through the roll and the liftoff: elevator stick 0.00, **elevator trim 0.07**, aileron/rudder trim 0, flap handle
0. So the original lifts off on its own at 86 kias with a trim of 0.07 and a free stick, whereas the port needs 0.24
for the same liftoff (and does not leave the ground at 0.07). The fitted 0.24 is therefore a compensation for
something else missing in the port's pitch balance, about a factor 3.4 in nose-up moment at the tail: the likely
candidates are the propeller slipstream over the tail (raises elevator effectiveness and tail lift at full power
and low speed), the wing's downwash on the tail, or the ground effect. None of these is checked yet; the test
`hands_off_takeoff_lifts_off_near_the_recorded_original` pins the fit, not the cause.

## Tried and discarded: propeller slipstream over the tail
A first model (tail surfaces see `sqrt(V² + 2T/(ρ·2.9 m²))` weighted by a 0.6 share of the extra speed) moves the
fitted trim only from 0.24 to 0.20 (hands-off liftoff 85 kt after 33.5 s at 0.20; 0.07 still lifts only at 110 kt
after 75 s) and lowers the pitch swing (VVI peak about 1000 fpm, pitch 11°), so it does not explain the original's
0.07 and was not kept. The remaining candidates: the thrust line height against the CG (the port puts the
thrust at y=0 while the CG is at 0.30 m; the ACF fields with the engine position were not found by name in a first
grep), ground contact (nose gear load and tire/strut model), and the elevator/stabilizer lift model.

## Thrust point read from the ACF
The thrust now acts at the engine part position `_engn/0/_part_x/y/z` (0, 0.1, -3.72 ft = 0, 0.03, -1.13 m) instead
of the hard-coded (0, 0, -1.6) m; the CG is at y = 0.09 m, so the thrust line lies only 0.06 m below it and the moment is
small. Effect on the hands-off run: with trim 0.24 the port now lifts off 36.4 s after the start (86 kt); with 0.07 it
still does not lift. The missing nose-up moment is therefore not in the thrust line. Candidates left: the
nose-gear load/strut model and the stabilizer/elevator lift model.

## Elevator effectiveness instead of a fitted trim
Sweeping the control effectiveness constant with the original's trim of 0.07: 1.3 → liftoff after 48.8 s at 100 kt,
2.0 → 36.5 s at 89 kt, 2.5 → 31.8 s at 83 kt; the original 33.8 s at 86 kt. The elevator now uses 2.2 (the other
controls stay at 0.65): the port lifts off 34.4 s after the roll starts at 86.7 kt, and its pitch swing has the
original's period (about 25 s; peaks 5-10 s and 30 s after liftoff) but still about half the amplitude (VVI 1085
against 2000 fpm, pitch 12 against 18 degrees). 2.2/0.65 = 3.4 is the same factor as 0.24/0.07, so the earlier
trim fit and this one are the same finding: the port's elevator authority is 3.4 times too low. Whether the other
controls need the same factor is untested (no recording with aileron or rudder input yet), flaps keep 0.65.

## Long hands-off run: the original's swing grows, the port's damps
Second recording (`Data_c172_climb`, hands-off, 108 s after the roll started) against the port with trim 0.07 and
elevator effectiveness 2.2 (160 s), seconds since the roll started:

| | pitch extremes | after |
|---|---|---|
| original | 2.4° (32 s), 16.5° (42-47 s), 2.2° (57 s), 14.6° (67 s), 2.6° (82 s), then 6° at 92 s, -0.5° at 97 s, -5..-6° at 102-122 s | the swing does not die out: from 97 s IAS rises to 126 kt, VVI -1600 fpm (a diving phugoid) |
| port | 9.5° (38 s), 5.7° (78 s), 10.7° (88 s), 6.1° (98 s), 9.7° (108 s), 7.2° (118 s), 7.7° (148 s) | damps to a steady climb of about 500 fpm at 82-83 kt, pitch 8° |

Both have a period near 25 s, but the original's first peak is 16.5° against 9.5° and it never settles (the long
phugoid is neutrally stable or growing), whereas the port's is damped within about 80 s. So the port's longitudinal
damping is too high or its speed stability (the change of pitch moment with speed) too strong; the amplitude
mismatch of the climb is the early part of the same difference. Next test: reduce the damping contribution of
the tail (pitch-rate arm) and the wing downwash lag and see whether the first peak and the growth follow.
