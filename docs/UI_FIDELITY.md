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

The complete command Begin/End executor now has 1768 original-code comparisons (`research/COMMAND_BEGIN_END.md`). Dispatches include menu IDs, but their callbacks still supply controlled effects. Live ID-3 registration and window actions remain unidentified; there is no Alt/Esc renderer or paired visual evidence yet.

The command lifecycle now includes native Continue, verified in 2152 combined cases (`research/COMMAND_CONTINUE.md`). Real held scheduling and ID-3 window actions are still missing; this adds no screenshot/motion evidence and does not change the viewer's Esc exit behavior.

The current external-hook investigation now requires the original Windows `Resources/plugins/XPLM_64.dll`, absent from the supplied tree (`research/UI_REGISTRATION_NEXT_INPUT.md`). Existing keyboard comparisons replace its exported callback; they do not verify that module or the real menu action. Candidate manager tables containing on-disk zeros are not runtime callback evidence. The requested paired screenshot/motion comparisons remain outstanding.

The requested SDK module is now available and its actual Windows keyboard hook has 1090 local native comparisons (`research/XPLM_KEY_HOOK.md`). Phase routing and iteration state are recovered, but plugin services and real window action effects remain boundaries. Separate DLL replay does not yet prove EXE↔DLL integration, and no visual fidelity evidence has been added.

The keyboard comparison now crosses the actual EXE→Windows DLL export boundary in 2152 synthetic CPU cases (`research/KEYBOARD_PAIRED_REPLAY.md`). It replaces the export-result logger with original DLL routing, but controls registrations/context/callback effects. It does not initialize or render the original game; live Alt/Esc and paired pixels/motion remain outstanding.

SDK context ctor/activation/dtor now execute inside both hook comparisons (`research/XPLM_CONTEXT.md`), with 1108 direct contexts and stronger 1090/2152 hook/paired cases. Plugin 0 bypasses enabled checks; other plugins use actual record bytes with controlled lazy loading. Ready TLS and callback effects are explicit inputs, not live game initialization or menu pixels.


The concrete kind-11 window creation path now identifies menu panel vtable
0x1426064f0 and its own Esc/Enter callback (`research/UI_MAIN_MENU_KEY.md`).
1940 contexts compare direct and focused/filter routes with the real getters,
constant query and no-op slots unchanged. The nontrivial fallback action is
still controlled, and actual creation/flags/focus/child graph, Alt activation
and layout/rendering are unverified. This does not add paired visual evidence.


The concrete Esc fallback request flow now has 746 contexts across direct, own
callback and focused/filter routes (`research/UI_MENU_ACTION.md`). Actual
property-2 getter and empty-wrapper lifecycle execute unchanged. Query/window
construction/direct/resolution/submission effects are still service boundaries;
no opening/closing or visual outcome is proven by those ordered requests.


The direct menu action now has 694 full native control-flow comparisons
(`research/UI_MENU_DIRECT.md`), including c9 re-entry protection and the kind-11
close request. Close/window/document/dialog bodies remain explicit services;
this does not prove hiding a real window or menu rendering.


The registered-window close helper now has 574 original-code contexts
(`research/UI_MENU_CLOSE.md`), including actual kind-map value clearing and
post-close presence queries. Window hide/delete/update and renderer effects
remain service boundaries; no paired screenshot or motion evidence is added.

Native window extraction and classification-dependent reinsertion now match 700
combined CPU cases ([window order evidence](../research/UI_WINDOW_ORDER.md)).
The recovered lookup removes a list node; it must not be treated as a pure query.
This does not establish full show/hide effects or viewer rendering. No new paired
screenshots/GIF/video were produced by this CPU comparison.

Show/hide now has 1456 original-code calls verifying registry/membership and leaf
alpha mutations ([visibility evidence](../research/UI_WINDOW_VISIBILITY.md)).
Ancestor alpha is preserved, including repeated membership in the controlled
zero-ancestor profile. Parent/dirty propagation and other effect bodies are still
services; live menu rendering and matched screenshots/motion remain unverified.

Widget reparent and dirty propagation now have 744+744 original-code comparisons
([widget-tree evidence](../research/UI_WIDGET_TREE.md)), including callbacks that
change parent/flags. Shared window/widget ancestry is the next integration gate;
independent opacity-path and tree comparisons do not prove the combined menu
lifecycle. No paired visual captures were produced by these CPU comparisons.

Show/hide and actual widget reparent now share ancestry: 2184 original visibility
calls include native queries after each action ([combined graph evidence](../research/UI_WINDOW_GRAPH.md)).
An old zero-alpha ancestor no longer determines visibility after reparenting to
the implementation node; that node's alpha is now used. Actual close/menu route
and viewer integration are still pending. This CPU evidence produces no paired
screenshots or motion capture and does not raise UI/graphics verified coverage.

Controller close now toggles the shared graph: 1456 original close calls reach
367 original visibility bodies and match post-close presence queries
([close-graph evidence](../research/UI_MENU_CLOSE_GRAPH.md)). Delete/reset/update
bodies remain controlled and live menu/viewer integration is pending. No
matched screenshots or motion captures were produced by this CPU comparison.

The direct/close bridge now has combined original-code evidence for argument 0
and UI mode 0/2: 1456 direct actions reach 1092 actual close helpers and 252
visibility bodies ([limits](../research/UI_MENU_DIRECT_GRAPH.md)). This does not
prove mode-1 dialogs, actual Esc entry, construction or paired visual fidelity.

The combined direct/close replay now exercises document/dialog request flow in
mode1, including success/failure responses and cleanup. Its actual load/dialog
effects remain controlled ([current scope](../research/UI_MENU_DIRECT_GRAPH.md));
this does not prove a working dialog or original GPU pixels.
