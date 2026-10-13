# Command catalogue and registry

Updated: 2026-10-13. Baseline identity is in [BASELINE.md](BASELINE.md).

The native catalogue at `0x142ee1400` contains 3138 consecutive 40-byte records. The original loader's bounds and stride establish this count; the earlier 3012-record pattern scan was incomplete. It excluded modifier masks, internal names without the `cmnd_` prefix, and three reserved VR records with low flag 2. Evidence and default-key layout are in [KEYMAP.md](KEYMAP.md).

Qwords 0..2 contain internal name, command name and description pointers. Qword 3 holds a low flag and a high default virtual key. Qword 4 holds the default modifier mask (Shift=1, Alt=2, Ctrl=4). It is not a runtime command pointer.

`tools/extract_commands.py` validates this exact baseline SHA256 and reads all records in native order. Its usual TSV remains name/internal/key/description. `--modifiers` appends the mask. Commands registered by scripts at runtime are outside this static catalogue.

```sh
python3 tools/extract_commands.py /path/to/X-Plane.exe > research/local/commands.tsv
python3 tools/extract_commands.py /path/to/X-Plane.exe --modifiers --prefix sim/operation/
```

The original registration callbacks and full command dispatcher remain unresolved. `crates/xp-dataref/src/commands.rs` implements an independent named registry with Begin/Continue/End and handler chains. Registration order, double-begin errors and failed-dispatch active-state handling are openXplane policy; static catalogue recovery does not verify those behaviors.

The registry's four-column catalogue API retains the legacy `flag_hint` name for its default-key field. Modifier-aware default input belongs to `keymap.rs`. Implementing a catalogue entry does not implement its native handler.
