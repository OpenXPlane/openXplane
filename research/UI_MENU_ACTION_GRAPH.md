# Concrete fallback -> direct -> shared graph close

`ui_menu_action_graph` reuses the complete recovered concrete fallback handler.
Its direct endpoint calls `ui_menu_direct_graph`, which invokes actual recovered
controller close on shared window/widget state. Other fallback construction,
resolution and submission effects delegate unchanged. UI mode is synchronized
between the recovered handlers, which read the same field in the original.
Errors propagate through the newly fallible direct endpoint.

Ordinary integration tests cover nonzero-engine direct routing, query-A/B direct
routing, actual graph alpha/membership/map effects, guard behavior and the
concrete property-2 zero gate. Both routing branches close the actual graph;
the gate performs no effect. Previous 746 native fallback scenarios still pass
with the Result API. Direct/close/document composition has separate native
shared-state evidence; these are not a combined fallback-chain native replay.

Remaining boundary: execute original fallback/direct/close together and connect
own/focused Esc entry. Constructor/submission/resolution, queries, lifetime and
other effect bodies remain services. Cross-handler mutations of shared engine/
UI state through those services require further evidence. Original construction,
viewer layout/fonts/GPU output and matched Alt/Esc screenshots/GIF/video remain
unfinished. UI/graphics verified coverage stays zero.
