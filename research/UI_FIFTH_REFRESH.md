# Fifth dialog refresh and whole-factory composition

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

ui_fifth_refresh ports outer141139460 through its normal return plus hash-clear
14062b900 dispatcher/bulk behavior.528 complete original calls match full0xb0
object/declared mapped memory/global/index/override snapshots, ordered provider
arguments, live operand reads and provider-written locals. Initial scratch is
captured; incidental native spills are excluded. The native hash-clear and bucket
fill1405dd530 execute originally. Sparse range-erase1406215a0 remains controlled,
so this is not proof of that callee's internal erase/ownership logic.

## Recovered caller behavior

The refresh clears bytes30/28, clears the hash table and resets the pointer-vector
end to its reloaded begin. Engine AL, active-global and shared resource-index/
resource presence gate collection initialization. Iterator indices are sign
extended from32 bits before adding5 and scaling by8; profiles include negative
index-1 into valid records. Direct append reads the live item pointer/value and
end, while full capacity requests the original growth helper. Iterator advance
can modify vector metadata before the next iteration.

Temporary tree cleanup terminates the last link, loads each next pointer before
a sized32-byte free, and reloads its head from the local for the final free.
The pointer-vector sort request uses begin/end and signed shifted length. A
separate AL predicate selects payload fetch/assignment followed by cleanup at
S+40 then S+28, or the fallback vector reset/mark loop. The mark loop caches its
begin/end but reads future item values live, writes each key into S+c0, performs
the controlled lookup and clears byte18 at the returned entry.

The fallback override reads live global142f069a0 and copies raw bits from142f069a4
to object+90; byte94 becomes1 only when previously0. Listener dirty18 brackets
notifications, with next and head reloaded after each callback and a final0 store.
The source image bridge preserves partial mapped state on errors; provider
identities are version-bound data, never executable host function pointers.

Hash clear returns immediately for zero count. A sparse table delegates the
original range-erase call with table/first/sentinel. The bulk path terminates the
last next link, follows cached next pointers while freeing32-byte nodes, reloads
the sentinel from the table after provider effects, restores its self links,
zeros count and fills valid bucket slots with that sentinel. Profiles include
sentinel replacement during free. This proof covers bounded mapped ranges;
source iteration guards do not claim original fatal/exception behavior on invalid
or nonterminating state.

## Standalone matrix

432 systematic cases cross engine/global/resource gates, payload branch,
collection sizes0/1/3, capacities0/1/4 and three hash profiles.96 seeded cases add
raw bool returns such as256 (AL0), shared resource indices includingu32::MAX, NaN/infinity/signed
zero override bits and mixed mutable effects. Object/allocator poison seeds vary.
Initial retained vector elements fit capacity and are cleared before appending.

The528 initial hash profiles are174 empty,175 bulk and179 sparse. Nonempty tables
contain one or three nodes and have8 or64 buckets. Sparse erase effects are
controlled. There are71 collection/sort branches,97 iterator advances,24 growth
requests,258 payload assignments,53 mark lookups,520 sized free calls across hash/
temporary cleanup and714 listener notifications. Empty/stable-three/mutated-stop-
after-one listener profiles are covered. Controlled free can overwrite the current
next pointer and change the final sentinel; iterator advance can append an extra
item; lookup can change stored vector begin and future values. The unchanged
native caller still determines which cached/live operands are used next.

Iterator records/tree fixtures and provider result arrays are synthetic scoped
inputs, not proof of actual registry/container production or complete sort/growth/
payload/lookup semantics. Sorting, allocator, free and provider calculations are
controlled; only their caller-visible order/state effects are established.

The payload-presence predicate14113b9a0 remains controlled here. Static inspection
shows another engine/global/resource/settings-mode gate inside it; its body has
not yet joined these native runs. The synthetic predicate result may vary
independently of the earlier collection gate, so these branch counts are not a
claim about real-engine scenario frequencies. Recover this gate against the same
shared settings record next.

## Whole factory with root/context/fifth bodies

118 original factory1410d6ca0 calls now contain all four constructors and inline
fifth/hash initialization, actual root141138cf0, both nested14113a300 calls,
payload/string/vector helpers, context1411396d0 and fifth141139460. Normal return
and bundle RAX are asserted. The fifth subtrace is captured during this single
original run, not obtained from an independent refresh invocation.

The source factory checks the fifth initial object against its own allocation,
checks inherited active-global/resource-index, replays the recovered body through
refresh_image and returns changed known factory-owned blocks/global/index. All
prior root/context traces, allocation/child-return/final bundle/declared heap
snapshots, receiver links/counts and callback peer checks remain active.649 retained
entries survive cleanup. Fresh fifth lists/tables imply no notifications or
nonempty hash clear in this composed corpus; those paths have standalone evidence.

There are34 fifth collection/sort branches,44 advances,22 growth requests,
59 payload branches,17 mark lookups and78 temporary free calls in the composition.
Dependent providers own their synthetic external buffers; this is not evidence
of actual engine allocation/lifetime or registry calculations. Other child refresh
and leaf providers retain their previously stated boundaries. The earlier118
factory/root/context C rows regenerate byte-for-byte unchanged after this extension.

## Reproduce and remaining work

```sh
python tools/gen_ui_fifth_refresh_vectors.py /path/to/X-Plane.exe > /tmp/fifth-refresh.txt
OPENXPLANE_UI_FIFTH_REFRESH_VECTORS=/tmp/fifth-refresh.txt cargo test --offline -p xp-app --test ui_fifth_refresh_local -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context --native-fifth > /tmp/factory-fifth.txt
OPENXPLANE_UI_FACTORY_FIFTH_CHAIN_VECTORS=/tmp/factory-fifth.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_root_context_and_fifth -- --ignored
```

Executable, original assets and generated corpora stay local. Only source,
generators and evidence reports are published. Next recover selection/weather
refreshes and the remaining range-erase/engine/config/record/registry/iterator/
growth/sort/payload/cleanup/notification/context/weather providers, then complete
full dialog/layout/widget bindings/singleton/Main Menu and teardown.

These modules still are not wired into original live menu rendering. Alt/Esc,
GPU pixels and paired screenshots/GIF/video remain unverified. Shares reviewed
and unchanged:41.4% implemented/28.2% verified overall; Rendering/UI verified0.

Validation:204 ordinary tests pass;62 opt-in tests are ignored in that run.
All528 standalone fifth/hash-clear cases and118 whole-factory fifth-chain cases
pass explicitly. The earlier118 factory/root/context replay still passes and its
C rows regenerate byte-for-byte unchanged. Formatting, Python syntax,
all-test-target compilation and diff checks pass. No live viewer/GPU run or paired
visual capture was performed.


2026-10-18: [actual payload gate and shared settings](UI_FIFTH_PAYLOAD_GATE.md) now has528 new whole-fifth and118 whole-factory native comparisons with14113b9a0 executing originally. Root/fifth settings identity and bytes are checked; original lookup/provider production and live GPU remain pending. Earlier opaque-predicate evidence retains its explicit scope. Estimates unchanged.
