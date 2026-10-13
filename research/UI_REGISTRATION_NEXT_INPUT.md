# UI registration investigation and next required local reference

Date: 2026-10-14. Reference executable SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Authoritative missing module

The reference executable contains the exact module path `Resources\plugins\XPLM_64.dll` and export string `XPLMKeyPressHook`. A filename search over the supplied local `Xplane12` tree finds no XPLM/XPWidgets module. `Xplane12/Resources/plugins` itself is absent. The next external-hook investigation therefore requires the Windows x64 `Resources/plugins/XPLM_64.dll` from the same installation/build as the executable. No other missing file is requested at this stage.

The previously recovered adapter `0x141917f20` reads export pointer `0x146106038`; loader call `0x141915684` resolves the `XPLMKeyPressHook` string and stores its result there at `0x141915689`. The outer keyboard interceptor can invoke that adapter before/after focused UI dispatch. Existing replays execute the wrapper and adapter but replace the export body with a controlled endpoint. They do not establish actual export behavior, module initialization, context services or plugin handling.

Need the module to inspect its exports/code and recover the real endpoint and registration/context behavior. Finding this path does not prove that the DLL implements the game's show_menu action; its role in keyboard interception is the established connection. The actual ID-3 window callback remains unresolved.

## Registration observations and exclusions

The direct owner-observer global address `0x1457774f8` appears at `0x140c7f380` in held-command processing. This is a read/snapshot use, not registration. Observer removal `0x140c6c9e0` has a direct call at `0x141a8d441`, with an object subcomponent at caller object+8. That establishes a removal site, not a menu handler identity.

UI-manager global `0x145a0e308` has observed stores at `0x14056f4f4` and `0x14056f508`. The first site's preceding construction assigns `0x142675158` to the manager candidate and `0x14267ade8` to its implementation candidate. These two candidate vtable regions contain zero words in the supplied PE's on-disk .rdata bytes, including offsets +0x38/+0x40/+0x50. Their actual runtime initialization/use is not established. Do not invent concrete Alt/Esc callbacks from these candidate addresses, or treat raw on-disk zeros as observed runtime behavior. The recovered actual main-menu button tables and controlled input replays remain separate evidence.

Startup `0x141c26694..0x141c2669c` still establishes the menu/settings controller's registration, but not show_menu ID 3. The third command phase is now ported and tested; this registration investigation adds no live menu or pixel evidence.

## User-directed pause and resumption

The user explicitly requested stopping when a genuinely necessary reference file is needed and resuming after they attach it. Preserve the full whole-game/UI 1:1 objective. Pause at this missing external-module boundary after publishing these observations. Resume by checking the supplied DLL's identity, architecture/build compatibility and exports; then analyze the actual keyboard hook/context endpoints. Do not count a module attachment or green kernel tests as proof of original menu behavior.

Alt activation, actual Esc window actions, live viewer integration, rendering and paired original/openXplane screenshots/GIF/video remain required. Progress rubric shares are unchanged.
