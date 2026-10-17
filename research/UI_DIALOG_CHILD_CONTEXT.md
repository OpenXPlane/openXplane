# Child constructor1410d7490 and retained parent callbacks

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered construction

Outer dialog factory1410d6ca0 invokes1410d7490 for its third0x60-byte allocation,
with its second returned object as parent. The port composes raw initialization,
callback-container creation, peer registration, parent-list copying and temporary
teardown. Base table142642d00, empty120-byte list sentinel and flag18=0 precede
the callback container at+20. Derived tables are142642ff0/142643020. Defaults at
+38/+3c are dword80/float12; qwords+40/+48 are0, byte+50 is1 and+58 is parent.

After refresh1411396d0, the constructor reloads parent+58, creates a bound0/0
temporary against self+20, copies it into that parent's list via141149460, then
destroys the temporary. The retained copy remains registered against self+20.
Refresh replacements are retained. No semantic meaning is inferred for the raw
payload defaults or refresh output beyond these observed stores.

The source models the complete96-byte object and120-byte base sentinel as byte
images. Its fresh base receiver list is also registered for later child bindings;
the [second-to-third receiver test](UI_DIALOG_CHILD_SELECTION.md) uses this graph. Callback-tree backing remains opaque: head identity/count fields match,
while peer sets/list links use the recovered callback modules. An initialized
container is adopted without counting its sentinel allocation twice.
`Initialization` returns base-sentinel ownership and opaque callback-head identity;
it is not an executable native memory layout.

`construct_factory_child` adapts kind2 in ui_dialog_children, writes back the
96-byte object and returns internal ownership to its caller. An ordinary factory
test uses this adapter/constructor/receiver; other endpoints are controlled.
This is not a combined original whole-factory heap replay or live viewer test.

## Native evidence

The generator executes the complete original child through normal return plus
original callback ctor/bind/copy/list append/base teardown bodies unchanged.
Only refresh, allocator and sized free are controlled.108 two-child sequences
cover216 full child calls: three poison patterns, initial parent-list lengths
0/1/3 and second-list0/0/2, both parent arguments, both refreshed parents and raw
replacement mode bytes0/1/255. Refresh replaces32 payload bytes and may redirect
parent+58. Parents stay valid/stable; custom callables are null.

Every call matches all96 final object bytes, all120 base-sentinel bytes, two raw
allocation-entry snapshots and refresh-entry object bytes, parent next/previous/
count fields, all live callback parameters/ordered peers and allocation/free
request order. The temporary is absent from final live state. All216 pass.
Prior324 callback-link and72 callback-list sequences pass again. Ordinary tests
verify parent reload and real outer-factory kind2 dispatch.

No whole callback-tree/heap byte or free-pointer-order claim is made; internal
tree-store snapshots and actual refresh-body equivalence are unverified.
Reference addresses are metadata identities. Invalid/null parents, refresh
mutation of callback-container headers, allocation failure, nonnull custom
callables, full child/list teardown, reentrancy and concurrency remain unverified.

Corpus stays local at `/private/tmp/openxplane-ui-dialog-child-context.txt`;
replay uses `OPENXPLANE_UI_DIALOG_CHILD_CONTEXT_VECTORS`. Executable/assets and
corpus are not published; source/generator/test/report are published.

## Next

The [whole refresh caller](UI_CONTEXT_REFRESH.md) now has972 original cases and
ordinary constructor integration. A [972-case combined original replay](UI_CHILD_REFRESH_COMBINED.md)
now covers child+refresh+retained callback lifetime; providers, CRT parsing, time
conversion and notification effects remain controlled. Recover these remaining bodies plus
child constructors1410d6a40/1410d6fc0/1410d75a0 and resource/refresh bodies, then
compare combined original constructor/factory composition. Finish dialog140ec9c10, layout/widget bindings, Main Menu singleton
publication and construction. Opening ID3/Alt, original viewer/GPU output and
paired screenshots/GIF/video remain missing. Esc still exits the viewer.
Estimates stay41.4% implemented/28.2% verified; Rendering/UI verified0. Whole-game
parity remains unproven.


2026-10-18: [factory with actual root/context refreshes](UI_FACTORY_ROOT_CONTEXT_REFRESH.md) now has118 whole original calls with both refreshed children, shared resource index and full callback/list composition. Context reads88/77 after gating; leaf/CRT and remaining refreshes/providers/full dialog/live GPU remain pending. Estimates unchanged.
