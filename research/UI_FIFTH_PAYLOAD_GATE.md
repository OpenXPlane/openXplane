# Actual fifth payload gate and shared settings record

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

ui_fifth_refresh::payload_available recovers complete14113b9a0. The normal
Services implementation now executes it during fifth refresh141139460. The old
opaque predicate remains an explicit adapter seam for earlier caller-only
comparisons; it is not used in the new actual-predicate replays.

## Original branch and state reloads

At14113b9ab the helper calls engine1417fb320 with global142f2d770. AL0 or nonzero
active-global145a10798 returns false. Otherwise it reads index142640504 at
14113b9be and calls resource141a64260(global146124398,index). Zero64-bit resource
presence at+6188 returns false. It then reloads index at14113b9da, calls settings
141a64250(global1461243b0,index), and returns true only for the full32-bit zero
mode at settings+dac. Incoming register arguments are not used by this helper. The result is checked
at the caller's AL width; unspecified upper RAX bits are outside this proof.
Normal bounded mapped profiles apply; native error/fatal/exception behavior is
not established.

The port reads active/index after provider effects and independently reloads index
for settings. Controlled resource lookup can increment index, includingu32::MAX
wrapping to0. A cached pre-resource index would fail the next recorded argument.
Standalone profiles also vary the second engine/global/index result independently
of the earlier collection gate, with AL0 for raw return256 and full-width unsigned
indices/nonzero modes. Override bit/global effects remain controlled engine inputs.

## Separate fifth-refresh composition

528 complete original fifth calls now execute actual14113b9a0 as well as the
previous outer refresh and hash-clear dispatcher/bulk/bucket-fill bodies. Their
native harness asserts no predicate stub is installed. Source tests use a wrapper
that inherits the production default gate; only leaf invoke is forwarded to the
strict replay. A generator accidentally replacing the predicate with the old
opaque event would fail the first expected engine call, rather than silently
fall back to the old mode.

All earlier raw object/declared memory/global/index/override snapshots, provider
arguments/live reads/local writes, cleanup and notification checks remain active.
There are1056 engine calls (two per refresh),297 resource lookups,82 settings
lookups and43 reached payload branches. Settings arguments include0/1/77/78/88/89/
u32::MAX. The same synthetic matrix retains71 collection/sort branches,24 growth
requests,520 cleanup free calls,91 mark lookups and714 notifications. Settings
records have zero or nonzero32-bit modes, including all-one bits. This is a
controlled joint matrix, not evidence of real runtime occurrence frequencies.

## Whole factory and identity across children

118 normal original factory1410d6ca0 calls contain all four constructors, inline
fifth/hash initialization, actual root/nested/payload/context/fifth refreshes and
this predicate. No independent original refresh invocation generates its subtrace.
The fifth provider is given the root harness's existing settings record; its mode
is preserved instead of initializing another synthetic settings object.

The source caller retains the known root settings word by address/bytes, checks
root getter return identities when reached, compares that word in the fifth's
initial mapped inputs and checks the fifth getter's returned record identity.
Thus the test verifies physical record/word continuity, not merely equal mode
values in two unrelated buffers. Global/index propagation and all649 retained
callback entries plus prior allocation/child-return/final heap/bundle/graph checks
remain active. Native settings lookup itself stays controlled and may alias the
same record for multiple indices; actual registry mapping/production is not proved.

There are236 fifth engine calls,90 fifth resource lookups,34 settings lookups and
17 fifth payload branches in this composition. Settings calls use88 or89 after
controlled index mutations. Root's shared mode0/7 decides the final result instead
of the earlier independently supplied opaque predicate boolean. Other controlled
leaf effects and successful fresh-list assumptions retain their prior boundaries.

## Legacy evidence, reproduction and remaining work

The earlier528 whole-fifth and118 factory/fifth C rows regenerate byte-for-byte
unchanged. Their Rust replay explicitly overrides the controlled predicate
endpoint and continues to prove only its caller; the new modes require actual
gate execution. New and old tests do not infer their scope from the trace contents.

```sh
python tools/gen_ui_fifth_refresh_vectors.py /path/to/X-Plane.exe --native-predicate > /tmp/fifth-gate.txt
OPENXPLANE_UI_FIFTH_GATE_VECTORS=/tmp/fifth-gate.txt cargo test --offline -p xp-app --test ui_fifth_refresh_local native_fifth_refresh_with_actual -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context --native-predicate > /tmp/factory-shared-gate.txt
OPENXPLANE_UI_FACTORY_SHARED_GATE_VECTORS=/tmp/factory-shared-gate.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_fifth_gate_and_shared -- --ignored
```

Executable, original assets and generated corpora stay local. Only source,
generators and evidence reports are published. Engine/resource/settings providers
remain controlled, as do sparse erase, iterator/growth/sort/payload/cleanup/
notification and other previously listed effects. Recover these lookups and
remaining selection/weather refreshes next, then complete registry/runtime
production, full dialog/layout/widget bindings/singleton/Main Menu and teardown.

Original live Alt/Esc, GPU pixels and paired screenshots/GIF/video remain
unverified. Shares reviewed and unchanged:41.4% implemented/28.2% verified overall;
Rendering/UI verified0. No claim of complete engine or visual parity is made.

Validation:204 ordinary tests pass;64 opt-in tests are ignored in that run.
Both new528-case actual-gate and118-case shared-settings factory replays pass,
including inherited production gate and getter identity checks. Earlier528
controlled-predicate and118 factory/fifth replays still pass, and their C rows
regenerate unchanged. Both new corpora regenerate identically. Formatting,
Python syntax, all-test-target compilation and diff checks pass. No live
viewer/GPU run or paired visual capture was performed.


2026-10-18: [actual indexed record access](UI_INDEXED_RECORDS.md) now has872 direct original-code cases,528 fifth compositions and118 whole-factory runs with original fifth getter bodies plus constructor index20 lookup. Root/selection/context getters and actual registry production remain controlled; no live GPU/visual parity claim. Earlier modes retain their explicit scope; estimates unchanged.
