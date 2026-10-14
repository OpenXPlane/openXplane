# Direct menu action -> graph-backed close, including document/dialog flow

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The bridge reuses the complete recovered direct handler and replaces its close
endpoint with actual graph-backed controller close. Other effects delegate
unchanged. Controller identity mismatch is a port error contract, not a claim
about original invalid-pointer behavior. Ordinary integration tests cover guard,
graph alpha/membership/map changes and error propagation.

## Combined native comparison

`tools/gen_ui_menu_direct_graph_vectors.py` executes 728 synthetic two-action
scenarios: 1456 original direct actions, 1092 real close helpers and 252 reached
visibility bodies. Close is not stubbed. Original query/getter/mode/toggle,
show/hide, registry/membership, reparent/dirty, map stores, enum predicate and
callback-wrapper construction/destruction execute unchanged on shared state.
Original presence is queried after each action. Modes 0/1/2 are now exercised;
argument is 0. Guard 255 suppresses action and unguarded calls clear c9 at end.

Mode-1 paths make 364 notification requests, 244 document/load requests, 48
controlled load failures and 168 dialog construction/destruction requests.
All 244 document paths reach temporary cleanup. Load-result responses, byte
validation and synthetic inline string `test.sit` are controlled inputs, not
actual content loading. Dialog wrappers bind callback values 0/3 and release
second then first; global byte 0x1457770c6 is stored before component requests.

Rust compares both presence queries, panel c9/c8/UI-mode/global-byte state,
controller map/modes, registry/membership/attachment, leaf alpha bits, parents,
flags, child vectors and complete effect/cleanup request order. All 728
scenarios pass. Corpus stays local at
`/private/tmp/openxplane-ui-menu-direct-graph-dialog.txt`; opt-in test uses
`OPENXPLANE_UI_MENU_DIRECT_GRAPH_VECTORS`. Original files and numeric corpus are
not published. The preceding mode0/2 corpus was narrower; current generator
now includes mode1. Separate 694-case direct evidence remains applicable.

## Limits and next work

Preparation/controller singleton lookup, document/query/validation/load,
binding/dialog constructor/component/destructor and remaining close/window/
callback/allocation/CRT-copy effects remain controlled. Manager identity and
entry/widget mapping are stable; listener/protected/observer lists empty,
parent chains finite, vectors have spare capacity and document strings are
inline. Actual construction/destruction, other arguments/string storage, mode
mutation through effect callbacks and real document/dialog implementations need
further evidence. These tests prove request composition, not functional dialogs.

Join concrete fallback and own/focused Esc routing to this shared state next.
Actual widget construction, original layout/fonts/GPU output, matched Alt/Esc
screenshots/GIF/video and whole-game parity remain unfinished. No paired visual
capture was produced; UI/graphics verified coverage remains zero.
