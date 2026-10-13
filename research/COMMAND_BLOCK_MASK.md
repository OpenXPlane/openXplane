# Native command block-mask query

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Complete query and exemptions

Function `0x140c63f00(command, source_mask)` returns false for exact low IDs selected by bitset `0x11f08a000e` (1, 2, 3, 17, 19, 23, 28, 29, 30, 31, 32, 36), IDs `0xb44`, `0xb4a`, `0xb4c` selected by bitset 0x141, and the inclusive range `0xb8a..0xc10`. Native range comparisons use wrapping unsigned subtraction; the lookup itself uses signed command keys.

For other commands, the function looks in the ordered map headed by global `0x145777510`: the nil byte is node+0x19, signed key is +0x1c and value is +0x20. A present key supplies its raw 32-bit mask through existing-key accessor `0x14071d590`; a missing key uses default global `0x145777508`. A zero source mask tests whether any effective mask bit is set. A nonzero source mask removes only that source's bits, testing `(mask & ~source_mask) != 0`.

`xp-dataref::command_gate` ports the complete query with explicit default/per-command state. Mask assignments and their lifecycle remain unported. The query does not establish which UI or engine situation sets each bit.

## Placement in the Esc route

Native Begin dispatcher `0x140c7fac0` first checks the separate global block byte `0x145a0df02`, then invokes this query at `0x140c7fae7`. Thus show_menu ID 3, toggle_main_menu ID 29 and toggle_settings_window ID 30 bypass this per-command mask query, but do not bypass the global block byte, earlier keyboard gates or UI/plugin interception. Exemption is not proof of an opening action.

After the query, Begin updates counters/timestamps, checks an override map through `0x14180d3f0`, processes observer callbacks and plugin gates, and conditionally invokes the registered primary callback. Those bodies/order are not yet ported as a complete live executor.

The candidate bulk registration loop `0x1414c0a70` skips ID 3 via bitset `0x1dfff800878`; its object cannot be assigned as show_menu's primary handler on that evidence. A second nearby registration routine (`0x1414c1320`) explicitly registers IDs 4/5/6 and several higher IDs, not 3. The examined candidate vtable `0x1426a2138` selects `0x1414c0c00`; that observation does not identify the show_menu action. The actual ID-3 observer/primary path still requires recovery. These are static exclusions, not executed menu behavior.

## Original-code comparison

`tools/gen_command_gate_vectors.py` executes the entire query and the found-key accessor unchanged, with no stubs. 6418 local cases include every ID in the validated 3138-record catalogue with a full default block mask, exemption/range boundaries, signed command extremes, missing/present individual masks, zero/full/overlapping source masks and seeded ordered-map contexts. The found-key accessor never needs insertion because its caller checks membership first. Allocation failures, concurrent map mutations and mask initialization are outside this comparison.

```sh
python tools/gen_command_gate_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-command-gate.txt
OPENXPLANE_COMMAND_GATE_VECTORS=/private/tmp/openxplane-command-gate.txt cargo test --offline -p xp-dataref --test command_gate_local -- --ignored
```

The corpus remains local and the comparison is ignored in ordinary CI. Clippy over workspace/all targets also passed. No engine/menu runtime, screenshots or motion fidelity are established.

Next: reconstruct complete Begin/End callback dispatch and trace its ID-3 observer/override/primary registration to the real show_menu action. Connect the verified input/state/services to the viewer only with recovered effects; Alt activation, original menu rendering and paired captures remain work.
