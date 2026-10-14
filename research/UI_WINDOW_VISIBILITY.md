# Window show/hide state and native membership mutations

Reference: supplied X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered behavior

Show/hide entry `0x1412f97e0..0x1412f997e` captures implementation at manager+0x128,
finds the first registered matching window token and computes root-to-leaf
float32 opacity. The **entire show dword** selects show versus hide. Mode/time
arguments are not read in this body; animation is not established here.

Show, when initially nonvisible, calls `0x1412e42f0`: stores manager in
entry+0x88, appends the entry to the manager+0x90 doubly linked membership list,
then calls owner propagation `0x1412f6250`. An ancestor-chain match against
`0x145a0def0` conditionally calls `0x1412e47a0` with flag 1. Classification-2
listeners are then traversed (the current executed profile has no listeners).
Even when already visible, show destructively extracts the first token match
and reinserts it with flag 1, writes **only the widget leaf alpha** to f32 1,
calls reparent helper `0x1412f77a0(widget, impl)`, then refresh `0x1412fa3b0`.

Hide does nothing when initial cumulative opacity is nonpositive. Otherwise
`0x1412d2370..0x1412d24bf` extracts/reinserts with flag 0, calls preparation
`0x1412fc4b0` on entry+0x80, conditionally calls `0x1412e4680`, clears entry+0x88,
calls owner propagation with null when captured manager+0x128 is nonnull,
removes the **first** matching entry from the membership list, writes leaf
alpha to f32 0, then requests reparent-to-null and refresh. Its classification-2
hide listeners follow refresh. Finally the outer helper calls
`0x1410c90c0(4, 1, original window token)`.

Showing does not replace ancestor alpha. In the controlled parent-service
profile, a zero ancestor can keep a window nonvisible after leaf alpha is set
to 1. Repeated show can consequently append repeated membership entries.
This behavior is retained; no deduplication or assumed always-visible result
is added. `toggle` uses the recovered !positive-opacity decision.

## Original execution and Rust comparison

`tools/gen_ui_window_visibility_vectors.py` executes complete show/hide,
attachment-list construction/unlink, registry extraction/reinsertion, opacity
queries and alpha stores unchanged. There are 728 synthetic contexts with two
consecutive calls each (1456 original visibility executions). Fields compared:
ordered registry identities, membership identities/counts, each entry+0x88,
all leaf alpha bits and effect request order, including alpha at reparent time.
Native forward/backward links are checked after both calls. Inputs include
signed classification extremes, equal classes, duplicate tokens, duplicate
membership identities, eight finite opacity paths, root-match/no-match,
already-visible/nonvisible and full show words 0/1/256/0xffffffff.

Allocation/free and owner/preparation/activation/reparent/refresh/event bodies
are controlled endpoints. Their request order is established, their complete
internal effects are not. Original listener head is empty and one manager/impl
identity is stable throughout. Original code bytes are not rewritten. The
numeric corpus stays local at `/private/tmp/openxplane-ui-window-visibility.txt`;
only generator, Rust implementation, opt-in replay and ordinary regression are
published. Replay uses `OPENXPLANE_UI_WINDOW_VISIBILITY_VECTORS`.

Both local Rust tests passed. The ordinary regression checks the full-word
show decision, preserved zero ancestor, repeated membership insertion and
nonvisible-hide no-op. These are state/CPU comparisons, not pixel evidence.

## Remaining boundaries

The manager's actual ancestor chain is represented by a predicate; alternative
entry+0x88 owners, manager/implementation replacement, populated mutable listener
lists, invalid/missing entries and unsafe object lifetimes are outside this
profile. Widget parent helper `0x1412f77a0` visibly removes/adds child identities,
invokes parent callbacks and propagates dirty flags; its body remains a service
here, not a proven renderer or focus transition. Owner/preparation/activation,
refresh and event bodies also require recovery. The close/direct/fallback/menu
routes must then share the actual registry/widget state, rather than independent
replay services. Viewer integration, original layout/fonts/GPU output and matched
Alt/Esc screenshots/GIF/video remain incomplete. UI verified coverage stays zero.
