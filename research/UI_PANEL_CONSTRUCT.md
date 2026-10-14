# Common panel constructor through original input base

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered construction

`ui_panel_construct` ports complete common ctor `0x14129c7a0`, through wrapper
`0x14129fc10` and verified input-node/callback/string constructors. RDX/R8/R9
supply label/identifier/auxiliary string objects. The identifier feeds the base
name at +0x60; label/auxiliary copies populate +0x128/+0x148. Three raw stack
argument bytes populate +0x168/+0x169/+0x16a, and key mask+0x18 is 0xfff.
All original source strings are consumed/reset at the end; inline residual bytes
are retained exactly. Intermediate class/interface table identities are preserved.

`key_state` exposes the stored168/169 bytes to the concrete key state instead of
requiring handwritten flags. It does not turn the common class into the concrete
main-menu class: that ctor still needs its own interface/child/resource setup.
Reference addresses are metadata identities, not host vtable function pointers.

## Native comparisons

`tools/gen_ui_panel_construct_vectors.py` executes all nested original bodies
unchanged; only allocator endpoints are controlled. 256 cases combine inline
lengths0..15, distinct label/identifier/auxiliary contents and residual bytes,
four object/allocation poison patterns and four flag triples including0/0/1.
Rust compares the complete 0x170-byte object, all consumed source strings,
sentinel buffers and full object snapshots before four allocations40/24/40/48.
All 256 comparisons pass. The allocator adapter extends base snapshots with the
untouched derived tail, rather than treating a base allocation as a fully
initialized panel. Getter extraction matches original stored flags in every case.

Corpus stays local at `/private/tmp/openxplane-ui-panel-construct.txt`; opt-in
comparison uses `OPENXPLANE_UI_PANEL_CONSTRUCT_VECTORS`. Original files and byte
corpus are not published. Only source/generator/test and evidence are published.

## Remaining boundaries

Profile requires separate valid inline string objects, capacity15/length<=15,
successful allocations and exact0x170 image size. Heap/localized strings, large
allocation guards/failure, destructor/lifetime and reentrant allocation remain
unverified. The raw constructor model is not yet the live node factory.

Concrete main-menu ctor `0x140ecb340`, outer kind11 window construction, child/
geometry/layout/resource setup and actual callback bodies remain unfinished.
Continue those stages and connect real initialized nodes to the key/focus graph,
then show_menu ID3/Alt activation and original GPU output. Paired screenshots/
GIF/video and whole-game parity remain unproven; UI verified coverage stays zero.
