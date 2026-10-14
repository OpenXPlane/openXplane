# Direct menu action -> graph-backed close

The bridge reuses the complete recovered direct handler and overrides only its
close endpoint with `ui_menu_close_graph::dispatch`. All remaining direct effects
are forwarded unchanged. The caller supplies controller identity; mismatch
returns an error before modifying the graph. That validation is a port contract,
not a claim about native invalid-pointer behavior.

Ordinary composition tests verify re-entry guard suppression, actual graph alpha/
membership changes, map clearing, request order and mismatch error propagation.
The previously established 694 native direct-handler cases remain a separate
stage comparison. The combined replay below now proves one explicit bridge
profile; it does not establish the entire native menu route.

Next extend the combined replay to the mode-1 document/dialog branch,
then join the concrete fallback and own/focused Esc callback. Actual widget
construction, effect bodies and lifetime remain incomplete. Viewer rendering,
original Alt/Esc screenshots/GIF/video and whole-game parity remain unproven.

## Combined original-code replay

`tools/gen_ui_menu_direct_graph_vectors.py` executes 728 synthetic two-action
scenarios: 1456 original direct actions (`0x140f0a2c0`), 1092 original close
helpers (`0x14150ce20`) and 252 reached visibility bodies. Close is not stubbed.
Original getter/mode/toggle/show/hide, registry/membership, actual widget
reparent/dirty and map stores execute on the same graph. Original presence is
queried after each action. Raw guard 255 suppresses the whole action; unguarded
actions normalize the close flag from c8 and clear c9 on completion.

The Rust bridge comparison checks final panel guard/c8/UI mode, both presence
queries, controller map, modes, registry/membership, attachments, alpha bits,
parents/flags, child vectors and remaining-effect request order. All scenarios
passed. Corpus is local at `/private/tmp/openxplane-ui-menu-direct-graph.txt`;
opt-in replay uses `OPENXPLANE_UI_MENU_DIRECT_GRAPH_VECTORS`. Original files
and corpus are not published.

This combined profile uses argument 0 and UI mode 0/2, so the mode-1 document/
dialog branch is not reached. Preparation/controller singleton lookup and
remaining close/window/callback/allocation/CRT-copy effects are controlled;
manager identity is stable, listener/protected/observer lists empty and vectors
have spare capacity. Initial state is synthetic rather than real construction.
Mode-1 combined effects, fallback and own/focused Esc entry, lifetime and viewer
rendering remain unverified. No paired visual capture was produced; UI/graphics
verified coverage remains zero.
