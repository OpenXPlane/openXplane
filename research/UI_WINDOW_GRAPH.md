# Combined window visibility and widget ancestry

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Composition implemented

`ui_window_graph` now calls the recovered show/hide and widget reparent
implementations on one tree. Tree parents and raw widget alpha values are the
authoritative ancestry/opacity state; registered opacity paths are derived
before dispatch and after actual reparent. A show/hide leaf-alpha store is
applied to its mapped widget before the binding/parent callbacks execute.
Consequently the next action queries the new ancestry rather than the old
synthetic path. Reparent errors propagate instead of continuing to refresh.

This resolves a concrete boundary in the earlier controlled visibility replay:
a shown widget is reparented to the window implementation node, so a zero-alpha
old ancestor can stop influencing its visibility. With implementation alpha 1,
show becomes visible and the following toggle hides and detaches it. With a
zero or negative implementation alpha, show can remain nonvisible and a later
toggle can show again. Neither earlier isolated module proved these combined
sequences; no visual/runtime inference was drawn from its controlled paths.

## Original execution comparison

`tools/gen_ui_window_graph_vectors.py` executes complete original
show/hide `0x1412f97e0`/`0x1412d2370`, attachment list mutations,
registry extraction/reinsertion, reparent `0x1412f77a0`, first-match vector
search and dirty propagation `0x1412e48f0` together on a shared synthetic graph.
Each of 728 contexts executes two supplied actions and then original toggle
`0x1412fb9a0`: **2184 original visibility executions**, including 728 toggle
wrappers. After each action, original `0x1412fd160` queries positive cumulative
opacity. Its answer is compared to the Rust graph query at the same stage.

Final comparisons cover ordered registry/membership identities, entry attachment,
all leaf alpha bits, all widget parent identities/dirty flags/child vectors and
the complete remaining-effect request trace. Inputs retain signed class
extremes, duplicate tokens/membership, full show words, eight finite paths,
root-match/no-match and implementation alpha 1/0/-1/0.5. Native linked list
counts and backward links are checked. Both Rust tests passed, including the
ordinary show-from-zero-ancestor then toggle-detach regression. Previous 1456-call
controlled-reparent visibility comparisons also still pass after the fallible
reparent API update.

Binding, preparation, activation, parent virtual callbacks, memmove, allocation,
refresh and final event effects remain controlled. The reparent body itself is
**not** replaced, and all its parent/child/flag stores execute unchanged. The
parent callbacks in this combined profile only log requests; their mutable
behavior has separate standalone evidence, not combined-lifecycle proof here.
The numeric corpus remains local at `/private/tmp/openxplane-ui-window-graph.txt`.
Opt-in replay uses `OPENXPLANE_UI_WINDOW_GRAPH_VECTORS`. Original files/assets
are not published.

## Remaining work and limits

The manager/implementation identity and entry-to-widget mapping are stable;
listener head is empty, chains are finite and native vectors have spare capacity.
Actual owner/focus effects, parent callback bodies, mutable listeners, factory,
reset/delete/update/environment and object lifetime need recovery. This graph
is not yet connected to the close/direct/fallback/focused routes or live viewer.
The next gate is a shared close/menu route with actual graph state, then original
layout/font/GPU rendering. No paired original/port screenshots or motion capture
was produced here; UI/graphics verified coverage stays zero. Whole-game parity
and trajectory/content/audio/plugin requirements remain open.
