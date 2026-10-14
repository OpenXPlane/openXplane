# Menu graph with mutable owner/selected focus

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

`ui_menu_focus_graph` executes recovered owner/selected binding within the complete
menu key/fallback/direct/controller graph. Entry/widget identities map to binding
nodes and target identities explicitly. Focused entry reads current focus before
child dispatch; null focus declines, nonzero child result normalizes to 1 before
filter, and declined child continues on the captured panel. Other focused widget
classes require their own executor. Binding errors are port errors, not a claim
about native invalid-state failure atomicity.

## Combined original execution

`tools/gen_ui_menu_focus_graph_vectors.py` runs 728 sequences of original own key
callback then focused entry. There are **1456 native key executions**, 300 direct
actions, 300 real closes, 72 visibility bodies and **144 original binding-helper
executions**. The previous `0x1412f6250` stub is removed. Native key/filter,
fallback/direct/close, reparent/dirty/map stores and wrapper lifecycle run on
shared owner/selected/focus/graph memory. Blur, stop and child virtual endpoints
are controlled. Actual callbacks reached: 16 blur, 25 stop, including one nonzero
stop. The generator asserts nonzero-stop coverage instead of inferring it from
input configuration.

Inputs include panel/null initial focus, selected implementation/entry links,
Esc/Enter/0xbb and declined keys, down/release/zero flags, raw 168/169 gates,
mask rejection, child consumption, owner redirection and global-focus mutation
through blur/stop callbacks, plus all preceding graph/reset/map/environment and
document/dialog branches. Controlled blur can clear global focus, causing the
next native focused wrapper to return without dispatch. The binding helper
itself does not write global focus.

Rust compares consumed bytes, post-action queries, panel/global/engine fields,
map/modes, registry/membership/attachment, alpha, parents/flags/child vectors,
every binding owner/selected link, final global focus and complete request order.
All 728 sequences pass. Ordinary integration regressions also cover focus loss
and child-before-filter consumption. Corpus stays local at
`/private/tmp/openxplane-ui-menu-focus-graph.txt`; opt-in replay requires
`OPENXPLANE_UI_MENU_FOCUS_GRAPH_VECTORS`. Original files/table bytes and corpus
are not published; generator/test/source are published.

## Remaining boundaries

Actual blur/stop/child, activation/preparation/query/cache/document/dialog/
construction/resolution/submission and other close/allocator/CRT-copy effects
remain controlled. Parent callbacks are controlled; nodes/manager identities are
stable, masks fixed per sequence, vectors have spare capacity, chains finite,
listeners/protected/observer lists empty and documents synthetic inline strings.
Focus supports this concrete panel or null in this profile, not other live UI
classes. Real destruction and cross-handler global mutation need further work.

Recover real widget construction and actual callback/effect bodies, show_menu ID3
opening and Alt activation, then connect original layout/fonts/GPU output to the
viewer and verify matched screenshots/GIF/video. The viewer remains unwired;
this CPU evidence does not establish live menus or paired pixels. UI/graphics
verified coverage stays zero; the whole-game objective remains incomplete.
