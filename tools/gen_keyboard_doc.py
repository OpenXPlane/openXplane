#!/usr/bin/env python3
"""Generates docs/KEYBOARD.md from assets/keymap/default_keys.tsv and the list of commands that act.

    python3 tools/gen_keyboard_doc.py > docs/KEYBOARD.md
"""
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ACTS = {
    'sim/engines/throttle_up': 'throttle +5% (repeats while held)',
    'sim/engines/throttle_down': 'throttle -5% (repeats while held)',
    'sim/engines/throttle_full': 'throttle to full',
    'sim/flight_controls/flaps_up': 'flaps one detent up',
    'sim/flight_controls/flaps_down': 'flaps one detent down',
    'sim/flight_controls/brakes_regular': 'brakes at 50% while held',
    'sim/flight_controls/brakes_toggle_max': 'maximum brakes on/off',
    'sim/flight_controls/pitch_trim_up': 'elevator trim +1%',
    'sim/flight_controls/pitch_trim_down': 'elevator trim -1%',
    'sim/flight_controls/aileron_trim_left': 'aileron trim -1%',
    'sim/flight_controls/aileron_trim_right': 'aileron trim +1%',
    'sim/flight_controls/aileron_trim_center': 'aileron trim to zero',
    'sim/flight_controls/rudder_trim_left': 'rudder trim -1%',
    'sim/flight_controls/rudder_trim_right': 'rudder trim +1%',
    'sim/flight_controls/rudder_trim_center': 'rudder trim to zero',
    'sim/operation/pause_toggle': 'pause',
    'sim/view/default_view': 'default chase view',
    'sim/view/free_camera': 'free camera (stops following the heading)',
    'sim/general/left': 'move the view left (arrows, when the keyboard stick is off)',
    'sim/general/right': 'move the view right (when the keyboard stick is off)',
    'sim/general/up': 'move the view up (when the keyboard stick is off)',
    'sim/general/down': 'move the view down (when the keyboard stick is off)',
    'sim/general/rot_left': 'rotate the view left',
    'sim/general/rot_right': 'rotate the view right',
    'sim/general/rot_up': 'tilt the view up',
    'sim/general/rot_down': 'tilt the view down',
    'sim/general/zoom_in': 'zoom in',
    'sim/general/zoom_out': 'zoom out',
    'sim/general/forward': 'move the camera closer',
    'sim/general/backward': 'move the camera back',
}
rows = []
for line in (ROOT / 'assets/keymap/default_keys.tsv').read_text().splitlines():
    cmd, key, code, desc = line.split('\t')
    rows.append((key, code, cmd, desc))
rows.sort(key=lambda r: int(r[1], 16))
acts = sum(1 for r in rows if r[2] in ACTS)
print(f'''# Keyboard

openXplane uses the original X-Plane 12 default keyboard map. It is read from the command table of the
reference build ({len(rows)} commands have a default key; see [research/KEYMAP.md](research/KEYMAP.md)), so the keys
below are the original's, not guesses. Of them, {acts} act in the flight viewer today; the rest are recognised and
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
|---|---|---|''')
for key, code, cmd, desc in rows:
    effect = ACTS.get(cmd, 'not simulated')
    print(f'| {key} | `{cmd}` — {desc} | {effect} |')
print('''
Notes: the original's default modifiers (if any) are not stored in the table and are not reproduced; the step
sizes of "a bit" commands are openXplane's choice; the gear is fixed, so the gear key does nothing.''')
