# The default keyboard map

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the EXE only.

## Where it is

The default keys are part of the command table ([COMMANDS.md](COMMANDS.md)). Each 40-byte record has a flag
word; its low dword is `3` in every record, and its high dword is **the default key of the command** (0 = no
default key). 71 of the 3012 records have one, and no key is used twice.

## The key codes

The codes are the XPLM virtual key codes, which follow the Windows virtual-key numbering, plus a block of
punctuation keys:

| Codes | Keys |
|---|---|
| `0x08 0x09 0x0D 0x1B 0x20` | Backspace, Tab, Return, Escape, Space |
| `0x21..0x28`, `0x2D 0x2E` | Page Up/Down, End, Home, the arrows, Insert, Delete |
| `0x30..0x39`, `0x41..0x5A` | digits and upper-case letters |
| `0x60..0x69`, `0x6A..0x6F` | numpad digits and operators |
| `0x70..0x87` | F1..F24 |
| `0xB0..0xBD` | `=` `-` `]` `[` `'` `;` `\\` `,` `/` `.` `` ` `` Enter NumpadEnter Numpad= |

The reading is supported by the data itself: F1..F19 are bound to engine commands in an unbroken sequence
(`throttle_down` F1, `throttle_up` F2, `throttle_full` F3, then horizontal throttle, prop, mixture, carb heat,
cowl flaps, magnetos, starters), `pause_toggle` is `P` (`0x50`), `show_menu` is Escape (`0x1B`),
`time_down`/`time_up` are `K`/`L`, `zoom_in`/`zoom_out` are `=`/`-`, the two pitch trim commands are `]`/`[`,
the aileron and rudder trims sit on the digit row (`5 6 7` and `8 9 0`) and flaps are `1`/`2`. The punctuation
block `0xB0..0xBD` follows the layout of the public XPLM key constants; those values were not independently
confirmed beyond the pairs above (`= -`, `] [`, `` ` `` for the flight configuration window).

`tools/extract_keymap.py` writes the table as `assets/keymap/default_keys.tsv` (command, key name, code,
description); `src/keymap.rs` embeds it.

## What is not in the table

The default modifier keys (Shift, Ctrl, Alt), if the original uses any, are not part of this record and are not
reproduced. There is no default keyboard stick: the arrows are bound to moving the view, the original flies with
a joystick or the mouse yoke. openXplane's keyboard stick (arrows, Z/X) is therefore an addition, switchable with
Tab ([../docs/KEYBOARD.md](../docs/KEYBOARD.md)). User re-binding (the original's `Joystick, Keys & Nullzone`
settings and its key file) was not analysed.

## Implementation and limits

`src/keymap.rs` parses and indexes the table (key name to command and back). `src/pilot.rs` maps the commands that
mean something to the approximate flight model onto the aircraft controls: throttle, flaps, brakes, the three
trims, pause and the camera. The rest report "not simulated". The step sizes ("a bit" of throttle or trim) are
openXplane's choice.

## Checks

75 unit tests pass. The new ones check that the embedded table has 71 bindings and spot values (`P` = pause,
`F2` = throttle up, `1` = flaps up, `[` = pitch trim down), that F1..F19 follow the virtual key sequence, that the
parser rejects malformed rows and duplicate keys, and that each pilot command changes the controls as documented
(throttle clamps, flaps move one detent per press, brakes hold and toggle, trims step, clamp and centre). A flight
test confirms that positive elevator trim pitches the aircraft up.
