# Keyboard

openXplane uses the original X-Plane 12 default keyboard map. It is read from the command table of the
reference build (71 commands have a default key; see [research/KEYMAP.md](research/KEYMAP.md)), so the keys
below are the original's, not guesses. Of them, 30 act in the flight viewer today; the rest are recognised and
reported in the window title as "not simulated" because the engine has no engine controls, magnetos,
instruments or maps yet.

```sh
cargo run --release --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

## openXplane's own keys

The original has no default keyboard stick (the aircraft is flown with a joystick or the mouse yoke), so these
are additions, not part of the ported map:

| Key | Action |
|---|---|
| Down / Up arrows | stick back (nose up) / forward |
| Left / Right arrows | roll left / right |
| Z / X | rudder left / right |
| Tab | give the arrows and X back their original meaning (view movement, smoke toggle), and back |
| Delete | reset the aircraft to the runway |
| Esc | quit (the original opens its menu) |
| Mouse drag, scroll | orbit and zoom the chase camera |

## The original's default keys

| Key | Command | Effect here |
|---|---|---|
| Return | `sim/operation/contact_atc` — Contact ATC. | not simulated |
| Escape | `sim/operation/show_menu` — Show the in-sim menu. | not simulated |
| Space | `sim/general/action` — General action command. | not simulated |
| Left | `sim/general/left` — Move view left. | move the view left (arrows, when the keyboard stick is off) |
| Up | `sim/general/up` — Move view up. | move the view up (when the keyboard stick is off) |
| Right | `sim/general/right` — Move view right. | move the view right (when the keyboard stick is off) |
| Down | `sim/general/down` — Move view down. | move the view down (when the keyboard stick is off) |
| 0 | `sim/flight_controls/aileron_trim_right` — Roll trim right. | aileron trim +1% |
| 1 | `sim/flight_controls/flaps_up` — Flaps up a notch. | flaps one detent up |
| 2 | `sim/flight_controls/flaps_down` — Flaps down a notch. | flaps one detent down |
| 3 | `sim/flight_controls/speed_brakes_up_one` — Speedbrakes retract one. | not simulated |
| 4 | `sim/flight_controls/speed_brakes_down_one` — Speedbrakes extend one. | not simulated |
| 5 | `sim/flight_controls/rudder_trim_left` — Yaw trim left. | rudder trim -1% |
| 6 | `sim/flight_controls/rudder_trim_center` — Rudder trim center. | rudder trim to zero |
| 7 | `sim/flight_controls/rudder_trim_right` — Yaw trim right. | rudder trim +1% |
| 8 | `sim/flight_controls/aileron_trim_left` — Roll trim left. | aileron trim -1% |
| 9 | `sim/flight_controls/aileron_trim_center` — Aileron trim center. | aileron trim to zero |
| B | `sim/flight_controls/brakes_regular` — Hold brakes regular. | brakes at 50% while held |
| C | `sim/view/free_camera` — Free camera. | free camera (stops following the heading) |
| D | `sim/instruments/DG_sync_mag` — vacuum DG sync to magnetic north. | not simulated |
| E | `sim/general/rot_right` — Rotate view: pan right. | rotate the view right |
| F | `sim/general/rot_down` — Rotate view: tilt down. | tilt the view down |
| G | `sim/flight_controls/landing_gear_toggle` — Landing gear toggle. | not simulated |
| I | `sim/map/show_instructor_operator_station` — Toggle the instructor operator station (IOS) window. | not simulated |
| K | `sim/operation/time_down` — Time: a little earlier. | not simulated |
| L | `sim/operation/time_up` — Time: a little later. | not simulated |
| M | `sim/map/show_current` — Toggle the sectional map window. | not simulated |
| P | `sim/operation/pause_toggle` — Toggle simulation paused state. | pause |
| Q | `sim/general/rot_left` — Rotate view: pan left. | rotate the view left |
| R | `sim/general/rot_up` — Rotate view: tilt up. | tilt the view up |
| V | `sim/flight_controls/brakes_toggle_max` — Toggle brakes max effort. | maximum brakes on/off |
| W | `sim/view/default_view` — Default view. | default chase view |
| X | `sim/flight_controls/smoke_toggle` — Toggle smoke puffing. | not simulated |
| Y | `sim/operation/toggle_yoke` — Toggle yoke visibility. | not simulated |
| Numpad0 | `sim/view/quick_look_0` — Go to save 3-D cockpit location #1. | not simulated |
| Numpad1 | `sim/view/quick_look_1` — Go to save 3-D cockpit location #2. | not simulated |
| Numpad2 | `sim/view/quick_look_2` — Go to save 3-D cockpit location #3. | not simulated |
| Numpad3 | `sim/view/quick_look_3` — Go to save 3-D cockpit location #4. | not simulated |
| Numpad4 | `sim/view/quick_look_4` — Go to save 3-D cockpit location #5. | not simulated |
| Numpad5 | `sim/view/quick_look_5` — Go to save 3-D cockpit location #6. | not simulated |
| Numpad6 | `sim/view/quick_look_6` — Go to save 3-D cockpit location #7. | not simulated |
| Numpad7 | `sim/view/quick_look_7` — Go to save 3-D cockpit location #8. | not simulated |
| Numpad8 | `sim/view/quick_look_8` — Go to save 3-D cockpit location #9. | not simulated |
| Numpad9 | `sim/view/quick_look_9` — Go to save 3-D cockpit location #10. | not simulated |
| F1 | `sim/engines/throttle_down` — Throttle down a bit. | throttle -5% (repeats while held) |
| F2 | `sim/engines/throttle_up` — Throttle up a bit. | throttle +5% (repeats while held) |
| F3 | `sim/engines/throttle_full` — Throttle to wide open! | throttle to full |
| F4 | `sim/engines/throttle_horizontal_down` — Throttle horizontal down a bit. | not simulated |
| F5 | `sim/engines/throttle_horizontal_up` — Throttle horizontal up a bit. | not simulated |
| F6 | `sim/engines/prop_down` — Prop coarse a bit. | not simulated |
| F7 | `sim/engines/prop_up` — Prop fine a bit. | not simulated |
| F8 | `sim/engines/mixture_min` — Mixture to cut off. | not simulated |
| F9 | `sim/engines/mixture_down` — Mixture lean a bit. | not simulated |
| F10 | `sim/engines/mixture_up` — Mixture rich a bit. | not simulated |
| F11 | `sim/engines/mixture_max` — Mixture to full rich. | not simulated |
| F12 | `sim/engines/carb_heat_off` — Carb heat off. | not simulated |
| F13 | `sim/engines/carb_heat_on` — Carb heat on. | not simulated |
| F14 | `sim/engines/carb_heat_toggle` — Carb heat toggle. | not simulated |
| F15 | `sim/flight_controls/cowl_flaps_open` — Move cowl flaps open a bit. | not simulated |
| F16 | `sim/flight_controls/cowl_flaps_closed` — Move cowl flaps to closed a bit. | not simulated |
| F17 | `sim/magnetos/magnetos_off` — Magnetos off. | not simulated |
| F18 | `sim/magnetos/magnetos_both` — Magnetos both. | not simulated |
| F19 | `sim/engines/engage_starters` — Engage starters. | not simulated |
| = | `sim/general/zoom_in` — Zoom in. | zoom in |
| - | `sim/general/zoom_out` — Zoom out. | zoom out |
| ] | `sim/flight_controls/pitch_trim_up` — Pitch trim up. | elevator trim +1% |
| [ | `sim/flight_controls/pitch_trim_down` — Pitch trim down. | elevator trim -1% |
| , | `sim/general/backward` — Move view backward. | move the camera back |
| / | `sim/engines/beta_toggle` — Toggle Beta prop. | not simulated |
| . | `sim/general/forward` — Move view forward. | move the camera closer |
| ` | `sim/operation/toggle_flight_config` — Toggle the Flight Configuration window. | not simulated |

Notes: the original's default modifiers (if any) are not stored in the table and are not reproduced; the step
sizes of "a bit" commands are openXplane's choice; the gear is fixed, so the gear key does nothing.
