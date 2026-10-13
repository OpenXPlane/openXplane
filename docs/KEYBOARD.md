# Keyboard

openXplane uses the original X-Plane 12 default keyboard map. It is read from the command table of the
reference build (182 commands have a default key; see [research/KEYMAP.md](../research/KEYMAP.md)), so the keys
below preserve the original table, including Shift/Alt/Ctrl masks. Of them, 30 act in the approximate flight
viewer today. Most other commands report "not simulated"; four ambiguous chords are listed but not dispatched
until the recovered stateful native loop is connected to the viewer. Many command handlers, instruments and maps remain unfinished.

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
| Shift+Space | `sim/operation/screenshot` — Take a screenshot. | not simulated |
| Ctrl+Space | `sim/operation/video_record_toggle` — Toggle AVI movie recording. | not simulated |
| Shift+Alt+Ctrl+Space | `sim/operation/make_panel_previews` — Make screenshots of your panels. | not simulated |
| Left | `sim/general/left` — Move view left. | move the view left (arrows, when the keyboard stick is off) |
| Shift+Left | `sim/general/left_fast` — Move view left fast. | not simulated |
| Ctrl+Left | `sim/general/left_slow` — Move view left slow. | not simulated |
| Up | `sim/general/up` — Move view up. | move the view up (when the keyboard stick is off) |
| Shift+Up | `sim/general/up_fast` — Move view up fast. | not simulated |
| Ctrl+Up | `sim/general/up_slow` — Move view up slow. | not simulated |
| Right | `sim/general/right` — Move view right. | move the view right (when the keyboard stick is off) |
| Shift+Right | `sim/general/right_fast` — Move view right fast. | not simulated |
| Ctrl+Right | `sim/general/right_slow` — Move view right slow. | not simulated |
| Down | `sim/general/down` — Move view down. | move the view down (when the keyboard stick is off) |
| Shift+Down | `sim/general/down_fast` — Move view down fast. | not simulated |
| Ctrl+Down | `sim/general/down_slow` — Move view down slow. | not simulated |
| 0 | `sim/flight_controls/aileron_trim_right` — Roll trim right. | aileron trim +1% |
| Shift+0 | `sim/view/3d_cockpit_cmnd_look` — 3-D cockpit. | not simulated |
| Ctrl+0 | `sim/radios/nav2_standy_flip` — NAV 2 flip standby. | not simulated |
| 1 | `sim/flight_controls/flaps_up` — Flaps up a notch. | flaps one detent up |
| Shift+1 | `sim/view/linear_spot` — Linear spot. | not simulated |
| Ctrl+1 | `sim/starters/engage_starter_1` — Engage starter #1. | not simulated |
| 2 | `sim/flight_controls/flaps_down` — Flaps down a notch. | flaps one detent down |
| Shift+2 | `sim/view/still_spot` — Still spot. | not simulated |
| Ctrl+2 | `sim/starters/engage_starter_2` — Engage starter #2. | not simulated |
| 3 | `sim/flight_controls/speed_brakes_up_one` — Speedbrakes retract one. | not simulated |
| Shift+3 | `sim/view/runway` — Runway. | not simulated |
| Ctrl+3 | `sim/starters/engage_starter_3` — Engage starter #3. | not simulated |
| 4 | `sim/flight_controls/speed_brakes_down_one` — Speedbrakes extend one. | not simulated |
| Shift+4 | `sim/view/circle` — Circle. | not simulated |
| Ctrl+4 | `sim/starters/engage_starter_4` — Engage starter #4. | not simulated |
| 5 | `sim/flight_controls/rudder_trim_left` — Yaw trim left. | rudder trim -1% |
| Shift+5 | `sim/view/tower` — Tower. | not simulated |
| Ctrl+5 | `sim/starters/engage_starter_5` — Engage starter #5. | not simulated |
| 6 | `sim/flight_controls/rudder_trim_center` — Rudder trim center. | rudder trim to zero |
| Shift+6 | `sim/view/ridealong` — Ride-along. | not simulated |
| Ctrl+6 | `sim/starters/engage_starter_6` — Engage starter #6. | not simulated |
| 7 | `sim/flight_controls/rudder_trim_right` — Yaw trim right. | rudder trim +1% |
| Shift+7 | `sim/view/track_weapon` — Track weapon. | not simulated |
| Ctrl+7 | `sim/starters/engage_starter_7` — Engage starter #7. | not simulated |
| 8 | `sim/flight_controls/aileron_trim_left` — Roll trim left. | aileron trim -1% |
| Shift+8 | `sim/view/chase` — Chase. | not simulated |
| Ctrl+8 | `sim/starters/engage_starter_8` — Engage starter #8. | not simulated |
| 9 | `sim/flight_controls/aileron_trim_center` — Aileron trim center. | aileron trim to zero |
| Shift+9 | `sim/view/wingman` — Wingman. | not simulated |
| Ctrl+9 | `sim/radios/nav1_standy_flip` — NAV 1 flip standby. | not simulated |
| B | `sim/flight_controls/brakes_regular` — Hold brakes regular. | brakes at 50% while held |
| C | `sim/view/free_camera` — Free camera. | free camera (stops following the heading) |
| Shift+C | `sim/view/g_loaded_camera` — G-loaded camera (internal views). | not simulated |
| D | `sim/instruments/DG_sync_mag` — vacuum DG sync to magnetic north. | not simulated |
| Ctrl+D | `sim/flight_controls/landing_gear_down` — Landing gear down. | not simulated |
| E | `sim/general/rot_right` — Rotate view: pan right. | rotate the view right |
| Shift+E | `sim/general/rot_right_fast` — Rotate view: pan right fast. | not simulated |
| Ctrl+E | `sim/general/rot_right_slow` — Rotate view: pan right slow. | not simulated |
| Shift+Ctrl+E | `sim/operation/auto_start` — Auto-start engines to running, real-time. | not simulated |
| F | `sim/general/rot_down` — Rotate view: tilt down. | tilt the view down |
| Shift+F | `sim/general/rot_down_fast` — Rotate view: tilt down fast. | not simulated |
| Ctrl+F | `sim/operation/fail_system` — Fail selected in failures screen. | ambiguous original chord; not dispatched |
| Ctrl+F | `sim/general/rot_down_slow` — Rotate view: tilt down slow. | ambiguous original chord; not dispatched |
| Shift+Ctrl+F | `sim/operation/show_fps` — Toggle on-screen frame-rate display. | not simulated |
| G | `sim/flight_controls/landing_gear_toggle` — Landing gear toggle. | not simulated |
| Ctrl+G | `sim/operation/toggle_data_output_graph` — Toggle display of the data output graph. | not simulated |
| Shift+H | `sim/view/cinema_verite` — Hand-held camera (external views). | not simulated |
| I | `sim/map/show_instructor_operator_station` — Toggle the instructor operator station (IOS) window. | not simulated |
| K | `sim/operation/time_down` — Time: a little earlier. | not simulated |
| Shift+K | `sim/operation/time_down_lots` — Time: a lot earlier. | not simulated |
| L | `sim/operation/time_up` — Time: a little later. | not simulated |
| Shift+L | `sim/operation/time_up_lots` — Time: a lot later. | not simulated |
| M | `sim/map/show_current` — Toggle the sectional map window. | not simulated |
| Shift+M | `sim/operation/cycle_dump` — Print out a cycle dump for this frame. | not simulated |
| Alt+M | `sim/operation/toggle_weather_map` — Toggle display of the weather map. | not simulated |
| Ctrl+M | `sim/view/show_physics_model` — Toggle physics model visualization. | not simulated |
| Shift+N | `sim/view/night_vision` — Night vision. | not simulated |
| P | `sim/operation/pause_toggle` — Toggle simulation paused state. | pause |
| Shift+P | `sim/view/3d_path_reset` — 3-D path reset. | not simulated |
| Ctrl+P | `sim/view/3d_path_toggle` — 3-D path toggle. | not simulated |
| Q | `sim/general/rot_left` — Rotate view: pan left. | rotate the view left |
| Shift+Q | `sim/general/rot_left_fast` — Rotate view: pan left fast. | not simulated |
| Shift+Alt+Q | `sim/flight_controls/tailhook_down` — Tailhook down. | not simulated |
| Ctrl+Q | `sim/general/rot_left_slow` — Rotate view: pan left slow. | not simulated |
| R | `sim/general/rot_up` — Rotate view: tilt up. | tilt the view up |
| Shift+R | `sim/general/rot_up_fast` — Rotate view: tilt up fast. | not simulated |
| Alt+R | `sim/replay/replay_toggle` — Toggle replay mode on/off. | not simulated |
| Ctrl+R | `sim/general/rot_up_slow` — Rotate view: tilt up slow. | not simulated |
| Shift+S | `sim/view/sunglasses` — Sunglasses. | not simulated |
| Ctrl+S | `sim/ground_ops/service_plane` — Service the airplane with ground trucks. | not simulated |
| Alt+T | `sim/operation/ground_speed_change` — Sim 1x 2x 4x ground speed. | not simulated |
| Ctrl+T | `sim/operation/flightmodel_speed_change` — Sim 1x 2x 4x sim speed. | not simulated |
| Ctrl+U | `sim/flight_controls/landing_gear_up` — Landing gear up. | not simulated |
| V | `sim/flight_controls/brakes_toggle_max` — Toggle brakes max effort. | maximum brakes on/off |
| W | `sim/view/default_view` — Default view. | default chase view |
| Shift+W | `sim/view/forward_with_hud` — Forward with HUD. | not simulated |
| Alt+W | `sim/view/forward_with_2d_panel` — Forward with 2-D panel. | not simulated |
| Shift+Alt+W | `sim/flight_controls/tailhook_up` — Tailhook up. | not simulated |
| Ctrl+W | `sim/view/forward_with_nothing` — Forward with nothing. | not simulated |
| X | `sim/flight_controls/smoke_toggle` — Toggle smoke puffing. | not simulated |
| Y | `sim/operation/toggle_yoke` — Toggle yoke visibility. | not simulated |
| Numpad0 | `sim/view/quick_look_0` — Go to save 3-D cockpit location #1. | not simulated |
| Ctrl+Numpad0 | `sim/view/quick_look_0_mem` — Memorize 3-D cockpit location #1. | not simulated |
| Numpad1 | `sim/view/quick_look_1` — Go to save 3-D cockpit location #2. | not simulated |
| Ctrl+Numpad1 | `sim/view/quick_look_1_mem` — Memorize 3-D cockpit location #2. | not simulated |
| Numpad2 | `sim/view/quick_look_2` — Go to save 3-D cockpit location #3. | not simulated |
| Ctrl+Numpad2 | `sim/view/quick_look_2_mem` — Memorize 3-D cockpit location #3. | not simulated |
| Numpad3 | `sim/view/quick_look_3` — Go to save 3-D cockpit location #4. | not simulated |
| Ctrl+Numpad3 | `sim/view/quick_look_3_mem` — Memorize 3-D cockpit location #4. | not simulated |
| Numpad4 | `sim/view/quick_look_4` — Go to save 3-D cockpit location #5. | not simulated |
| Ctrl+Numpad4 | `sim/view/quick_look_4_mem` — Memorize 3-D cockpit location #5. | not simulated |
| Numpad5 | `sim/view/quick_look_5` — Go to save 3-D cockpit location #6. | not simulated |
| Ctrl+Numpad5 | `sim/view/quick_look_5_mem` — Memorize 3-D cockpit location #6. | not simulated |
| Numpad6 | `sim/view/quick_look_6` — Go to save 3-D cockpit location #7. | not simulated |
| Ctrl+Numpad6 | `sim/view/quick_look_6_mem` — Memorize 3-D cockpit location #7. | not simulated |
| Numpad7 | `sim/view/quick_look_7` — Go to save 3-D cockpit location #8. | not simulated |
| Ctrl+Numpad7 | `sim/view/quick_look_7_mem` — Memorize 3-D cockpit location #8. | not simulated |
| Numpad8 | `sim/view/quick_look_8` — Go to save 3-D cockpit location #9. | not simulated |
| Ctrl+Numpad8 | `sim/view/quick_look_8_mem` — Memorize 3-D cockpit location #9. | not simulated |
| Numpad9 | `sim/view/quick_look_9` — Go to save 3-D cockpit location #10. | not simulated |
| Ctrl+Numpad9 | `sim/view/quick_look_9_mem` — Memorize 3-D cockpit location #10. | not simulated |
| F1 | `sim/engines/throttle_down` — Throttle down a bit. | throttle -5% (repeats while held) |
| Shift+F1 | `sim/operation/slider_01` — Slider #1 On/Off control. | not simulated |
| F2 | `sim/engines/throttle_up` — Throttle up a bit. | throttle +5% (repeats while held) |
| Shift+F2 | `sim/operation/slider_02` — Slider #2 On/Off control. | not simulated |
| F3 | `sim/engines/throttle_full` — Throttle to wide open! | throttle to full |
| Shift+F3 | `sim/operation/slider_03` — Slider #3 On/Off control. | not simulated |
| F4 | `sim/engines/throttle_horizontal_down` — Throttle horizontal down a bit. | not simulated |
| Shift+F4 | `sim/operation/slider_04` — Slider #4 On/Off control. | not simulated |
| Alt+F4 | `sim/operation/quit` — Quit X-Plane. | not simulated |
| F5 | `sim/engines/throttle_horizontal_up` — Throttle horizontal up a bit. | not simulated |
| Shift+F5 | `sim/operation/slider_05` — Slider #5 On/Off control. | not simulated |
| F6 | `sim/engines/prop_down` — Prop coarse a bit. | not simulated |
| Shift+F6 | `sim/operation/slider_06` — Slider #6 On/Off control. | not simulated |
| F7 | `sim/engines/prop_up` — Prop fine a bit. | not simulated |
| Shift+F7 | `sim/operation/slider_07` — Slider #7 On/Off control. | not simulated |
| F8 | `sim/engines/mixture_min` — Mixture to cut off. | not simulated |
| Shift+F8 | `sim/operation/slider_08` — Slider #8 On/Off control. | not simulated |
| F9 | `sim/engines/mixture_down` — Mixture lean a bit. | not simulated |
| Shift+F9 | `sim/operation/slider_09` — Slider #9 On/Off control. | not simulated |
| F10 | `sim/engines/mixture_up` — Mixture rich a bit. | not simulated |
| Shift+F10 | `sim/operation/slider_10` — Slider #10 On/Off control. | not simulated |
| F11 | `sim/engines/mixture_max` — Mixture to full rich. | not simulated |
| Shift+F11 | `sim/operation/slider_11` — Slider #11 On/Off control. | not simulated |
| F12 | `sim/engines/carb_heat_off` — Carb heat off. | not simulated |
| Shift+F12 | `sim/operation/slider_12` — Slider #12 On/Off control. | not simulated |
| F13 | `sim/engines/carb_heat_on` — Carb heat on. | not simulated |
| Shift+F13 | `sim/operation/slider_13` — Slider #13 On/Off control. | not simulated |
| F14 | `sim/engines/carb_heat_toggle` — Carb heat toggle. | not simulated |
| Shift+F14 | `sim/operation/slider_14` — Slider #14 On/Off control. | not simulated |
| F15 | `sim/flight_controls/cowl_flaps_open` — Move cowl flaps open a bit. | not simulated |
| Shift+F15 | `sim/operation/slider_15` — Slider #15 On/Off control. | not simulated |
| F16 | `sim/flight_controls/cowl_flaps_closed` — Move cowl flaps to closed a bit. | not simulated |
| Shift+F16 | `sim/operation/slider_16` — Slider #16 On/Off control. | not simulated |
| F17 | `sim/magnetos/magnetos_off` — Magnetos off. | not simulated |
| F18 | `sim/magnetos/magnetos_both` — Magnetos both. | not simulated |
| F19 | `sim/engines/engage_starters` — Engage starters. | not simulated |
| = | `sim/general/zoom_in` — Zoom in. | zoom in |
| Shift+= | `sim/general/zoom_in_fast` — Zoom in fast. | not simulated |
| Ctrl+= | `sim/radios/com2_standy_flip` — COM 2 flip standby. | ambiguous original chord; not dispatched |
| Ctrl+= | `sim/general/zoom_in_slow` — Zoom in slow. | ambiguous original chord; not dispatched |
| - | `sim/general/zoom_out` — Zoom out. | zoom out |
| Shift+- | `sim/general/zoom_out_fast` — Zoom out fast. | not simulated |
| Ctrl+- | `sim/radios/com1_standy_flip` — COM 1 flip standby. | ambiguous original chord; not dispatched |
| Ctrl+- | `sim/general/zoom_out_slow` — Zoom out slow. | ambiguous original chord; not dispatched |
| ] | `sim/flight_controls/pitch_trim_up` — Pitch trim up. | elevator trim +1% |
| Shift+] | `sim/weapons/weapon_target_up` — Target select up. | not simulated |
| [ | `sim/flight_controls/pitch_trim_down` — Pitch trim down. | elevator trim -1% |
| Shift+[ | `sim/weapons/weapon_target_down` — Target select down. | not simulated |
| Shift+; | `sim/flight_controls/drop_tank` — Drop all drop tanks. | not simulated |
| Ctrl+; | `sim/operation/reset_flight` — Reset flight to most recent start. | not simulated |
| , | `sim/general/backward` — Move view backward. | move the camera back |
| Shift+, | `sim/general/backward_fast` — Move view backward fast. | not simulated |
| Ctrl+, | `sim/operation/test_data_ref` — Test dataref: Run from 0 to 1 and back. | ambiguous original chord; not dispatched |
| Ctrl+, | `sim/general/backward_slow` — Move view backward slow. | ambiguous original chord; not dispatched |
| Shift+Ctrl+, | `sim/instruments/panel_bright_down` — Panel brightness down a bit. | not simulated |
| / | `sim/engines/beta_toggle` — Toggle Beta prop. | not simulated |
| Shift+/ | `sim/engines/thrust_reverse_toggle` — Toggle thrust reversers. | not simulated |
| . | `sim/general/forward` — Move view forward. | move the camera closer |
| Shift+. | `sim/general/forward_fast` — Move view forward fast. | not simulated |
| Ctrl+. | `sim/general/forward_slow` — Move view forward slow. | not simulated |
| Shift+Ctrl+. | `sim/instruments/panel_bright_up` — Panel brightness up a bit. | not simulated |
| ` | `sim/operation/toggle_flight_config` — Toggle the Flight Configuration window. | not simulated |
| Ctrl+` | `sim/operation/toggle_settings_window` — Toggle the Settings window. | not simulated |

Notes: Shift=1, Alt=2 and Ctrl=4 come from the original modifier parser. The viewer uses exact chord matching
and retains the press chord through repeat/release, even if modifiers change while a key is held. This lifecycle
and focus-loss release policy have not been compared with the native dispatcher. Super/Command is not mapped
to Ctrl. Alt by itself has no command-table binding; native menu-bar handling remains unresolved.
Step sizes of "a bit" commands are openXplane's choice; the gear is fixed, so the gear key does nothing.
