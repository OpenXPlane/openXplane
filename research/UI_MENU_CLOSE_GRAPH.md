# Controller close on the shared window/widget graph

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Implementation

`ui_menu_close_graph::State` owns the actual shared graph, kind map, deletion
kinds, raw global/controller gates and modes aligned with stable entry identities.
Mode lookup follows current registry order; reordering does not move mode values
to other windows. Presence queries use the mapped token and current ancestry.

Full close `0x14150ce20` now queries presence, updates, obtains the current mapped
registered window, independently rereads mode 4 and mode 6 around reset requests,
performs actual graph toggle, conditionally requests deletion and stores map
value zero, updates with the raw flag, handles environment/command/refresh gates
and always performs final update. Reset may change mode or alpha before toggle.
Map clearing retains the key and is separate from actual node destruction.

## Combined original-code comparison

`tools/gen_ui_menu_close_graph_vectors.py` executes 728 synthetic contexts with
two consecutive kind-11 close calls: **1456 complete native close executions**.
A code observer counts **367 actual visibility-entry executions**. Original
close/query/registered getter, mode queries, toggle/show/hide, registry/membership,
reparent/dirty and map accessor/store run unchanged. Original `0x141512eb0`
queries presence after each close.

Controlled update can switch token 11 to 12 before getter; reset can change
mode 4 to 6 (two requests) and clear leaf alpha (toggle then shows). Inputs
include duplicate tokens/membership, signed classification extremes, eight
finite widget paths, implementation alpha 1/0/-1/0.5, mapped/null initial state,
delete/global/controller/environment gates and raw caller flag bytes. The second
close uses flag 255. Rust compares both post-close queries, final map token,
all modes, registry/membership, attachment, leaf alpha bits, parents/flags,
ordered child vectors and complete remaining-effect request order.

All comparisons passed. The ordinary regression verifies double reset,
reset-to-show, map clearing while graph visibility remains true and a second
close issuing only final update. Numeric corpus is local at
`/private/tmp/openxplane-ui-menu-close-graph.txt`; replay requires
`OPENXPLANE_UI_MENU_CLOSE_GRAPH_VECTORS`. Original binary/assets and corpus
are not published; generator, implementation and opt-in tests are published.

## Limits and next work

Reset/delete/update/environment/command/final bodies remain controlled, as do
binding/focus, activation, parent callbacks, allocation/CRT copy, graph refresh
and final event. Protected-kind/observer/listener lists are empty. The mapped
window remains nonnull/registered after update; native null-map factory, mutable
listeners, alternate managers, vector growth and destruction/lifetime are not
verified. Controlled deletion does not alter registry/membership.

Connect this graph-backed close to direct/fallback/own/focused menu routing and
actual widget construction/viewer state. Original layout/fonts/GPU rendering,
matched Alt/Esc screenshots/GIF/video and whole-game parity remain incomplete.
CPU comparisons do not raise UI/graphics verified coverage.
