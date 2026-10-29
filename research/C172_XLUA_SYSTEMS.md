# Cessna 172 SP: system rules in the stock xlua scripts

Source (read in place, nothing copied): `Aircraft/Laminar Research/Cessna 172 SP/plugins/xlua/scripts/`
(`c172_init`, `c172_electrical`, `c172_starter`, `c172_fuel_selector`, `c172_IAS`, `c172_sound`,
`c172_custom_datarefs`; 574 lines in total). This is the original's own definition of the electrical
start state, so it replaces guessing for Phase 5 item 2.

## Start state (c172_init, `flight_start`)
- Branch key: `sim/operation/prefs/startup_running` (cold-and-dark vs engine running) and
  `sim/aircraft/electrical/num_batteries` (1 = analog Cessna, 2 = G1000).
- Cold and dark: `cross_tie = 0`, `battery_on[1] = 0`, `fuel_pump_on[0] = 0`.
- Running, analog: `cross_tie = 1`, `fuel_pump_on[0] = 0`. Running, G1000: `cross_tie = 0`,
  `battery_on[1] = 1`, `fuel_pump_on[0] = 0`.
- `aircraft_load`: `instrument_brightness_ratio[0..3] = 0.4, 0.75, 0.75, 0.2`;
  `panel_brightness_ratio[3] = 0.3`.
- Every frame: autoboard/autostart in progress turns `cross_tie` on in the analog Cessna; in the G1000
  the glareshield brightness follows `instrument_brightness_ratio_manual[0]`.

## Electrical logic (c172_electrical)
- G1000 only: `cross_tie = 1` while `bus_volts[0] > max_bat_volt_standard` (ACF value), else 0.
- Master switch interlock: when `battery_on[0]` changes to 0, `generator_on[0]` is forced to 0; when
  `generator_on[0]` changes to 1, `battery_on[0]` is forced to 1.
- The amps needle `laminar/c172/electrical/battery_amps` low-passes `battery_amps[0]` with rate
  `10 / s`.

## Not in the scripts
`battery_on[0]` and `generator_on[0]` at start, and the bus voltage and amps values themselves, come
from the native electrical model and the start mode, so they still need a reference recording (see
`research/LIVE_DATAREF_ACCESS.md`). The scripts only fix the interlock, cross-tie and the values above.

## Ported so far
- `xp_sim::electrical`: start switches, interlock, brightness defaults (no bus volts/amps).
- `xp_sim::flight::c172_pilot_ias`: the `c172_IAS` ramp (0 below 15 kt, `(CAS-15)*70/55` to 70 kt).
- `xp_sim::fuel`: tank selector steps (left, all, right), cutoff and the handle datarefs of
  `c172_fuel_selector`. Start position "all" is assumed; the commands are not bound to keys and fuel
  flow is not modelled.
- `xp_sim::flight::c172_flap_side_shift` and `laminar/c172/knob_OAT` (starts on volts, value 2).
- `xp_sim::ignition` (state in `Controls.ignition`): the `c172_starter` key steps and starter hold (spring-back inferred; engine ignores it), plus `magnetos_off`/`magnetos_both` setting the key directly.
- `c172_sound` needs no port: it only copies ten `audio_volume_*` radio datarefs (nav1-4, adf1-2, dme, dme1-2,
  marker) into `laminar/C172/sound/radio_volume_pilot[0..9]` every frame, in that order. It matters once the
  radios and sound exist (Phase 5 item 3, Phase 6).
- Not ported: the ADF frequency clamps (200..1799 for the active and standby frequency) and
  `knob_TAS`/`knob_EGT` of `c172_custom_datarefs`; they need radio and gauge state that does not exist yet.
