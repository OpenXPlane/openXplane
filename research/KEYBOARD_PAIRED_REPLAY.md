# Paired original EXE and Windows XPLM keyboard replay

Date: 2026-10-14.
EXE SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
DLL SHA256 `706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f`.

## Actual module boundary

`tools/gen_keyboard_dispatch_vectors.py` now maps both original PE section images into one Unicorn CPU address space at their preferred bases (EXE 0x140000000, DLL 0x180000000). It pins both digests and validates the DLL's AMD64/PE32+ identity. The EXE adapter's export pointer `0x146106038` is set to actual `XPLMKeyPressHook` at `0x18001a670`, replacing the former export logger.

The complete EXE keyboard entry, class collection/visibility query, interceptor wrapper/adapter, focused wrapper/filter/initial CRT and binding loop execute unchanged. Calls across the EXE adapter reach the original DLL export/body and its consumed-sniffer iteration-end helper. Nonmutating hooks record DLL entry phase and full EXE/UI/interceptor arguments. Original DLL routing/return/counter stores are not replaced.

The explicit DLL state has one before sniffer and one after sniffer, plugin ID 0, active context, no hotkeys or plugin-focused window, and no pending list mutations. The two sniffer refcons 0/2 select supplied before/after results. Their callbacks can change EXE mode/block globals, testing that both the continuing EXE entry and later interceptor gates see those changes. Negative sniffer returns pass onward as the DLL's normalized return 1; raw zero consumes.

## Evidence and exact limits

2152 cases compare ordered query/interceptor/DLL/sniffer/UI/Begin/End trace arguments, owner suppression and binding active fields, and final mode/block state. Both implementations also assert sniffer/hotkey depth and pending fields close to zero under these particular registrations. Rust's combined replay now calls both recovered `key_intercept` and `xplm_key_hook` implementations instead of a direct export-result provider.

The native run retains controlled EXE clock/context/allocator services, DLL validation/context activation/release, sniffer callback bodies, the concrete UI own-key callback and Begin/End effects. DLL context is always active in this combined corpus; broader context/hotkey/plugin-window/flush branches remain covered separately by the 1090-case DLL comparison (`XPLM_KEY_HOOK.md`). Timing/scope branches remain covered separately by the interceptor corpus. Those separate corpora do not prove their services' concrete behavior.

This is a synthetic CPU replay of both binaries, **not** Windows module loading or an original game process. It does not execute DllMain, resolve imports, initialize TLS/registration, load user plugins, establish EXE/DLL release-build pairing, or validate real GUI graphs. The preferred-base mapping is sufficient for this pinned, controlled call path only; it is not a general plugin loader. Actual show_menu/window effects, held scheduling, renderer integration and paired visual/motion evidence remain unfinished.

```sh
python tools/gen_keyboard_dispatch_vectors.py /path/to/X-Plane.exe /path/to/XPLM_64.dll > /private/tmp/openxplane-keyboard-paired.txt
OPENXPLANE_KEYBOARD_DISPATCH_VECTORS=/private/tmp/openxplane-keyboard-paired.txt cargo test --offline -p xp-app --test keyboard_dispatch_local -- --ignored
```

The new test requires both digest headers, rejecting the older EXE-only corpus. Corpora remain local and the comparison is ignored in ordinary CI. The original DLL and executable are not copied into the repository.

Next: recover DLL context/activation and real callback/list registration, then trace actual menu handlers and connect input/lifecycle to the viewer. Whole-game X-Plane 12 parity and Alt/Esc pixel/motion fidelity remain unproven.
