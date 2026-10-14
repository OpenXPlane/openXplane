# Input-node base construction

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Constructor chain and recovered stores

Main panel ctor `0x140ecb340` calls `0x14129c7a0`, which calls `0x14129fc10`,
which calls input-node base ctor `0x1412a70a0`. This base initializes owner+8 and
selected+0x10 to null, mask+0x18 to 0xfff, interface identities, callback storage
at +0x48, name string at +0x60 and empty containers at +0x90/+0x100/+0x110.
It is an input-node object: its +0x60 holds a string, unlike the separate widget
geometry object whose +0x60 is a parent pointer. These objects must not be merged.

Callback ctor `0x1410c7b50` allocates a 40-byte sentinel with three self links and
word0x101 at +0x18. Base ctor then allocates 24/40/48-byte sentinels with first two
self links; payload bytes are not initialized. Container counts/pointer fields
are cleared. The unaligned zero store at object+0xf4 includes +0xf8, but trailing
padding is retained. The input name is consumed/reset after construction.

String-copy ctor `0x1405dd5c0` copies **all 16 inline bytes**, not merely visible
characters. Thus source residual bytes remain in the destination, and source
cleanup clears only first byte/length/capacity, retaining other inline bytes.
Reference vtable addresses in the raw port are opaque identities, not host
function pointers. Transient base/interface stores are retained in allocator
snapshots rather than flattened into only the final table.

## Original execution evidence

`tools/gen_ui_input_construct_vectors.py` executes the complete original base
ctor, callback ctor and string-copy ctor unchanged. Only allocator endpoints
are controlled. 64 scenarios cover inline lengths0..15 and four distinct poison
patterns in object/name/allocation bytes. Rust compares the full 0x128-byte object,
consumed source string, all four sentinel buffers and object snapshots before
allocations. All 64 comparisons pass; allocation order is 40/24/40/48.

Corpus stays local at `/private/tmp/openxplane-ui-input-construct.txt`; opt-in test
uses `OPENXPLANE_UI_INPUT_CONSTRUCT_VECTORS`. Original binary/assets and corpus
are not published. Only source/generator/test and evidence are published.

## Limits and next steps

Verified profile is a valid inline string with capacity15 and length<=15,
successful nonzero allocator results and exact object size. Heap-string copying/
large allocation guards, allocator failure, destructor/lifetime and reentrancy
are not verified. This is raw base initialization, not complete panel creation,
widget geometry or GPU rendering. It is not yet the live menu node factory.

Continue outer constructors to recover flags168/169/16a, strings, panel interface
members and child/layout/resource construction, then join real initialization
with the recovered key/focus graph. Main panel's immediate base call passes raw
bytes0/0/1 to those flags, observed statically; outer-constructor execution is
still required. Nearby `0x14129fcf0` uses inline identifier `menu bar`, but that
alone does not prove Alt activation. Opening ID3/Alt, original rendering and
matched screenshots/GIF/video remain unfinished; UI verified share stays zero.
