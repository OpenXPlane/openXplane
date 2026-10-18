# Real indexed records through the root and nested UI chain

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Root141138cf0 and both nested14113a300 invocations in constructor1410d6a40
now use the recovered141a64260/141a64250 table address calculations. Root
Services exposes default resource_record/settings_record methods over mapped
logical memory. ui_refresh_chain synchronizes its nested state and routes those
requests through the typed methods, preserving partial-state/error handling.
Other provider requests still go through invoke. No host pointer is executed.

Legacy Replay explicitly supplies controlled getters. New IndexedReplay inherits
both production default lookups and retains itself across recursive nested
refreshes, rather than accidentally dropping back into the legacy adapter.
Constructor allocation snapshots and callback/context checks remain unchanged.

## Original execution scope

The explicit --native-root-tables mode implies actual fifth query/predicate/table
mode.118 complete original factory1410d6ca0 calls execute all four constructors,
root141138cf0, both nested14113a300 bodies, payload/string/vector helpers,
context1411396d0, fifth141139460/hash clear and its availability predicate.
Resource/settings getter dispatch now also yields to original machine code
throughout the root constructor. Each real entry/RET is counted, keyed by function
and low32 index, with per-constructor equality asserted. The outer factory wrapper
now propagates the nested dispatcher return value, so yielding to original getter
code is preserved through both wrappers.

Original root/nested resource returns:90 at index5,68 at77 and68 at88 (226 total).
Settings returns:46 at88. No resource/settings I events substitute for those
calls in the root row; IndexedReplay uses the production lookup implementations.
The original fifth query returns236 times, its resource getter90 and settings
getter23 times; constructor resource index20 executes118 actual returns.
Whole factory normal return, heap/bundle/child snapshots and649 retained list
entries/logical callback state remain checked.

## Distinct valid string inputs and shared memory

Root consumes a32-byte resource string at record+6178, whose length at+6188 also
controls presence. An8-byte presence-only mapping cannot prove that caller.
The new fixture maps each complete32-byte string, its matching settings+dac word,
and both8-byte table descriptors. Nine signed-index cells are supplied:
-1/0/1/5/20/77/78/88/89, using the original6668/433c0 strides in hex.
The synthetic allocation reserves a mapped cell before index0 for the tested-1;
it does not prove a real engine sentinel policy.

Record88 preserves the existing inline/heap/4096-boundary aligned source-string
profiles and mode word. Other records contain distinct valid inline strings
record<index>, capacity15, and either their own text length or zero depending on
case/index. Adjacent settings modes differ. This models supplied resource inputs,
not registry construction, ownership or real engine string occurrence frequencies.

The root replay contains all input fields. The factory source adapter extracts
all nine complete strings, all nine settings words and both descriptors from the
returned root runtime (20 mapped regions). It checks them against the original
fifth initial snapshot together with inherited global/index and fifth object
bytes. The fifth harness retains complete strings in this mode, without overlapping
presence-only mappings. Original and source lookups select actual indexed cells;
they do not return one aliased record for every index.

This changes the composed path: the root ends at index77 in95 cases and88 in23,
with22 reached inline payload assignments across the root/nested chain. The fifth
then reaches23 collection/sort calls,11 payload assignments and12 mark lookups.
These counts describe the synthetic profiles only. The smaller branch counts
relative to the earlier controlled-root cohort are a consequence of the different
per-index fields and the resulting index flow, not a reduction of assertions.

Selection/context getter bodies still use controlled providers returning the
anchor record88. Their source adapters explicitly read that fixture's presence;
this is not evidence of actual index selection in those callers. Root/nested/
selection/context engine query effects remain controlled. Only the fifth query
body is actual in this cohort, with its byte supplied at fifth entry. Registry
loading, byte producers/lifetimes, full string ownership, remaining provider
computations, selection/weather refresh bodies, callback RB backing and complete
dialog/layout/widget/singleton/menu/viewer behavior remain unverified.

## Reproduction and checks

```sh
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context --native-root-tables > /tmp/factory-root-tables.txt
OPENXPLANE_UI_FACTORY_ROOT_TABLES_VECTORS=/tmp/factory-root-tables.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_real_root_nested_and_fifth_record_tables -- --ignored
```

All118 new cases pass.204 ordinary xp-app tests pass with71 opt-in tests ignored;
workspace/all-target Clippy passes locally with -D warnings. The three prior118-
case root refresh/chain/constructor replays pass. The previous118-case actual-fifth-query factory replay also passes, and its
corpus regenerates118 C rows byte-for-byte unchanged. Python AST,
formatting and diff checks pass. Corpora/executable/assets stay local; only source,
generators and reports are published.

No live viewer/GPU run, original Alt/Esc menu or paired screenshots/GIF/video were
produced or verified. Estimates reviewed and unchanged:41.4% implemented/28.2%
verified overall, Rendering/UI verified0. Next connect actual indexed resource
lookups to selection/context, recover real query/registry production and remaining
child/provider bodies, then complete the dialog and live menu/rendering path.
