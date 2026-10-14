# Concrete own/focused key entry on shared menu graph

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

`ui_menu_key_graph::State` owns concrete key flags, fallback/direct panel state
and graph-backed controller state. Own key dispatch calls the recovered key
handler; its fallback endpoint now runs the complete fallback/direct/close chain.
Focused dispatch reuses `ui_input` with supplied focus/mask and absent child.
Errors propagate after the callback rather than inventing a native error result.

## Combined original execution

`tools/gen_ui_menu_key_graph_vectors.py` runs 728 synthetic sequences, each with
original own key callback `0x1412e1de0` followed by original focused entry
`0x1412c6300`: **1456 native key executions**. These reach 318 direct actions,
318 real close helpers and 73 visibility bodies. The local primary vtable copies
original concrete panel slots, overriding only the controlled preparation slot.
Original getters/constant/no-op slots, focused filter, fallback/property/thunk,
direct/document/dialog requests, close/reparent/dirty/map stores and wrapper
lifecycles execute unchanged. No original machine-code bytes are rewritten.

Cases cover Esc, Enter, 0xbb, declined 0xbc/letter keys, down/release/zero flags,
raw flags168/169, accepted/zero mask, UI modes0/1/2, engine mutation and all prior
shared graph/reset/map/environment inputs. Character is fixed to 27 in this
combined corpus; broader key/character classification has separate stage evidence.
Rust compares both consumed bytes and presence queries plus panel/global/engine
fields, map/modes, registry/membership/attachment, alpha bits, all parents/flags/
child vectors and complete effect/cleanup order. All scenarios pass. Ordinary
integration tests additionally check absent focus, mask rejection, release
rejection, flag169's consumed no-op and actual Esc graph-close behavior.

Corpus stays local at `/private/tmp/openxplane-ui-menu-key-graph.txt`; opt-in
comparison uses `OPENXPLANE_UI_MENU_KEY_GRAPH_VECTORS`. Original files, table bytes
and numeric corpus are not published; only generator/source/tests are published.

## Remaining boundaries

Focus is pinned to this panel for the native wrapper, child+0x10 is null, mask is
stable during callbacks and manager/widget identities remain stable. Actual
binding/focus lifecycle, activation, preparation, query/cache/document/dialog/
construction/resolution/submission, remaining close effects and parent callbacks
are controlled. Vector capacity has spare room, listeners/protected/observer
lists are empty and documents synthetic inline strings. Real construction and
lifetime invariants remain unverified.

This validates key handling on a constructed synthetic menu, not opening through
show_menu ID3, Alt activation, real application focus or viewer rendering. The
viewer is still unwired; this does not claim its Esc now shows/closes an original
menu. Recover construction/focus/effects and opening/Alt routes, integrate original
layout/font/GPU output and obtain matched screenshots/GIF/video. UI/graphics
verified coverage remains zero and whole-game parity remains incomplete.
