# Widget reparent and dirty propagation

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Observed native implementation

`0x1412f77a0..0x1412f7908` first calls binding helper `0x1412f6250`.
The argument is widget+0x68 when nonnull, otherwise the requested parent.
It then reloads the old parent at widget+0x60, finds the first child identity
in parent+0x70..0x78 (original `0x1422c5240`), shifts following pointers and
shrinks the vector end. Parent virtual +0x188 runs after erase. The function
reloads widget+0x60 **after** this callback and ORs mask 0x0d into that node's
flags at +0xf8, propagating newly introduced bits through its ancestors.

It next stores the captured requested parent into widget+0x60. A nonnull new
parent gets an appended child pointer and virtual +0x180 callback. The child
flags and its parent are both reloaded after this callback: flags & 3 selects
mask 0x0f, otherwise mask 0x0d. This mask is stored/propagated on the current
parent, which need not remain the requested parent after callback mutation.
There is no equal-parent early return in this entry: first-match removal and
append also happen when old and requested parents are equal. Duplicate child
identities remain, and only the first occurrence is removed.

Dirty helper `0x1412e48f0..0x1412e4945` ORs all requested bits into the current
node. Only bits absent before the store continue upstream. At each ancestor
step bit 1 becomes bit 2; if bits 4 or 8 are present, bit 1 is introduced.
Propagation ends on a null parent or no newly introduced bits. Raw bit
semantics beyond these operations are not inferred as named layout properties.

## Executed evidence

`tools/gen_ui_widget_tree_vectors.py` runs 744 original reparent scenarios,
then 744 standalone original dirty-helper cases. Reparent, first-match search
and dirty machine code execute unchanged. Binding, parent virtual callbacks
and CRT memmove are controlled endpoints; memmove applies the requested byte
copy, leaving the native erase size/vector-end operations intact. Native vector
capacity has spare space, so allocation/growth branches are not exercised.

Inputs include null/equal/distinct parents, duplicate children, raw override
binding identities, flags with high bits, already-present dirty masks and
callbacks that change parent or child flags. The comparison checks every node's
parent/flags, all ordered child vectors and callback request order. Standalone
dirty comparisons use full 32-bit masks on the resulting finite chains.
All 1488 original comparisons passed; the ordinary regression also checks
new-bit-only propagation. The corpus stays local at
`/private/tmp/openxplane-ui-widget-tree.txt`; replay is opt-in through
`OPENXPLANE_UI_WIDGET_TREE_VECTORS`. Original binary/assets are not published.

## Integration limits

Binding helper contains focus-link behavior which is not implemented here.
Actual parent callbacks, vector growth/allocator failures, missing child
membership, null parent after callback, cycles and object destruction are not
verified; invalid native states are rejected rather than silently accepted.
Test graphs have finite parent chains and valid pointer vectors, but do not
claim full application construction/lifetime invariants.

The widget tree now needs to be shared with visibility/menu routing. In
particular, actual reparenting changes the ancestry used for cumulative
opacity, while the preceding controlled reparent replay intentionally retained
its supplied opacity paths. Its zero-ancestor/repeated-membership results are
limited to that controlled profile, not a claim about the completed runtime.
Connect both stages on one native-equivalent graph and verify consecutive
show/hide/close actions before renderer integration. Actual original menu
layout/font/GPU output and paired Alt/Esc screenshots/GIF/video remain missing.
UI verified share remains zero.
