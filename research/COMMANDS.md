# Command table and the command registry

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the EXE only;
the original was not run.

## Record layout

The built-in commands sit in `.data` as consecutive 40-byte records (5 qwords). Observed around
`sim/flight_controls/flaps_up` (`0x142ee57d0 ..`):

| Qword | Content |
|---|---|
| 0 | VA of an internal name in `.rdata`, always `cmnd_...` (for example `cmnd_flaps_up`) |
| 1 | VA of the command name (`sim/flight_controls/flaps_up`) |
| 2 | VA of the description (`Flaps up a notch.`) |
| 3 | low dword `3` in every record found; high dword nonzero for some records |
| 4 | zero |

`tools/extract_commands.py` enumerates every record whose three pointers reach printable strings in `.rdata`,
whose first string starts with `cmnd_`, whose second contains `/` and no space, and whose last qword is zero.
These are byte-pattern candidates. On the reference build it finds 3012 records with no duplicate command name:

| Prefix | Commands |
|---|---:|
| `sim/GPS` | 384 |
| `sim/flight_controls` | 301 |
| `sim/ice` | 262 |
| `sim/radios` | 253 |
| `sim/instruments` | 248 |
| `sim/autopilot` | 145 |

The high dword of qword 3 is zero in 2941 records and nonzero in 71. In the examples read it is `0x31` for
`flaps_up`, `0x32` for `flaps_down`, `0x71` for `throttle_up` and `0x50` for `pause_toggle`, which resembles a
default key code, but this was not confirmed against the key binding code and is stored only as a hint. The
low dword `3` is not interpreted either. The listing (name, internal name, flag word, description) is written
to `research/local/commands.tsv` and is not published:

```sh
python3 tools/extract_commands.py Xplane12/X-Plane.exe > research/local/commands.tsv
python3 tools/extract_commands.py Xplane12/X-Plane.exe --prefix sim/engines/
```

Commands created by scripts (for example `laminar/c172/...`) are not in this table; they are registered at
runtime. The handler tables behind the built-in commands were not analysed.

## Implementation and limits

`src/commands.rs` has `CommandRegistry`. Commands have the phases begin, continue and end; `once` is begin
followed by end. A command carries a chain of handlers; a handler returns whether the next one should run.
`load_catalog` reads the TSV above, all or nothing; the engine itself contains no original names.

The following are openXplane policy, not behaviour confirmed in the reference build: handlers run in
registration order; beginning an already active command, or continuing/ending an inactive one, is an error; a
failed dispatch does not change the active state. How the original orders handlers and treats such sequences is
not established.

```sh
cargo run --offline -- commands research/local/commands.tsv sim/flight_controls/flaps
```

Not done: connecting commands to aircraft state (flaps, throttle, gear), keyboard and joystick bindings, and
the original's own handlers for the built-in commands.

## Checks

38 unit tests pass, Clippy without warnings, offline build. The new tests check the phase sequence and misuse,
`once`, stopping the handler chain, catalog validation and all-or-nothing loading. The command lists 12 flaps
commands for the prefix `sim/flight_controls/flaps` out of 3012.
