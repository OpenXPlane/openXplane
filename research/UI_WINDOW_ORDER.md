# Window registry extraction and classified reinsertion

Reference: local X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Original-code observations

`0x1412eecc0..0x1412eed2c` is a destructive extraction, not a read-only lookup.
It scans the doubly linked list at impl+0x150, skips null entry pointers, and
matches entry+0x138 against the window token. It unlinks the first matching
node, decrements impl+0x158, requests a 24-byte free, and returns the entry.
The missing-token path reaches the sentinel and still unlinks/frees it;
this unsafe profile is rejected by the Rust port.

`0x1412e4220..0x1412e42d8` inserts an entry into that same list. It compares
signed classification fields at entry+0x170. With a nonzero low flag byte it
stops before the first existing class <= the target; with zero it stops before
the first class < the target. It allocates a 24-byte node and increments the
list count. Existing order is not sorted or deduplicated. Null entries in this
insertion scan are dereferenced by the original and are unsupported here.

Show helper `0x1412f97e0` calls extraction followed by insertion with flag 1;
hide body `0x1412d2370` calls both with flag 0. Consequently equal-class
entries move to different sides of their peers. The show helper does not read
its mode/time parameters in the inspected body; transition animation is not
established by those arguments. The subsequent show/hide widget alpha writes,
manager membership, callbacks and parent changes still require a complete port.

## Executed comparison

`tools/gen_ui_window_order_vectors.py` executes both original helpers unchanged
on 700 synthetic valid nonnull lists. Only allocation/free endpoints are
controlled. It checks returned entry identity, intermediate/final counts,
forward order and native backward links. Cases include empty-after-extraction
lists, repeated entry identities, duplicate/zero window tokens, signed class
extremes, unsorted lists and flags whose high bytes differ from their low byte.
Rust comparison passed all 700 cases; the ordinary tie/duplicate regression
also passed. The numerical corpus stays local at
`/private/tmp/openxplane-ui-window-order.txt`; original binary bytes/assets are
not published. Replay is opt-in through `OPENXPLANE_UI_WINDOW_ORDER_VECTORS`.

## Limits and next work

`ui_window_order::Registry` models stable entry identities separately from
list order. Allocation failure/max-size checks, null-entry insertion, unsafe
sentinel extraction, concurrent/lifetime mutation and actual viewer wiring
are not verified. This stage neither renders a window nor proves overall UI
parity. Continue with actual show/hide membership, alpha, parent and listener
bodies, then join the close/direct/fallback/focused routes and original GPU UI.
Matched screenshots and GIF/video remain required; UI/graphics verified share
remains zero.
