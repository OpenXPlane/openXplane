# Native default keyboard map

Updated: 2026-10-13. Exact baseline identity is in [BASELINE.md](BASELINE.md).

## Corrected table bounds and modifiers

The previous extractor incorrectly required qword 4 to be zero and internal names to start with `cmnd_`, and required low flags to equal 3. That produced an incomplete 3012-command catalogue and only the 71 unmodified bindings. Its claim that modifiers were absent from the record was wrong.

The native initializer at `0x140c6478d..0x140c64827` walks from `0x142ee141c` (record 0 + 28) to exclusive `0x142effe6c`, stepping 40 bytes. The catalogue starts at `0x142ee1400` and has 3138 records. The earlier catalogue-copy loop at `0x140c3c981..0x140c3ca1d` independently uses the same base + 8, stride 40 and count `0xc42` (3138). Three reserved VR commands have low flag 2. Some internal names use `xmnd_` or `rep_`; they are valid native table members.

Each record contains internal-name/name/description pointers (qwords 0..2), low flags and high default key (qword 3), and a modifier mask (low dword of qword 4). The initializer copies the key to vector entry +0 and the mask to +4, then resolves the command name and stores its result at +8. Vector entries are 16 bytes. A subsequent native loop removes entries whose key is zero.

There are 182 bound commands: 71 unmodified, 52 Shift, 47 Ctrl, 5 Alt, 4 Shift+Ctrl, 2 Shift+Alt and 1 Shift+Alt+Ctrl. They use 178 distinct chords. Names, descriptions, key codes and masks now preserve native catalogue order.

## Modifier-bit evidence

Native helper `0x140c816e0` parses modifier text. It recognizes `SHIFT` and `MAJ` as bit 1; `CTRL` and `STRG` as bit 4. The second mask bit uses the string object at `0x145777648`: initializer `0x140559d00` builds it from the literal string object at `0x142f021e0`, which contains `Alt` (length 3, inline capacity 15), via normalization helper `0x141622e00`. Parsing a matching normalized Alt string ORs bit 2 at `0x140c8185c`. Formatting helper `0x140c60300` tests the same bits 1/2/4.

This establishes Shift=1, Alt=2, Ctrl=4 for the supplied Windows baseline. Super/Command mappings of another original platform are not established.

## Conflicting original chords

| Chord | Commands in native order |
|---|---|
| Ctrl+, | test_data_ref; backward_slow |
| Ctrl+F | fail_system; rot_down_slow |
| Ctrl+- | com1_standy_flip; zoom_out_slow |
| Ctrl+= | com2_standy_flip; zoom_in_slow |

The native binding-vector loop is now recovered separately; duplicate activation depends on active state and down/up flags ([KEY_EVENT_LOOP.md](KEY_EVENT_LOOP.md)). Complete input interception and callback behavior are not recovered. `bindings_for_chord` preserves both records; `command_for_chord` returns a binding only when unique. The viewer does not guess which conflicting command should run.

## Port and checks

`tools/extract_commands.py` now uses the observed native bounds and requires this exact baseline SHA256. Its four-column catalogue output remains compatible; `--modifiers` appends a fifth mask column. `tools/extract_keymap.py` generates five-column key TSV. The Rust parser also accepts legacy four-column rows as unmodified; duplicate commands are rejected but conflicting chords are retained.

The viewer reads winit modifier state and the logical key without modifiers. It matches the exact chord rather than sending Ctrl+P to unmodified P. It retains the press chord for repeats and release and releases held commands on focus loss; that event-lifecycle policy is independently implemented and not verified against original OS events. Keyboard-stick additions apply only to unmodified keys.

`tools/check_native_key_defaults.py` executes the original copy block `0x140c647a0..0x140c647b4` for every one of the 3138 table records. All 182 embedded key/mask pairs match its output exactly; there are no calls or stubs in that block. This does not replay command-name lookup, vector allocation/removal, modifier text normalization, conflict resolution or complete event dispatch. No original-output corpus is distributed; the check uses the locally supplied EXE.

Rust checks cover modified versus unmodified lookup, conflicts, mask validation, retained press chords on modifier changes and unsupported Super-key input. None is a UI screenshot or native event comparison.

Esc remains bound to `sim/operation/show_menu`; Ctrl+backtick maps to `sim/operation/toggle_settings_window`. The active viewer still exits on Esc and does not implement those original menu handlers. Alt alone has no entry here, so its native menu-bar behavior must be recovered elsewhere. Matched original screenshots and motion evidence remain outstanding. Full key rebinding and layout/platform-specific parity are not claimed.

The local 2728-case comparison of the new binding-vector loop checks active/suppression transitions and ordered Begin/End requests with callback bodies logged. Its complete viewer integration remains pending; see [KEY_EVENT_LOOP.md](KEY_EVENT_LOOP.md).
