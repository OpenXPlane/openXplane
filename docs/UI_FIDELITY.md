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

Combined fallback/direct/close now has 1456 original fallback calls across all
fallback routes, including independent queries and engine mutation
([limits](../research/UI_MENU_ACTION_GRAPH.md)). Own/focused Esc entry and original
viewer/GPU construction remain pending; this CPU replay is not visual evidence.

Concrete own/focused Esc/Enter handling now reaches graph-backed close in 1456
original-code entries ([key-graph scope](../research/UI_MENU_KEY_GRAPH.md)). This
is a synthetic already-created panel with pinned focus; it does not establish
show_menu opening, Alt activation or viewer rendering. Paired visuals remain
pending and UI/graphics verified coverage stays zero.

Owner/selected binding now matches 1045 whole native helper cases
([scope](../research/UI_FOCUS_BINDING.md)). Binding and widget parent links are
distinct; binding callbacks can change the chain being cleared. This is not yet
joined to the key graph and does not prove live focus or paired UI pixels.

Focus/menu composition now reads current focus after graph-close and executes
recovered owner/selected binding. Its ordinary tests use controlled blur/child
effects; combined original replay is still pending
([limits](../research/UI_MENU_FOCUS_GRAPH.md)). This does not establish live viewer
focus or paired rendering.

Mutable focus is now part of combined original key/close/binding replay: 1456 key
entries and 144 original binding calls match owner/selected/global-focus state
and subsequent routing ([current scope](../research/UI_MENU_FOCUS_GRAPH.md)).
Virtual callbacks and real construction remain controlled/unverified; this is
not a working original viewer menu or paired visual evidence.

Input-node base construction now matches 64 original byte/snapshot comparisons
([scope](../research/UI_INPUT_CONSTRUCT.md)). This establishes initial input/
container state for inline names, not full panel geometry, live creation or GPU
pixels. Original menus and matched captures remain pending.

Common panel construction now has 256 original nested ctor comparisons and
provides initial Esc/Enter flag fields ([scope](../research/UI_PANEL_CONSTRUCT.md)).
Concrete main-menu geometry/resources, heap/localized strings and live creation
remain pending; these byte images are not original GPU pixels.

Embedded main-menu member construction now matches 180 original complete ctor/
registration/growth cases ([scope](../research/UI_MENU_MEMBER_CONSTRUCT.md)).
This establishes member state and parent registration, not full panel creation,
layout/resources or pixels. Matched captures remain pending.

Embedded-interface global linking/unlinking now has 210 whole original ctor/
destructor comparisons ([limits](../research/UI_INTERFACE_CHAIN.md)). This
is one construction/lifecycle dependency, not complete panel teardown or GPU
pixels. Original rendering and matched captures remain pending.

Concrete Main Menu initialization now composes these constructors in480 exact
original-prefix comparisons ([scope](../research/UI_MAIN_MENU_CONSTRUCT.md)).
The asserted stop0x140ecb592 precedes singleton dialogs; it is not a normal
constructor return. Full object/allocation bytes, global links and three parent
registrations match with controlled translation/memory endpoints. Actual shared
dialogs, children/layout/resources, opening/Alt, viewer rendering and paired
screenshots/GIF/video remain required. No visible menu change is claimed.

Bound dialog callback lifetime now has324 original sequences/5832 intermediate
state comparisons ([scope](../research/UI_CALLBACK_LINKS.md)). Temporary copies
preserve reciprocal registration and source removal detaches survivors. This
checks ordered logical links, full-width parameters and allocation/free counts,
not native tree bytes or a complete dialog. Heap-string/common-panel support,
real dialog children/resources/layout and singleton composition are next.
Original rendering and paired screenshots/GIF/video remain missing.

Common-panel heap-string ownership now matches324 complete original ctor cases,
including aligned storage and full allocation/free snapshots
([scope](../research/UI_PANEL_HEAP_STRINGS.md)). This enables the dialog's long
label profile through the recovered common/input constructors. Actual dialog
translation/formatting, children/layout/bindings and singleton composition are
still unfinished. Original viewer menus and paired captures remain missing.

The dialog's40-byte child bundle now has576 whole outer-factory comparisons
([evidence](../research/UI_DIALOG_CHILDREN.md)). Its five-pointer creation flow,
conditional constructor calls and inline fifth-object containers match with
explicit controlled nested constructors/resource/refresh. This is not full
widget construction or rendered UI. Recover those nested bodies, complete
dialog/layout/binding composition and retain the original paired-visual gate.

Parent callback-list receiver141149460 now has72 original append/lifetime
sequences/936 compared states ([scope](../research/UI_CALLBACK_LIST.md)). It
retains registered copies rather than references to temporary wrappers. List
link/count fields and logical callback state match; internal callback-tree bytes
remain unmodeled. Actual child refresh/construction, full dialog/layout/bindings,
opening/Alt and paired original GPU captures remain required.

Third-child1410d7490 construction now has216 complete original comparisons and
an ordinary outer-factory adapter test ([scope](../research/UI_DIALOG_CHILD_CONTEXT.md)).
Controlled refresh can redirect the parent; its list retains a registered0/0
copy after temporary cleanup. Final child/base bytes and logical list/callback
state match. Actual refresh, remaining children/full dialog, callback-tree heap
bytes and combined native factory/visual evidence remain unfinished.

The third-child refresh caller1411396d0 now has972 whole original comparisons
([scope](../research/UI_CONTEXT_REFRESH.md)) and ordinary child-constructor
integration. Context selection, Zulu/day override arithmetic, option cleanup and
mutable listener traversal match with explicit controlled providers/CRT parsing/
time conversion/notifications. Actual provider bodies and combined original
constructor/factory evidence remain pending. This is not rendered-menu parity.

Original child+refresh+retained callback lifetime now compose in972 whole native
constructor executions ([scope](../research/UI_CHILD_REFRESH_COMBINED.md)). The
refresh body is no longer replaced in this replay. Leaf providers remain
controlled, callback-tree heap bytes opaque and observers initially empty.
This closes a CPU composition gap; full factory/dialog and paired GPU evidence
remain missing, and no visible menu change is claimed.

Second-child1410d6fc0 now has920 whole original comparisons and an ordinary
receiver integration with the third child ([scope](../research/UI_DIALOG_CHILD_SELECTION.md)).
Global interface/data/catalog/variant/callback construction is recovered with
explicit controlled property/config/effect bodies. Raw constructor snapshots and
logical live receiver lists remain distinct; no executable ABI/live viewer claim
is made. First/fourth children, full factory/dialog and paired GPU evidence remain
unfinished.


## First child and receiver chain (2026-10-17)

[First child evidence](../research/UI_DIALOG_CHILD_ROOT.md):192 whole native
constructor cases match object/all seven heap blocks and service-entry snapshots.
Allocator and both refresh bodies remain controlled. Shared data initialization
is reused by the second child, whose920-case replay is rerun after extraction.
Ordinary integration now constructs root, selection and context receivers in
sequence, with controlled effects. Native full-factory composition, fourth child,
both root refresh bodies and remaining providers are next, followed by dialog/
layout/bindings/menu/teardown. Original Alt/Esc, GPU and paired captures remain
unverified. Compatibility estimates were reviewed and remain unchanged.


## Fourth child and all four receiver adapters (2026-10-17)

[Fourth child evidence](../research/UI_DIALOG_CHILD_WEATHER.md):720 whole native
runs match raw weather initialization/copying, provider-entry snapshots, saved
first/reloaded second parent, retained callbacks and temporary lifetime. Weather,
altitude/selection, refresh/special and allocation/free remain controlled. All
four child adapters are now connected in ordinary source integration. Native
combined factory, remaining providers/root refreshes, dialog/layout/bindings/
menu/teardown and original Alt/Esc/GPU paired captures remain pending. Estimates
reviewed and unchanged; this is not visual parity evidence.


## Nested root refresh and native constructor composition (2026-10-17)

[Nested refresh evidence](../research/UI_NESTED_REFRESH.md):432 whole native
refresh14113a300 runs match mutable vector traversal, gates, temporary cleanup
order and live observer links.192 combined native root constructor runs include
the actual nested refresh under controlled engine0/fresh lists; full object and
all seven allocations match. Leaf provider/effect bodies and final root
refresh141138cf0 remain controlled. Earlier192 controlled-refresh root cases
still pass. Next recover final root refresh/providers and compare full native
factory/dialog, then layout/bindings/singleton/menu/teardown and original
Alt/Esc/GPU paired captures. Estimates remain unchanged after evidence review.


## Payload ownership dependency (2026-10-17)

[Payload evidence](../research/UI_PAYLOAD_MOVE.md):720 whole native cases verify
payload assignment, optional string move and vector move/growth, including heap/
aligned releases and self-alias.432 nested-refresh compositions now execute real
assignment in27 reached branches with inline buffers. Full root refresh141138cf0
remains unported; its record/config parsing and other provider bodies still need
recovery. This removes an assignment stub in the nested-refresh adapter, without
claiming full factory/dialog or original Alt/Esc/GPU behavior. Estimates reviewed
and unchanged; paired visual captures remain required.


## Final root refresh outer body (2026-10-17)

[Root refresh evidence](../research/UI_ROOT_REFRESH.md):118 whole native runs
match resource/configuration selection, fallback strings, vector caller ownership,
count/refill, one-time retained callbacks and live primary listener traversal.
The apply-record flag is verified as R8B; upper register bits are excluded from
that byte argument. Root/declared memory and live provider operands are checked
through the refresh_image bridge. Leaf provider/parser/effect bodies and nested
refresh are controlled here. Next compose real nested refresh/payload movement,
then native full constructor/factory and remaining providers/dialog/layout/menu.
Original Alt/Esc/GPU and paired captures remain unverified; estimates unchanged.


## Shared root/nested/payload refresh execution (2026-10-17)

[Composed refresh evidence](../research/UI_REFRESH_CHAIN.md):118 whole native
runs execute actual root, nested and payload/string/vector assignment bodies.
Shared root memory, global/index, live links and logical callbacks match.17 runs
reach payload assignment under inline storage;322 root+nested notifications are
checked. Earlier118 independent corpus rows remain byte-for-byte unchanged.
Leaf calculations remain controlled. Next compare entire root construction with
both real refreshes, then full factory/dialog/layout/menu/teardown and original
Alt/Esc/GPU paired captures. Estimates reviewed and unchanged.


## Whole root construction with actual refreshes (2026-10-17)

[Constructor composition evidence](../research/UI_ROOT_CONSTRUCT_CHAIN.md):118
whole original constructors now execute both nested refreshes, final root refresh
and payload helpers in the same call. Shared state, seven allocation boundaries,
receiver links and logical callback peers match;34 inline assignments are reached.
Fresh lists apply; callback RB backing and remaining leaf calculations are still
unverified. Next compare full factory/dialog and recover providers/layout/menu/
teardown. Live Alt/Esc/GPU and paired captures remain pending; estimates unchanged.


## All child constructors within the original factory (2026-10-17)

[Factory composition evidence](../research/UI_FACTORY_COMPOSED.md):216 whole
native factory calls execute all four constructor bodies and inline fifth/hash
initialization. Raw declared heap/bundle snapshots, shared receiver links/counts
and callback peers match;972 retained entries survive temporary cleanup.
Refresh/provider effects remain controlled in these cases. Actual root/context
refresh evidence is still separate. Next combine those bodies with the factory,
recover fifth refresh/remaining providers and complete dialog/layout/menu/teardown.
Original live Alt/Esc/GPU and paired captures remain unverified; estimates unchanged.


## Whole factory with actual root refreshes (2026-10-18)

[Factory/root-chain evidence](../research/UI_FACTORY_ROOT_REFRESH_CHAIN.md):118
whole original factory calls execute all four constructors and actual root/nested/
payload bodies. Shared memory/global/index state, declared heap and649 retained
entries match. This exposed a resource-index bug: selection must reload global
142640504 after engine gating, receiving88/77 rather than0.34 inline assignments
are reached. Context/other child/fifth refresh and remaining leaf providers stay
controlled. Next include actual context refresh, recover remaining refreshes/
providers and complete dialog/layout/menu/teardown. Live Alt/Esc/GPU and paired
captures remain unverified; estimates unchanged.


## Shared factory with actual context refresh (2026-10-18)

[Root/context composition evidence](../research/UI_FACTORY_ROOT_CONTEXT_REFRESH.md):
118 whole original factory calls now execute context1411396d0 in addition to all
four constructors and root/nested/payload bodies. Shared index reloads88/77,
Zulu/day math, inline/heap/aligned string cleanup and649 retained entries match.
34 alternate/84 default context branches,82 parses and52 frees are observed.
Context leaf computations/CRT and selection/weather/fifth refresh/providers remain
controlled. Next recover those dependencies and full dialog/layout/menu/teardown;
original live Alt/Esc/GPU and paired captures remain unverified. Shares unchanged.


## Fifth refresh joins the shared factory (2026-10-18)

[Fifth-refresh evidence](../research/UI_FIFTH_REFRESH.md):528 native calls now
match outer141139460 and hash-clear dispatcher/bulk behavior, including cached
next pointers, sentinel reloads, signed iterator indexing, live vector reads,
override bits and714 notifications. Sparse range erase and remaining dependent
providers stay controlled.118 whole native factory runs now include actual root/
nested/payload/context/fifth bodies and649 retained entries; fresh fifth lists
apply there. Next recover selection/weather refreshes/providers and full dialog/
layout/menu/teardown. Live Alt/Esc/GPU and paired captures remain unverified;
shares reviewed and unchanged.


## Fifth payload predicate uses shared root settings (2026-10-18)

[Payload-gate evidence](../research/UI_FIFTH_PAYLOAD_GATE.md):528 complete fifth
refreshes and118 whole factory calls now execute actual14113b9a0, including repeated
engine/global gating, fresh resource-index loads and full32-bit mode checks.
Root/fifth getter identities and inherited settings bytes are checked;17 fifth
payload branches occur in the shared composition. Old opaque-predicate records
are unchanged and replay under an explicit controlled seam. Engine/resource/
settings lookup bodies, other dependencies, full dialog/menu/live Alt/Esc/GPU and
paired captures remain unverified. Shares reviewed and unchanged.


## Real indexed records in the fifth element (2026-10-18)

[Indexed-record evidence](../research/UI_INDEXED_RECORDS.md):872 complete original
accessors match signed-low32/wrapping64 address calculations.528 fifth-refresh
runs now execute original getters over distinct real-stride cells and live base
changes.118 whole factory runs also execute the constructor's real index20 lookup;
90/34 fifth resource/settings returns and118 constructor returns are checked.
Root descriptors/settings word and649 retained entries stay consistent. Other
callers' getter bodies and registry/record ownership remain controlled, followed
by remaining refresh/provider/dialog/layout/menu work. Live Alt/Esc/GPU and paired
captures remain unverified; shares reviewed and unchanged.


Read-only query1417fb320 is now recovered and used by default in the fifth refresh
and availability predicate.768 complete original calls cover every byte with
three poisoned receivers;528 fifth and118 whole-factory runs execute this actual
query alongside real indexed lookups (1056/236 actual query returns). Controlled
sort effects exercise15 intervening byte changes. Root/other child queries and
registry production/lifetime remain controlled or unproven, as do full dialog,
viewer and paired GPU evidence. See [UI_ENGINE_QUERY](../research/UI_ENGINE_QUERY.md).
Compatibility estimates were reviewed and remain unchanged.


Root141138cf0 and both nested14113a300 calls now use production indexed lookups.
118 complete original factory runs execute226 root/nested resource and46 settings
returns against distinct valid32-byte source strings. All20 array input regions
are inherited and compared before fifth refresh;649 retained entries match.
Root index flow changes the fifth paths (23 sorts/11 payload assignments), as
confirmed against original code. Selection/context lookups and other providers
remain controlled; registry production/full dialog/viewer/GPU/paired captures are
unverified. See [UI_ROOT_RECORD_TABLES](../research/UI_ROOT_RECORD_TABLES.md).
Estimates reviewed and unchanged.


Selection constructor1410d6fc0 and context refresh1411396d0 now query actual
indexed resource cells in118 original whole-factory runs.45 real getter returns
per caller (23 at88/22 at77),45 selection getter-entry snapshots and118 context
input maps match source mapped lookups on the20 inherited regions. Both APIs
propagate missing-map errors and offer mapped provider defaults; legacy endpoint
replays remain explicit. Context follows23 alternate/95 default branches.
Remaining selection/weather/query/registry/provider bodies, full dialog/menu and
live GPU/paired visuals remain unverified. See
[UI_CHILD_RECORD_TABLES](../research/UI_CHILD_RECORD_TABLES.md). Estimates unchanged.


Whole weather refresh141139fe0 now matches384 complete original calls, including
live settings/index reloads,424-byte forward/overlapping copies, flags and575
listener notifications.118 complete factories also execute this actual body,
with20 inherited table inputs,79 reset/11 resource/28 default paths and649 retained
entries. Computed weather/context/CLI/notification providers remain controlled;
real data producers/lifetime, remaining selection/full dialog/menu/viewer/GPU and
paired visual evidence remain unverified. See
[UI_WEATHER_REFRESH](../research/UI_WEATHER_REFRESH.md). Estimates unchanged.


Selection changed14113ba10 now matches348 original calls (547 notification
requests), including callback-preserved dirty bytes and nodes appended during
traversal.118 complete factories execute the actual body in95 fresh receivers.
Removing the previous opaque second-parent effect produces601 retained entries,
matching original code under these profiles; older649-entry controlled cohorts
retain their scope. The checked bridge validates/publishes the source sentinel
and image. See [UI_SELECTION_CHANGED](../research/UI_SELECTION_CHANGED.md).
Actual callbacks/remaining selection/providers/full menu/viewer/GPU and paired
visual evidence remain unverified. Estimates unchanged.


Selection alternate14111bb90 now matches528 complete original calls across both
major paths/all valid tags, with1324 actual settings returns, raw coordinate and
stack operands, live vector/result reloads, aligned frees and the real temporary
destructor.118 factories execute23 actual alternate and95 actual changed bodies;
590 retained entries match after removing the remaining opaque second-parent
effect. Real providers/registry/reset/final/full dialog/viewer/GPU and paired
visuals remain unverified. See
[UI_SELECTION_ALTERNATE](../research/UI_SELECTION_ALTERNATE.md). Estimates unchanged.


Whole selection finish14113d660 now matches560 complete original caller runs
with actual gate/query/primary/resource getters and explicit controlled leaf
boundaries. Production scalar values separately match1452 SSE instruction
results under Rosetta and the ordinary native x86_64 Linux and Windows CI tests. Raw whole-
caller replay uses an explicit Unicorn2.1.4 NaN override in test code only.
See [UI_SELECTION_FINISH](../research/UI_SELECTION_FINISH.md). Finish still
needs composition into recursive reset/callback/factory callers. Full dialog,
Alt/Esc live integration and paired original screenshots/GIF/video are unfinished;
this CPU evidence leaves rendering verified coverage at zero.
