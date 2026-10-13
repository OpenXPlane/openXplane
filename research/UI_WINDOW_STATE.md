# Native window state and cumulative opacity

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The recovered main-menu handler now has a state-backed request path in `xp-app::ui_state::WindowState`. It computes its original query results from the controller map, ordered window registry, parent opacity chain and raw engine-state byte; it does not fabricate an "open" boolean. This state is still explicit input, not a live original window graph.

## Native services

`0x141512eb0` searches the signed-kind controller map at +0x148. Missing kinds and null mapped window pointers return false. A non-null pointer must also be present in the window-system registry, as checked by `0x1412e35c0` on global manager `0x145a0e308`. The manager follows +0x128 to its implementation and +0x150 to a linked-list head. Null node payloads are skipped; a payload's +0x138 token must equal the controller's mapped window pointer.

If registered, `0x1412fd160` locates the first matching list entry and reads its widget pointer at +0x140. The widget's scalar at +0x58 is multiplied by its parent's effective scalar; parent pointers are at +0x60. Recursive helper `0x1412d9430` returns a root's stored scalar directly, then performs one float32 multiplication per child on unwind. The final query is strictly product > 0. Zero, negative zero and negative products fail; two negative factors can produce a positive result. No clamping to [0,1] is introduced.

`0x1417fb320` returns the raw byte at engine-state +0x525. The menu branch tests AL for nonzero. Three observed writers store 1; another stores 0. The lifecycle/meaning of the field is not yet established, so the port keeps it as `state_byte_525` rather than inventing a loaded/paused flag.

`WindowState` models pointer identity as opaque tokens, a signed-kind map and an ordered list of optional entries. Duplicate registrations preserve first-match behavior. Parent chains are stored root-to-leaf to preserve the original float32 multiplication order. Empty opacity paths are rejected because they do not correspond to a valid native widget. Native malformed pointer graphs/cycles are outside the supported input domain.

## Stronger integrated replay

`tools/gen_ui_state_vectors.py` creates explicit controller-tree, window-list and widget-chain inputs, then executes these native services unchanged:

- `0x141512eb0` controller query;
- `0x1412e35c0` registry membership;
- `0x1412fd160` window opacity predicate;
- `0x1412d9430` recursive parent opacity;
- `0x1417fb320` engine-state byte getter.

The complete menu handler `0x14150bfe0` also executes natively for commands 29 and 30. Non-mutating code hooks observe query entry points. Only four UI effect functions are replaced with loggers: `0x14150ce20`, `0x14150e900`, `0x14150d370` and `0x14150fe90`. Those loggers check controller identity, window kind and byte flag where defined. Their bodies do not execute.

All 1033 contexts match the Rust query result, exact opacity bits for each supplied non-null registration, and both ordered menu-command traces (2066 command executions). Coverage includes missing/null mappings, unregistered handles, null registry entries, duplicate registrations, zero and noncanonical state-byte values, signed-kind extremes, zero/negative opacity and parent chains up to eight nodes. All supplied opacity inputs are finite and within [-1,1], including very small values; this does not establish NaN payload propagation, arbitrary overflow behavior or GPU color/alpha fidelity.

```sh
python tools/gen_ui_state_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-state.txt
OPENXPLANE_UI_STATE_VECTORS=/private/tmp/openxplane-ui-state.txt cargo test --offline -p xp-app --test ui_state_local -- --ignored
```

The corpus stays local and the comparison is explicitly ignored in ordinary CI. The tree/list states are controlled synthetic inputs; original node creation, real window snapshots and callback mutation are not proved. The four window action bodies, the refresh operation, live viewer wiring, Esc/Alt handlers and paired visual/motion checks remain outstanding. No whole input/UI parity is established, and compatibility shares remain unchanged.
