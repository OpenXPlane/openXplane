# Original UI and graphics fidelity acceptance

The active goal requires X-Plane UI and graphics 1:1, including Alt/Esc menus. Fonts, dimensions, spacing, icons, transitions, animations, lighting and materials are part of the requirement. A redesigned prototype does not establish parity.

## Current authoritative state (2026-10-13)

The active workspace viewer exits on Escape (`crates/xp-app/src/viewer.rs`); its HUD uses a Roboto Mono bitmap atlas (`hud.rs`). The recovered default key map maps Escape to `sim/operation/show_menu` (`research/KEYMAP.md`). Original Alt behaviour has not been established by that key record.

The preserved `codex/verified-wing-legacy` source has an independent Alt/Esc menu prototype. Its own `research/UI.md` states that layout, dimensions, colors and system font were implementation choices, without original pixel comparisons. It is not a verified visual port and should not be copied into the active viewer as evidence of 1:1 fidelity.

Original font/menu resources are now present in the supplied reference directory. The local menu-resource loader decodes 25 PNG/DDS textures and retains 12 original sfnt fonts. No paired original screenshot corpus has been established. Wine/Wine64 were not found on PATH; this does not prove the absence of another installation or another machine. A usable reference UI runtime/capture source remains to be established. Existing openXplane gallery images are not reference screenshots.

## Required visual evidence

For each substantial UI/graphics change, capture the original and openXplane at the same resolution, DPI/UI scale, graphics settings, aircraft, location, camera, lighting, weather and simulation state. Preserve this metadata and reference build identity beside the captures. Mark original and port images clearly; identify every remaining difference. A pixel difference can support the comparison, but must not hide mismatched capture conditions.

Capture both Alt and Esc flows, hover/open/close transitions, keyboard/mouse selection and focus behavior in GIFs or short videos at matched timing/frame rate. Screenshots alone cannot prove animation or input behavior. Verify fonts and assets from original resources or original runtime evidence before treating a visual approximation as recovered behavior.

Next work: recover original handlers and layout/style parameters, connect the now-available original menu/font resources, obtain matched captures, port their state/layout/rendering and compare both stationary and moving results. UI/graphics verified coverage stays zero until that evidence exists. Physics/kernel verification continues independently and does not count as UI fidelity evidence.


## Confirmed original resource paths

Static ASCII strings in the supplied baseline EXE contain these paths:

- `Resources/fonts/Roboto-Regular.ttf`
- `Resources/fonts/Roboto-Light.ttf`
- `Resources/fonts/Roboto-Bold.ttf`
- `Resources/fonts/RobotoCondensed-Regular.ttf`
- `Resources/fonts/DejaVuSans.ttf`
- `Resources/fonts/DejaVuSansMono.ttf`
- `Resources/fonts/NotoSansCJK-SC-Regular.otf`
- `Resources/fonts/tahomabd.ttf`
- `Resources/bitmaps/interface11/ins_loading_background.png`

The menu-specific font/style and texture assignments still require handler/resource analysis. The original `Resources/fonts` and `Resources/bitmaps/interface11` directories are now available in the local `Xplane12/Resources` tree. They remain local reference assets. Run `cargo run -p xp-app -- ui-resources /path/to/Xplane12` to check the original menu files. The viewer does not yet use these textures or fonts. Matched reference screenshots and motion captures are also still required. An independently downloaded font with the same family name would not establish that its version or metrics match this reference build.

See [../research/UI_RESOURCES.md](../research/UI_RESOURCES.md) for the local codec check and native main-menu resource references.

The 2026-10-13 input correction recovers Shift/Alt/Ctrl masks and all 182 default chords. Esc remains show_menu; Ctrl+backtick is toggle_settings_window. Alt alone has no command-table binding, so menu-bar behavior is not established by the modifier recovery. See [../research/KEYMAP.md](../research/KEYMAP.md).

Native window membership and cumulative opacity are now recovered as explicit state services (`research/UI_WINDOW_STATE.md`). The stronger replay removes query stubs but retains four effect loggers; it is not a live window or pixel comparison.

The focused UI dispatcher/filter is now recovered (`research/UI_FOCUSED_INPUT.md`), including Esc acceptance bit 0x8 and child-first ordering. This does not establish the Esc callback action or Alt menu activation. Original/openXplane paired screenshots and motion captures remain required once the actual menu renderer is connected.

Selection/focus transition ordering now has 2024 local native comparisons (`research/UI_FOCUS_TRANSITION.md`), including callback mutations. Notification bodies, hit-testing and widget construction are still missing. The observed main-menu button's own key callback returns false; concrete menu actions must be recovered from its children and other command handlers. This adds no pixel/motion evidence.

The complete keyboard entry now has combined UI/filter/binding evidence across 2152 local native cases (`research/KEYBOARD_ENTRY_DISPATCH.md`). Query/interceptor effects and command effects remain explicit service boundaries; the viewer still uses its independent input path. This establishes when commands are gated under controlled outcomes, not actual Alt/Esc menus or visual fidelity.

The keyboard capture query now uses recovered classification/list/opacity behavior rather than a supplied boolean (`research/UI_CLASSIFICATION_QUERY.md`): 1054 query contexts and 2152 strengthened combined keyboard cases pass. Class-2 semantics, runtime registry construction and concrete menu actions remain unresolved. No original menu has been rendered or visually compared yet.

The native interceptor wrapper and registered adapter now have 1864 exact local comparisons and execute inside the strengthened 2152-case full keyboard replay (`research/KEY_INTERCEPT_WRAPPER.md`). Actual external export/context effects and runtime registration remain unverified. The live viewer is unchanged; this supplies no menu pixels or motion evidence.

The command block-mask query confirms exemptions for show_menu/main-menu/settings in 6418 local native cases (`research/COMMAND_BLOCK_MASK.md`). Exemptions do not bypass other input gates and do not prove opening behavior. Actual ID-3 actions, viewer UI and paired visual evidence remain unfinished.
