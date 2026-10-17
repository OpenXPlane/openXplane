# Dialog child factory1410d6ca0

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Caller and recovered factory

Dialog constructor140ec9c10 invokes this factory on its embedded40-byte bundle
at dialog+2d0, before layout/widget binding. The bundle receives five pointers.
The complete outer factory is now ported in `ui_dialog_children`; four nested
constructor bodies and resource/refresh effects remain explicit services.

In order, the factory requests0x3e0 bytes and calls1410d6a40 with low byte1;
requests0x2d0 and calls1410d6fc0 with the first returned object; requests0x60 and
calls1410d7490 with the second returned object; then requests0x3e8 and calls
1410d75a0 with the currently stored second object and third returned object.
Bundle slots use constructor results, not an assumption that they equal their
allocation. Each failed external allocation skips that constructor and stores0.
No semantic widget names are inferred for these four types.

The fifth request is0xb0 bytes. Its constructor body is inline in the factory:
base table142642d00, empty120-byte list sentinel, flag18=0, derived table142642fc0,
opaque resource lookup141a64260(global146124398,20) at+20, then empty fields and
an inline hash structure. A32-byte sentinel has two self links;16 pointer buckets
are allocated/fill with that sentinel through original1405de740. Hash fields
include float1 at+50, mask7 at+80 and bucket count8 at+88. Unwritten bytes retain
poison. Fields+98/+a0/+a8 are null before final refresh141139460; only after
refresh returns is the fifth pointer published to the bundle. If the0xb0 request
fails, all its internal allocations/resource/refresh calls are skipped.

## Native comparison and scope

`tools/gen_ui_dialog_children_vectors.py` executes the entire original outer
factory and its inline fifth-object/list/hash-vector initialization unchanged.
Four nested constructors1410d6a40/1410d6fc0/1410d7490/1410d75a0, resource lookup,
refresh and allocator are controlled. The empty vector resize1405de740 executes
its original body. This does **not** verify the controlled constructor bodies.

576 cases combine three poison values, all32 failure masks for the five external
allocations, nested constructor results equal to allocation or allocation+16,
and opaque resource results0/7/all64 bits set. Nested services stamp synthetic
allocation contents so every forwarding/return-store decision is observable.
Refresh mutates fifth-object fields, verifying that its result state is retained.
Every allocator/constructor/resource/refresh entry compares the complete bundle
and all current blocks; final bundle and every allocated block also match.
All576 cases pass. Internal120/32/128-byte allocations are successful only.

Null-root scenarios prove the outer factory's conditional control/forwarding
with controlled child endpoints. They do not prove actual child constructors
can accept a null parent or that the game gracefully survives allocation failure.
The source rejects failed required internal allocations rather than reproducing
native null dereferences. Addresses/tables are reference identities, not callable
host interfaces. General nonempty hash growth/erase, destructors, reentrancy,
concurrency and allocator failure behavior beyond the declared profile are unverified.

The ordinary regression checks forwarding of distinct constructor return pointers
and suppression of resource/refresh work when the fifth object is absent.
Numeric corpus stays local at `/private/tmp/openxplane-ui-dialog-children.txt`;
opt-in replay uses `OPENXPLANE_UI_DIALOG_CHILDREN_VECTORS`. Original files/assets
and corpus are not published; source/generator/test/report are published.

## Next dependencies

The [callback-list receiver](UI_CALLBACK_LIST.md) now has a72-sequence combined
copy/append/lifetime comparison; child1410d7490 and the dialog use this dependency.
Third-child1410d7490 now has a [216-call complete ctor comparison](UI_DIALOG_CHILD_CONTEXT.md)
and an ordinary outer-factory adapter test; its refresh remains controlled and
callback-tree backing logical. Second-child1410d6fc0 now also has a [920-case complete outer constructor replay](UI_DIALOG_CHILD_SELECTION.md),
with property/config/effect services controlled and ordinary receiver integration.
Recover first/fourth constructors1410d6a40/1410d75a0 and remaining provider bodies
and compare combined factory composition, then join
dialog140ec9c10's strings, callback copies and global interface.
The full dialog still needs layout/widget bindings; Main Menu singleton
publication and remaining construction are unfinished. Opening ID3/Alt, original
viewer/GPU output and paired screenshots/GIF/video remain missing. Esc still
exits the viewer. Shares remain41.4% implemented/28.2% verified overall and
Rendering/UI verified0. This is construction evidence, not visual parity.


2026-10-17 update: [first child](UI_DIALOG_CHILD_ROOT.md) now has192 native cases with two controlled refresh bodies. Shared data initializer is reused; source integration connects root/selection/context receivers. Fourth child and root refreshes remain pending; no native combined-factory or visual parity claim.


2026-10-17: [fourth child](UI_DIALOG_CHILD_WEATHER.md) now has720 native comparisons with controlled providers; source integration connects all four adapters. Root refreshes and provider bodies still need recovery, followed by native full-factory and visual checks.


2026-10-17: [nested refresh](UI_NESTED_REFRESH.md) now has432 native cases and192 root-constructor compositions with engine0/fresh lists. Final root refresh141138cf0 and remaining leaf providers still need recovery; full factory and visual parity remain unverified.


2026-10-17: [final root refresh outer body](UI_ROOT_REFRESH.md) now has118 whole native comparisons with controlled leaf bodies. Composition with actual nested refresh/payload assignment and complete factory still needs verification; this adds no visual parity evidence.


2026-10-17: [whole fresh root construction](UI_ROOT_CONSTRUCT_CHAIN.md) now has118 native runs with both actual nested refreshes, final root refresh and34 reached inline payload assignments. Remaining providers and full factory/dialog/live GPU remain pending; estimates unchanged.
