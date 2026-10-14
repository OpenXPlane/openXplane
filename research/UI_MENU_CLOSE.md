# Controller close: registered-window queries and actual kind-map clearing

Date: 2026-10-14. EXE SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Complete close helper and scalar effects

`0x14150ce20..0x14150cf7d` begins with the recovered signed-kind/window-registry/
parent-opacity query `0x141512eb0`. If false it still reaches final
`0x14150fee0(controller,kind)`. A true result requests controller update
`0x14150e420(kind,0)`, then obtains the then-current window through
`0x14150b770`. Token changes in the update service are observed.

The helper tests mode 4 through `0x1412fd100`, resets through
`0x1412f9480(window,0,-1)` if true, then **rereads** mode 6 through
`0x1412fd1f0` and resets again if true. Both mode helpers scan for the first
nonnull matching registry entry and compare raw signed word +0x1ac with the
constant. A reset changing mode 4 to 6 therefore produces two reset requests.
Absent registry entries follow unsafe native dereferences; the Rust port returns
an error rather than claiming a safe original predicate.

`0x1412fb9a0` executes the actual current-positive-opacity query `0x1412fd160`,
flips that bool, and requests `0x1412f97e0(manager,window,show,0,0.0)`. This is a
**toggle of current visibility**, not an unconditional hide. An earlier service
may replace the mapped token with another registered but currently invisible
window, resulting in show=1. Geometry/visibility effect bodies remain unported.

If kind occurs in controller's signed list +0x90..0x98, close requests
`0x1412c6070(window,1)` and then uses actual map accessor `0x140d8c5b0` to store
**0 in that kind's value**. The key remains present; this is not map erasure.
`ui_menu_close` performs the same value store. The original post-close query then
returns false for that null mapping even if the registered node and its positive
opacity remain. Actual node destruction is a separate service, not inferred
from this map store.

Afterward it requests controller update with the caller's raw flag byte. It
checks global signed int `0x145a11f48`, then conditionally queries
`0x14150cf80`. If that returns zero and then-current controller+0x2a8 byte is
nonzero, it requests `0x140c7f790(global 0x14577734c)` and `0x1414d7470`. The
global condition is not rechecked after environment service mutation. Final
`0x14150fee0` runs regardless of the first query or the conditional work.

## Native execution and port comparison

`tools/gen_ui_menu_close_vectors.py` executes the complete close helper,
original window query/membership/cumulative opacity, registered-only window
getter, mode-4/mode-6 queries, visibility-toggle wrapper/query, and map accessor/
store unchanged. Window update/reset/visibility/delete/environment/command/
refresh/final effect bodies are controlled endpoints. No lookup result, mode
result, close branch or map store is replaced.

The getter profile explicitly supplies an empty controller protected-kind list
(+0x60..0x68), null observer global `0x14577b930`, and a present nonnull mapped
registered token whenever the getter is reached. The actual getter creates a
window for a missing/null map value; that factory branch is not tested by this
profile and is not replaced by a guessed null return. `ui_menu_close` rejects
that unsupported post-update state. Initial null mapping is covered through the
false-query path, which never calls the getter.

574 contexts compare exact map token, two registry-mode words, final leaf opacity
bits, global/controller fields, original post-close query, and ordered endpoint
trace. They cover signed/extreme kinds, null mapping, positive/zero/negative and
nested finite opacity paths, raw mode/flag/global/environment values, optional
value clearing, mode mutations after reset, mapped-token replacement after
update, opacity mutation in the declared service, and current environment-byte
reads. Observed requests: 172 visibility toggles, 129 resets, 96 deletes, and
574 unconditional final updates. The ordinary regression checks mode rereading,
null-map effect without guessed renderer changes, environment suppression and a
second close issuing only final update.

Native comparison is opt-in/ignored in ordinary CI. The supplied EXE and numeric
corpus remain local; normal sequence test and xp-app test clippy pass.

```sh
python tools/gen_ui_menu_close_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-menu-close.txt
OPENXPLANE_UI_MENU_CLOSE_VECTORS=/private/tmp/openxplane-ui-menu-close.txt cargo test --offline -p xp-app --test ui_menu_close_local -- --include-ignored
```

## Remaining limits

Registered entries are unique/readable and paths finite; modes are aligned with
the explicit registry. Missing/unregistered post-update tokens, factory creation,
protected/observer callbacks, deletion lifetime, populated UI graph, allocator/
error/concurrency paths and NaN payloads are unverified. Service opacity changes
in the replay are declared effects, not recovered rendering behavior. Actual
hide/delete/update/environment/command/final bodies still need recovery.

Next join this close helper with concrete direct/fallback/own/focused menu routes
and recover those service effects and window creation/registry lifecycle. Real
show_menu ID-3 opening/Alt activation, layout/fonts/GPU rendering and matched
screenshots/GIF/video remain outstanding. The viewer still exits on Esc and
renders no original menus; whole-game X-Plane 12 1:1 remains unproven.
