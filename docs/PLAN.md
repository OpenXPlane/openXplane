# Plan for a full implementation

The active completion criteria and mandatory commit/push workflow are in
[GOAL.md](GOAL.md). Each completed coherent change must be checked, committed
and pushed; partial ports do not complete the full-game goal.

## Real reference window baseline (2026-10-26)

Computer Use recovered after kernel reset. New original Cirrus cockpit and
top-bar captures exist, with native replay/settings caveats and JPEG/resolution
metadata in [the baseline](../research/REFERENCE_WINDOW_BASELINE.md). A native
QA plan generator preserves hashes and avoids the reference's drive-qualified
path failure. Native export and a matched Cessna/KSEA comparison remain pending.
Next recover object flag/attachment selection for content-driven rendering and
obtain same-scene still/motion/menu evidence. No visual parity share is added.

## Selection dependency composition (2026-10-25)

[Dependency evidence](../research/UI_SELECTION_DEPENDENCIES.md):1988 original
calls verify mode1414cedd0, header1411222e0, singleton1414cc7f0 and their actual
secondary/gate callers. Full results, provider order, declared memory, local
operands and live cleanup reloads match. Reset/finish/callback defaults now use
the recovered dependency bridge; five ordinary tests cover integration and
scratch lifetime. Settings/accessor/header-string/catalog bodies and real
ownership remain controlled. Next remove these producer boundaries and compose
the new actual dependencies through full original parent/factory calls, then
dialog/layout/menu/Alt/Esc/viewer/GPU and paired captures. Estimates reviewed
and unchanged; there is no visible menu change.
REA verification completed2026-10-26: three bounded GNU objdump Evidence
records cover all752 original bytes without truncation through a disclosed
range adapter. Ghidra decompilation remains unavailable; its redundant full-PE
run was stopped after the bounded static review completed.

The goal is an engine that loads original X-Plane 12 content and behaves like the original. This plan orders
the work by dependency. Each phase has an exit check against the original (emulator vectors, content loads, or a
recorded trajectory), not only unit tests. Shares refer to the weights in [COMPATIBILITY.md](COMPATIBILITY.md).

## Phase 1: Flight model (weight 28)

1. Wing element: finish `0x1411b6630` (delta-wing block, helpers `0x14082b800`, `0x14121bfd0`, `0x1411e1940`).
2. Force aggregation: `get_el_force` (`0x1411b9840`) with the control-surface helper `0x141221220`, supersonic blend, flaps, slats, spoilers.
3. Fuselage, stabilizers, rudder and ground effect forces; moments about the centre of gravity.
4. Mass, centre of gravity, inertia from the ACF; fuel and payload.
5. Piston engine and propeller (blade element, governor, mixture, carburetor ice), then turboprop and jet.
6. Landing gear: springs, tyre friction, brakes, steering, retraction.
7. The flight loop: integration step, atmosphere and wind, then replace the approximate model piece by piece.

Exit: recorded trajectories of the stock Cessna 172 (takeoff, cruise, stall, landing) match within a stated tolerance.

## Phase 2: Datarefs and commands (weight 8)

1. Backing values for the 5503 datarefs, grouped by owner (flight, engine, controls, view, weather).
2. Command handlers for the 3138 commands, wired to the simulation state.
3. Input: keyboard (done), mouse, joystick and yoke axes, button assignments.

Exit: every dataref the stock Cessna reads has a value; every default key binding does what the manual says.

## Phase 3: Content and rendering (weights 7 + 6 + 8)

1. OBJ8: normal and lit maps, all animation datarefs, attribute commands, instruments and panel regions.
2. Rendering: sky, sun and lighting, shadows, PBR-like materials, night lights, cockpit panel.
3. ACF: type the rest of the 1117 properties as each subsystem needs them.

Exit: the stock aircraft renders with its own textures and animations driven by datarefs.

## Phase 4: World (weight 14)

1. DSF tile reader: terrain, forests, roads, objects, polygons.
2. Global scenery and library resolution (`library.txt`, `Custom Scenery`, scenery packs).
3. apt.dat: taxi routes, start locations, lights, signs, windsocks.
4. Weather: wind, clouds, visibility, pressure, time of day.

Exit: the aircraft sits on a real runway in real terrain, with taxiways, lights and nearby objects.

## Phase 5: Cockpit and systems (weight 10)

1. Panel (PANEL_3D) and the cockpit instruments of the stock aircraft.
2. The stock Cessna's Lua scripts and system logic (electrical, fuel, avionics).
3. Autopilot, GPS and radios.

## Phase 6: Audio, multiplayer, plugins (weight 7)

1. FMOD-compatible sound banks, engine and ambient sound.
2. Networking and multiplayer.
3. The XPLM plugin interface.

## Phase 7: Platforms and distribution (weight 6)

1. A launcher and settings UI, tagged releases.
2. Packaging for macOS, Windows and Linux.

## Method for every ported function

1. Locate the function and its callers with the PE tools and the disassembler.
2. Run it in the emulator on random inputs and write the vectors.
3. Port it with the same float32/double order of operations.
4. Require identical bits (a few ulps only for libm functions) in `original_vectors.rs`.
5. Record the evidence and what is not established in `research/`.
6. Update `docs/compat.json` and the roadmap numbers.

## UI fidelity work in progress (2026-10-13)

Original fonts and interface11 resources are available locally. The new `ui-resources` command loads 12 fonts and decodes 25 menu textures; native main-menu texture references are recorded in `research/UI_RESOURCES.md`. Next: recover Alt/Esc handlers, layout and font-role selection, connect these assets to the renderer, then obtain matched stationary and motion comparisons. No visual or input parity has been established; the active viewer still exits on Escape.

## Native input continuation (2026-10-13)

The binding-vector loop now has exact local transition/trace evidence across 2728 cases. Its Begin/End callback bodies are logged, not executed. Recover command continuation, input interception and Show Menu callback registration, then wire this stateful loop to the viewer. The current viewer still uses an independent lifecycle and declines conflicting chords. See `research/KEY_EVENT_LOOP.md`; no UI/whole-input parity is claimed.

## Native menu commands (2026-10-13)

Main-menu/settings registration and conditional helper request order are recovered (`research/MENU_COMMAND_ROUTING.md`). Eight local traces cover every boolean combination with all six service bodies replaced. Recover those services and the separate Esc/Alt entry paths before wiring the request planner into the viewer. This is not a working original menu or a visual comparison.

## Native window query services (2026-10-13)

The controller map, window registry and parent opacity now supply menu query results. Combined original-code replay covers 1033 contexts and 2066 command executions; only four action bodies are logged. Recover those effects and real state construction before connecting the renderer. Limits and sources: `research/UI_WINDOW_STATE.md`.

## Focused UI input (2026-10-13)

The base child-first dispatcher and complete acceptance-mask filter now match 7120 local original-code cases with two virtual handler loggers. Child mask mutation is covered; real focus and widget callback implementations are not. Next: their mask assignments and concrete Alt/Esc actions, followed by live graph/rendering and paired captures. Sources and limits: `research/UI_FOCUSED_INPUT.md`.

## UI selection and focus transition (2026-10-13)

The complete selection/focus transition now matches 2024 local native replays, including callback-visible state and notification mutations. Notification service bodies and graph construction remain unported. The main-menu button's own key callback declines input, so follow its actual children rather than inventing Enter/Esc behavior. Next: helper effects, selection callers and concrete child callbacks; then renderer integration and paired captures. Evidence: `research/UI_FOCUS_TRANSITION.md`.

## Complete keyboard entry and UI gates (2026-10-13)

The whole native keyboard entry now joins focused UI and the binding-vector loop, with 2152 combined local comparisons. Query/interceptor/own-key/Begin/End endpoints are logged. Recover the query collection and interceptor effects, then actual focus graph and menu callbacks before wiring the viewer. The current viewer still exits on Esc. Evidence and input-domain limits: `research/KEYBOARD_ENTRY_DISPATCH.md`.

## Window classification and keyboard capture query (2026-10-13)

The native ordered classification collection and visible-window query now match 1054 local contexts with only allocation/free replaced. The 2152-case keyboard replay removes its query stub and the Rust side uses the real recovered window-state query. Recover class assignments, registry lifecycle and the interceptor next; actual menu actions and viewer integration remain unfinished. Evidence: `research/UI_CLASSIFICATION_QUERY.md`.

## Keyboard interceptor wrapper/adapter (2026-10-13)

Both native interceptor layers now match 1864 local cases, including signed results/context tokens, phases 0/2, timing arithmetic and callback mutations. The 2152-case combined keyboard replay executes them unchanged; only their external clock/scope/context/export endpoints are controlled. Recover actual module/registration lifecycle and endpoint effects, plus menu widget actions, before live integration. Evidence: `research/KEY_INTERCEPT_WRAPPER.md`.

## Native command block-mask query (2026-10-13)

The complete per-command mask query now matches 6418 local original-code cases without stubs, including all 3138 catalogue IDs. Esc/main-menu/settings commands are exempt from this query but remain subject to other gates. The show_menu opening action is still unidentified; reconstruct complete Begin/End routing and its ID-3 callback path next. Evidence: `research/COMMAND_BLOCK_MASK.md`.

## Complete Begin/End command executor (2026-10-13)

Both native dispatchers now match 1768 local original-code cases, including counters/time, override priority, real observer snapshot/iteration, removals/additions and post-plugin primary changes. Allocator and callback/plugin endpoints are controlled. Trace ID-3 observer/override/primary construction to the actual window action, then integrate with the viewer. Evidence: `research/COMMAND_BEGIN_END.md`.

## Held-command Continue (2026-10-14)

The third native command phase now joins full Begin/End with 2152 combined original-code cases: zero-count nested Begin, float32 elapsed/ramp arithmetic and live dt callback reads are recovered. Scheduling and actual menu callback registration/effects remain unported. Next: the real ID-3 observer/override/primary chain, held scheduling, then live UI integration and paired captures. Evidence: `research/COMMAND_CONTINUE.md`.

## External keyboard module boundary (2026-10-14)

The next actual keyboard-hook endpoint needs `Resources/plugins/XPLM_64.dll` from the same original Windows installation. The executable names that path/export, but the supplied reference tree has no module. Per the user's instruction, pause for this specific file after publishing the investigation. Menu callback registration and candidate runtime vtables remain unproven; see `research/UI_REGISTRATION_NEXT_INPUT.md`. Resume with DLL identity/exports and actual hook/context behavior, retaining the full 1:1 acceptance scope.

## Actual Windows XPLM key hook (2026-10-14)

The requested DLL is now present and the pause is resolved. Its full keyboard hook matches 1090 local Windows-code cases for phase routing, sniffers/hotkeys/window input and iteration-state requests. Validation/context/activation/user-callback/deferred-list bodies are controlled endpoints; EXE↔DLL replay integration remains next. Exact EXE/DLL build pairing is not proven by version metadata. See `research/XPLM_KEY_HOOK.md`; live Alt/Esc menus and paired visuals remain unfinished.

## Paired EXE/XPLM keyboard execution (2026-10-14)

2152 combined cases now execute original EXE and actual Windows DLL keyboard dispatch in one CPU address space, replacing the export logger. Rust composes both recovered layers. Explicit sniffer registrations and controlled validation/context/callback effects remain; this is not a live Windows loader or game. Next: real SDK context/list lifecycle and menu actions. Evidence: `research/KEYBOARD_PAIRED_REPLAY.md`.

## SDK initialized-TLS context and activation (2026-10-14)

1108 local native contexts verify push/activation/pop, raw enabled records, cached IDs and pending-unload request order. Both the 1090-case DLL hook and 2152-case paired keyboard replay remove SDK context-result stubs and execute the original functions. TLS is explicitly initialized with reserved capacity; real OS/module lifecycle and validation remain unported. Next: loading/unloading and deferred-list bodies, then real menu actions. Evidence: `research/XPLM_CONTEXT.md`.

## Window visibility progress (2026-10-14)

Registry extraction and classified reinsertion now match 700 combined original-code
cases. See [UI_WINDOW_ORDER](../research/UI_WINDOW_ORDER.md) for pointer/list semantics
and supported profiles. Continue with show/hide membership, widget alpha/parents and
listener callbacks, integrate the menu close route, then render original menus and
collect paired Alt/Esc screenshots and motion captures. Neither this list kernel nor
the earlier close-map replay proves a functioning original UI; UI verified coverage
remains zero. Whole-game requirements and the phase exit checks above remain active.

## Show/hide state progress (2026-10-14)

The complete original show/hide control flow, registry and membership mutations,
entry attachment and leaf-alpha stores now match 728 consecutive two-call
scenarios (1456 native calls). See [visibility evidence](../research/UI_WINDOW_VISIBILITY.md).
Recover parent/dirty propagation, owner/preparation/activation/refresh/event and
mutable listeners next; then share actual state with close/direct/fallback/focused
menu routes and integrate the original renderer. Stable manager and empty listener
profiles do not establish whole UI equivalence. Paired screenshots and motion
captures remain pending; UI verified share remains zero.

## Widget parent/dirty progress (2026-10-14)

Widget reparent now matches 744 original scenarios, with another 744 comparisons
for full 32-bit dirty propagation ([evidence](../research/UI_WIDGET_TREE.md)).
Next combine it with visibility on a shared graph: parent changes alter opacity
ancestry, so the earlier controlled visibility paths cannot prove this composition.
Binding/focus effects, parent callback bodies, vector growth and lifetime remain
unfinished. Then connect menu routes and original rendering and obtain matched
screenshots/motion captures; UI verified coverage remains zero.

## Combined window/widget graph (2026-10-14)

Show/hide, actual reparent and dirty propagation now compose on shared ancestry
and match 728 three-action sequences (2184 original visibility calls, including
728 toggle wrappers). Native visibility is checked after every action. See
[combined graph evidence](../research/UI_WINDOW_GRAPH.md). Next share this state
with close/direct/fallback/focused menu routes and recover remaining binding,
callback, listener and factory/lifetime effects. Live original layout/font/GPU
rendering and paired visual evidence remain unfinished; UI verified share stays
zero and all whole-game exit checks remain active.

## Graph-backed controller close (2026-10-14)

Controller close now toggles the shared graph instead of logging visibility.
728 two-call scenarios match 1456 original close executions and 367 actual
show/hide calls, including reset-to-show, map clearing and post-close queries
([evidence](../research/UI_MENU_CLOSE_GRAPH.md)). Next integrate direct/fallback/own/
focused routes and actual construction, remaining service/lifetime effects and
original rendering. Delete is still a request, not proven destruction. Paired
captures remain pending; UI/graphics verified coverage stays zero.

## Direct action / close bridge (2026-10-14)

Direct action now delegates close to the shared graph. Ordinary integration tests
verify guard, alpha/membership/map effects and identity error propagation. Combined
native replay is still required before extending this evidence to the complete
Esc route. See [bridge limits](../research/UI_MENU_DIRECT_GRAPH.md). Visual checks
and the full phase requirements remain outstanding.

## Combined direct/close replay (2026-10-14)

728 two-action scenarios now match 1456 original direct actions, 1092 actual close
helpers and 252 visibility calls on one graph. Profile: argument 0, UI mode 0/2;
mode-1 document/dialog and fallback/own/focused Esc entry remain unverified
([evidence and limits](../research/UI_MENU_DIRECT_GRAPH.md)). Continue into these
branches and actual construction before original renderer and matched captures.
UI/graphics verified coverage stays zero.

## Direct/close/document/dialog composition (2026-10-14)

The combined replay now includes UI mode1: 244 document/load requests, 48
controlled failures and 168 dialog construction/destruction requests match
cleanup ordering and state with actual graph-backed close. Total remains
1456 direct actions over 728 two-call scenarios. Actual document/dialog bodies
are controlled, not functional runtime dialogs ([limits](../research/UI_MENU_DIRECT_GRAPH.md)).
Next connect fallback/own/focused Esc and recover construction/lifetime/effects,
then original renderer and matched visual captures. UI verified share stays zero.

## Concrete fallback / graph bridge (2026-10-14)

Both recovered fallback direct routes now call the full direct handler and
actual graph-backed close; the property gate remains intact. Ordinary integration
tests and 746 previous native fallback cases pass. Combined native fallback
replay and own/focused Esc entry remain pending ([limits](../research/UI_MENU_ACTION_GRAPH.md)).
Construction/query/submission/lifetime, original rendering and paired captures
remain incomplete; UI verified share stays zero.

## Combined fallback / graph replay (2026-10-14)

728 two-action scenarios now match 1456 original fallbacks, 938 direct actions,
927 real close helpers and 220 visibility calls on shared state, including every
fallback route and both wrapper lifecycles ([scope](../research/UI_MENU_ACTION_GRAPH.md)).
Queries and construction/resolution/submission/effect bodies remain controlled.
Next connect own/focused Esc entry, recover actual construction/lifetime/effects
and original rendering, then matched visuals. UI verified share stays zero.

## Concrete key entry / shared menu graph (2026-10-14)

728 sequences now match 1456 original own/focused entries through concrete
fallback/direct/graph close, including consumed bytes, gates and post-action
state ([evidence](../research/UI_MENU_KEY_GRAPH.md)). Native focus is pinned to
a panel, child absent and effect bodies controlled. Viewer is still unwired.
Next recover construction/focus, actual opening ID3/Alt and remaining effects,
then original renderer and matched captures. UI verified share stays zero.

## Owner/selected binding (2026-10-14)

Full owner binding now matches 1045 original cases, including blur/stop callbacks
that alter owners, selected links and focus ([evidence](../research/UI_FOCUS_BINDING.md)).
These links are separate from widget parents. Next join binding with the actual
window/key graph and verify subsequent routing; virtual callback bodies and real
construction/lifetime remain unfinished. Opening ID3/Alt, original rendering and
paired captures are still required. UI verified share stays zero.

## Focus / menu composition (2026-10-14)

Owner binding now executes within menu graph-close and focused entry reads
current binding focus/child state. Ordinary integration tests verify clearing,
controlled blur focus loss and next-key routing, plus child-before-filter
consumption. Combined original replay is still required
([scope](../research/UI_MENU_FOCUS_GRAPH.md)). Actual callback/construction/lifetime,
opening ID3/Alt, viewer rendering and paired captures remain unfinished.
UI verified share stays zero.

## Combined mutable focus / menu replay (2026-10-14)

728 native own/focused sequences now compare the whole menu graph plus actual
owner binding: 1456 key entries, 300 closes, 72 visibility and 144 binding calls.
Owner/selected/focus state and next-key routing match; 16 blur/25 stop callbacks
include one asserted nonzero stop ([scope](../research/UI_MENU_FOCUS_GRAPH.md)).
Callbacks, real construction/lifetime and opening ID3/Alt remain unfinished.
Next recover those effects and original renderer, then matched captures.
UI verified share stays zero; viewer remains unwired.

## Input-node base construction (2026-10-14)

Complete native base/callback/string initialization now matches 64 poisoned
inline-name scenarios, including full object/sentinel bytes and allocator
snapshots ([evidence](../research/UI_INPUT_CONSTRUCT.md)). Input-node +0x60 is
a string; widget geometry +0x60 is a distinct parent field. Continue outer panel
constructors and real factory/child/layout/resource creation, then opening ID3/Alt
and original rendering. Heap-name/lifetime and matched captures remain unverified.
UI verified share stays zero.

## Common panel construction (2026-10-14)

Full common-panel ctor through input/callback/string bases now matches 256
poisoned inline scenarios, including strings, raw168/169/16a flags, interface
identities and complete allocation snapshots ([evidence](../research/UI_PANEL_CONSTRUCT.md)).
Next concrete main-menu interfaces/children/layout/resources, heap/localized
strings and live factory/key/focus integration. Opening ID3/Alt, original
rendering and matched captures remain unfinished; UI verified share stays zero.

## Embedded menu members (2026-10-14)

180 original member/capture/registration/vector-growth scenarios now match exact
bytes, capacities and allocation/copy/free order
([evidence](../research/UI_MENU_MEMBER_CONSTRUCT.md)). Next global interface
linkage1410c7bb0, then combine concrete panel prefix with common/member stages
and recover singleton/child/layout/resource creation. Callable/lifetime/large
allocation and original rendering remain unfinished; UI verified share stays zero.

## Global interface lifecycle (2026-10-14)

210 complete native ctor/destructor sequences now verify global-head insertion,
head/middle/tail unlink, retained detached links and bit0-only free requests
([scope](../research/UI_INTERFACE_CHAIN.md)). Concrete prefix composition is
now verified below; full derived teardown, resources, opening ID3/Alt and
original renderer remain incomplete. No paired captures;
UI verified share stays zero.

## Concrete Main Menu prefix (2026-10-15)

480 original executions now verify composition of common/input/callback,
global-interface and three embedded-member constructors through the exact
intermediate boundary0x140ecb592
([evidence and controlled endpoints](../research/UI_MAIN_MENU_CONSTRUCT.md)).
Whole object/allocated bytes, global head/links, parent-vector growth and
allocation/copy/free order match, including null optional callback allocation.
The earlier710 native constructor cases pass again; ordinary xp-app tests pass
196 with40 opt-in tests ignored. The new480-case native replay runs separately.

Next recover shared-dialog singleton branches and actual dialog/callback
construction from0x140ecb592, then remaining children, layout, resources,
heap strings/full teardown and live factory/key/focus integration. The prefix
is not a complete usable menu. Opening ID3/Alt, original GPU output and paired
screenshots/GIF/video remain unverified. Esc still exits the viewer. Overall
rubric stays41.4% implemented/28.2% verified; UI/Rendering verified share stays0.

## Bound dialog callback lifetime (2026-10-15)

324 unchanged native sequences/5832 calls now verify reciprocal registration,
copy inheritance, duplicate suppression and removal in six lifetime orders
([evidence and scope](../research/UI_CALLBACK_LINKS.md)). Allocation/free endpoints
are controlled. Ordered live sets and parameters match; internal tree bytes and
free-pointer order are not represented. No callable invocation is claimed.

This supplies a concrete dependency of dialog140ec9c10: it copies bound temporaries
at+1f8/+260 before the outer Main Menu ctor releases those temporaries. Next add
the heap-string/common-panel profile required by the dialog label, recover its
children/layout/resources and compare complete dialog/singleton composition.
Actual opening/Alt/viewer/GPU output and paired captures remain incomplete.
Compatibility shares stay unchanged.

## Heap-string panel construction (2026-10-15)

324 complete original common/wrapper/input/string ctor cases now verify mixed
inline/heap source ownership, nested identifier temporaries, spare capacity,
embedded zeros and aligned allocation/cleanup around4096-byte boundaries
([evidence and limits](../research/UI_PANEL_HEAP_STRINGS.md)). Full panel/source/
allocated bytes and ordered allocate/free snapshots match. Earlier64 input,
256 inline common and480 concrete-prefix cases pass after the shared refactoring.

Next compose actual dialog140ec9c10: translation/formatting, callback-member
copies,1410d6ca0 child construction and layout/widget bindings, then singleton
publication and remaining Main Menu construction. Failure/overflow, reentrancy
and full teardown remain unverified. No opening/Alt/viewer/pixel parity claim;
UI verified share and overall estimates stay unchanged.

## Dialog child factory (2026-10-15)

576 complete original outer-factory1410d6ca0 executions now match bundle/heap
bytes at allocator/constructor/resource/refresh entries and on completion
([scope](../research/UI_DIALOG_CHILDREN.md)). All32 external allocation failure
masks and distinct constructor return pointers are covered. Inline fifth-object
and empty list/hash initialization are ported; four nested constructors and
resource/refresh effects remain controlled. Actual null-parent acceptance and
failed required internal allocations are not verified.

Next recover these nested constructor/resource/refresh bodies and compose them
with dialog140ec9c10, its verified strings/callback links/interface, then actual
layout/widget bindings and singleton publication. Real opening/Alt, viewer/GPU
rendering and paired captures remain required; compatibility shares stay unchanged.

## Retained callback-list copies (2026-10-15)

Parent receiver141149460 now composes callback copying/peer registration with
120-byte list-node append semantics.72 unchanged native sequences include576
appends and360 source/base teardowns; list links/counts, live copied parameters/
peer sets and allocation/free sizes/counts match after each of936 actions
([scope and remaining byte-layout boundary](../research/UI_CALLBACK_LIST.md)).
The ordinary bound-temporary lifetime regression passes;200 normal tests pass.

Next compose actual child1410d7490, including refresh1411396d0, this receiver and
temporary cleanup, then replace its controlled outer-factory endpoint. Recover
remaining nested children/resources/refresh and complete dialog/layout/binding/
singleton construction. Native tree layout/custom callable/list destruction and
actual opening/Alt/viewer/GPU paired captures remain unverified. Shares unchanged.

## Third child and retained parent registration (2026-10-16)

Child1410d7490 now composes raw initialization, callback-container creation,
reciprocal copy/parent-list registration and temporary base teardown.108 two-child
sequences/216 normal original returns match96-byte object images,120-byte base
sentinels, allocation/refresh snapshots and live callback/list state
([scope](../research/UI_DIALOG_CHILD_CONTEXT.md)). Refresh can change the parent;
registration uses the post-refresh parent+58. A production adapter and ordinary
test connect this constructor to kind2 of the outer factory. Actual refresh and
other nested endpoints remain controlled; callback-tree backing is logical and
there is no combined original whole-factory heap replay yet.202 ordinary tests
pass,45 local opt-in tests are ignored in that run; native replay runs separately.

Next recover refresh1411396d0 and remaining nested child/resource/refresh bodies,
then complete native factory/dialog/layout/binding/singleton construction and
live key/focus integration. Opening/Alt, original GPU output and paired captures
remain required. Compatibility shares stay unchanged.

## Whole context-refresh caller (2026-10-16)

972 unchanged original1411396d0 executions now verify context selection, ordered
Zulu/day overrides, exact float edge behavior, string cleanup and live listener
next/head reloads ([scope](../research/UI_CONTEXT_REFRESH.md)). Full object/global/
listener snapshots and service arguments/order match. Engine/context/option/CRT/
time-conversion/free/notification endpoints are controlled; actual calendar and
text parsing are not yet established. An ordinary constructor test now invokes
the recovered refresh rather than assigning a completed payload.

Next recover these remaining services and compare original constructor-plus-
refresh/factory composition; recover other nested children and complete dialog/
layout/binding/singleton/Main Menu construction and teardown. Default FP rounding
only; status flags, invalid/nonterminating graphs and failure paths are unverified.
Opening/Alt, original GPU output and matched captures remain required. Shares unchanged.

## Original child and refresh composed (2026-10-16)

972 original child1410d7490 executions now include actual refresh1411396d0,
callback construction/binding, parent list copy and temporary teardown in one
normal-return run ([scope](../research/UI_CHILD_REFRESH_COMBINED.md)). Object/
base bytes, service snapshots/arguments/order and parent/callback state match.
Only leaf providers/CRT/time conversion/allocator/free remain controlled; fresh
observer lists and valid stable parents apply. Shared replay extraction retains
all972 independent corpus rows byte-for-byte and the standalone replay passes.
203 ordinary tests pass;47 opt-in tests are ignored in that run, with the new
972-case native replay executed separately.

This strengthens composition evidence; production behavior and estimates are
unchanged. Next recover remaining nested child/resource/provider bodies and
compare the complete factory, then finish dialog/layout/bindings/singleton/menu
construction and full teardown. Original opening/Alt/viewer/GPU and paired
screenshots/GIF/video remain required.

## Second child and shared receiver creation (2026-10-16)

920 complete native1410d6fc0 executions now verify global-interface registration,
raw data initialization, catalog selection/equality/variant assignment and retained
parent callbacks ([scope](../research/UI_DIALOG_CHILD_SELECTION.md)). Full object/
sentinel bytes, allocation/provider snapshots and logical receiver/callback state
match. Property/config/effect bodies remain controlled. Nonempty stable catalogs,
valid tags/parents, successful allocations and absent incoming optional text apply;
heap destination optional cleanup and full teardown remain unverified.

The kind1 adapter and ordinary second-then-third test connect actual constructed
receiver lists. Prior216 child and972 combined-refresh cases still pass;204 normal
tests pass with48 opt-in cases ignored in that run. This is not native full-factory
or live viewer evidence. Next recover first/fourth children and remaining provider
bodies, then complete factory/dialog/layout/bindings/singleton/menu/teardown and
original opening/Alt/GPU paired captures. Compatibility shares stay unchanged.


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
