# Paired EXE/DLL keyboard routing with mutable registered sniffers

Date: 2026-10-14. EXE SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`;
Windows XPLM DLL SHA256
`706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## What now runs together

`tools/gen_keyboard_registered_vectors.py` maps both pinned PE artifacts in one
CPU emulator. Each initial sniffer is created through actual SDK registration
and initialized-TLS context constructor/destructor calls. Synthetic callbacks
call the original Register/Unregister APIs on the same emulated stack, as in
`XPLM_SNIFFER_DISPATCH.md`. They may also request controlled changes to EXE mode
and block state. Callback code cache is invalidated when rewritten, and root
callback returns are independently observed at RET. Original EXE/DLL code bytes
are not modified.

Each case runs two complete original EXE keyboard entries (`0x140c627a0`), not
standalone SDK event calls. Original window capture collection/opacity, focused
wrapper/base/filter and CRT, interceptor wrapper/adapter, resolved DLL export/
key-hook, SDK context/activation/enable query, registration/removal/EndCall/drain,
and native binding-vector loop all execute unchanged where reached. Route
observations expose the actual SDK phases reached from the EXE; supplied SDK
phase labels are retained only as inherited corpus metadata.

Capture registry, UI focus/mask, before/after sniffer bindings, activation bytes,
callback effects, current mode/block and initial bindings are explicit synthetic
state. UI own callback, EXE context/timing and Begin/End bodies, and SDK
validation/lazy loading/clock/tracking/allocator/diagnostics remain controlled
services. Hotkey and focused SDK window lists are empty; their depth/pending are
checked restored to zero. OS module loading and concrete original menu/plugin
callback bodies are not exercised.

`keyboard_registered_local` composes `keyboard_dispatch`, original-state
`ui_state` and `ui_input`, `key_intercept`, and the token-based registered
`xplm_sniffer`/`xplm_context`/`xplm_deferred` ports. Registrations/list state persist
across the two events. SDK callback changes to mode/block are seen by later
EXE gates. EXE return registers are not compared: emitted zero placeholders are **not**
native return evidence. Instead the comparison checks actual state and ordered
requests across both modules.

## Verified scope

528 cases compare exact live/deferred heads, signed depth/raw pending, every
node's links/owner/deleted/callback/plugin/before/refcon, freed/tracking state and
raw elapsed bits, preserved TLS stack [0], suppression and all three active
binding fields, final mode/block, and full service/route trace. It covers UI
capture/focus/filter decisions, mode and block gates, callback consumption,
head registration and deferred deletion, repeated events, and callback/UI-own
state changes. Source character inputs here are bytes 0/27/255; normalization
of 255 to 0 is checked at the callback logger. Full signed character/key
boundaries remain covered by the earlier 2152 fixed-list paired comparison.

Observed paths: 850 SDK export calls, 823 callbacks, 938 focused-UI routes,
137 UI-own calls, 160 Begin requests, 69 End requests, 67 node frees. These are
controlled replay observations, not executed original menu actions. Four direct
additional cases preserve callback mode=1000, callback block=255, consumed head
registration and focused capture consumption. An ordinary CI regression checks
self-removal followed by callback mode=1000: list deletion completes, later SDK
entry is gated and command binding fields remain unchanged across two events.

The common SDK harness still passes its regenerated 524 mutable scenarios.
The earlier fixed-list generator still passes all 2152 cases after its loader
was extended with a mapping-only option (default initialization is preserved).
All xp-app test clippy passes. Native comparisons are opt-in/ignored in ordinary
CI; corpora and supplied original artifacts stay local.

```sh
python tools/gen_keyboard_registered_vectors.py /path/to/X-Plane.exe /path/to/XPLM_64.dll > /private/tmp/openxplane-keyboard-registered.txt
OPENXPLANE_KEYBOARD_REGISTERED_VECTORS=/private/tmp/openxplane-keyboard-registered.txt cargo test --offline -p xp-app --test keyboard_registered_local -- --include-ignored
```

## Remaining acceptance

This extends paired CPU routing evidence to actual registrations and callback
list mutation. It does not prove EXE/DLL release-build pairing, Windows loader/
DllMain/TLS initialization, caller/thread validation or module load/unload,
tracking/allocator bodies, recursive/concurrent callback lifetime, concrete
widget/command effects, full SDK hotkey/window lifecycle or running viewer
integration. Begin/End action bodies still log requests; no show_menu behavior
is inferred from those IDs. Menus Alt/Esc still do not render and paired visual/
GIF/video evidence is absent. Whole-game X-Plane 12 1:1 remains unproven.

Next recover SDK window/hotkey lifecycle and trace actual show_menu/main-menu
window registrations and concrete focused widget callbacks, then connect the
original UI graph/layout/fonts/renderer and paired screenshots/motion checks.
