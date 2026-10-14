# Main-menu embedded member construction and registration

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Concrete main-menu ctor invokes `0x140ec3280` at +0x1b0/+0x208 and
`0x140ec3330` at +0x260. Each complete member ctor clears its leading word,
stores parent+8, sets interface identity at +0x10, creates an inline callable
at +0x18 capturing the member's own identity, stores its address at +0x50,
destroys the temporary callable and, for nonnull parent, appends interface
address member+0x10 to the parent's +0xd0 vector. The variants differ in callable
identity (0x14260d018/0x14260d050). Higher-level callback meaning is unresolved.

`ui_menu_member_construct` models the 0x58-byte member, logical registry values,
capacity and backing bytes. Registration uses native growth max(length+1,
capacity+capacity/2), writes the new slot before copying, frees old nonnull
storage and commits new capacity/data. Null-parent construction leaves registry
unchanged. Large aligned allocation and invalid-vector profiles are unverified.

## Native evidence

`tools/gen_ui_menu_member_construct_vectors.py` runs complete original member,
callable copy/temporary destructor, registration `0x1412fa2e0` and vector-growth
`0x1405dcc70` bodies unchanged. Only allocation/free/CRT-copy endpoints are
controlled. 180 cases cover both variants, null/non-null parents, capacities
0/1/2/3/4/8, empty/full/spare lengths and three poison patterns. Exact member
bytes, ordered registry values, capacity/backing address/bytes and allocation/
copy/free order match Rust. All 180 comparisons and clippy pass. The initial
failed test was only an incorrect expected case count (348 instead of 180);
all per-case state comparisons already passed before that correction.

Corpus remains local at `/private/tmp/openxplane-ui-menu-member-construct.txt`;
opt-in comparison requires `OPENXPLANE_UI_MENU_MEMBER_CONSTRUCT_VECTORS`.
Original files and corpus are not published; source/generator/test are published.

## Remaining construction dependencies

These are member constructors, not the full concrete panel. Next combine them
with common-panel initialization and concrete interface/guard/source stores.
`0x1410c7bb0`, called for panel+0x188, links a member through global0x14588f558
and updates the previous head: it must not be reduced to zero initialization.
Its lifecycle and the subsequent singleton dialog/layout/resource construction
remain to be recovered. Actual callable bodies, large allocations, reentrancy,
object destruction and runtime construction invariants remain unverified.

Opening ID3/Alt, viewer layout/fonts/GPU output and paired screenshots/GIF/video
remain incomplete. No visual evidence was produced; UI verified share stays zero.
