# Plan for a full implementation

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
