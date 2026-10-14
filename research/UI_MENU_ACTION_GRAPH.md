# Concrete fallback -> direct -> shared graph close

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

`ui_menu_action_graph` reuses the complete recovered concrete fallback. Its
direct endpoint calls the complete direct handler and actual graph-backed close;
other construction/resolution/submission effects delegate unchanged. UI mode is
synchronized across handlers and errors propagate through the direct endpoint.
Ordinary integration tests cover both direct routes and property-2 zero gating;
the preceding 746 native fallback stage comparisons remain separate evidence.

## Combined original execution

`tools/gen_ui_menu_action_graph_vectors.py` runs 728 synthetic two-action
scenarios: 1456 original fallback calls, 938 reached direct actions, 927 real
close helpers and 220 visibility bodies. Fallback/property2/engine-byte/enum,
virtual direct thunk, direct/close, mode/toggle/show/hide, registry/membership,
widget reparent/dirty and map stores execute unchanged on shared state. Wrapper
constructors/destructors also run unchanged for both empty fallback wrappers and
bound direct-dialog callbacks. Close and direct endpoints are not stubbed.

Inputs include UI modes 0/1/2, zero/nonzero engine byte, independent first/second
query results and cache response, optional engine-byte mutation after first
query, guard/c8, eight finite opacity paths, duplicate tokens/membership,
registry class/order, reset mutation, deletion and environment/controller gates.
All fallback branches are reached: property gate, engine direct, query/cache
direct, null submit and construction/resolution/destroy/submit. Mode1 direct
paths additionally include document/dialog request flow. Native presence is
queried after both actions.

Rust compares panel/global/engine fields, both presence results, map/modes,
registry/membership/attachment, alpha bits, parents/flags/child vectors and full
query/effect/cleanup request order. All 728 scenarios pass. Local numeric corpus:
`/private/tmp/openxplane-ui-menu-action-graph.txt`; opt-in replay uses
`OPENXPLANE_UI_MENU_ACTION_GRAPH_VECTORS`. Original files and corpus are not
published; generator and test are published.

## Limits and next work

Queries, cache, preparation/controller singleton, actual document/load/binding/
dialog/construction/resolution/submission and remaining close/window/callback/
allocation/CRT-copy effects remain controlled. Actual object construction and
lifetime are not proven by requests. Manager identity is stable, listener/
protected/observer lists empty, chains finite, vectors have spare capacity and
documents are synthetic inline strings. Argument passed to direct is 0.

Join own/focused Esc routing to this chain and verify the entire entry on shared
state next. Real widget lifetime and effect bodies, original viewer layout/
fonts/GPU output and matched Alt/Esc screenshots/GIF/video remain unfinished.
No visual capture was produced; UI/graphics verified coverage stays zero and
whole-game parity remains incomplete.
