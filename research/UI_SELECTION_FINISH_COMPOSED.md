# Actual finish through recursive selection and the whole factory

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
The original CPU caller chain and Rust caller chain now both execute finish
14113d660, replacing the controlled finish endpoints only in the new mode.
No full simulator, GPU output or paired capture is claimed.

## Preserving live state across three scopes

`--native-finish` implies actual recursive reset, initial callback and all their
previously recovered helpers. The native finish scope installs after the reset/
callback scopes and handles its own shared leaf calls. Ancestor observers are
inactive until finish returns; a normal callback finish cannot complete a reset
as if it were a tail call. Per-body gate/query/primary/getter entry/RET counters
remain separate. The factory's selection-getter observer also excludes all busy
reset/finish scopes, avoiding duplicate constructor events.

Normal finish calls use finish-frame=callback-frame-650. Reset restores its
frame and tail-jumps, giving finish-frame=reset-frame-570. The tail's old scratch
is dead; the finish prologue can overwrite it before the child snapshot. The
replay therefore checks every live inherited input, excluding only that reset's
SP+20/b0 and SP+e0/8 dead ranges. It still publishes the child's resulting bytes
through those old declarations for the final raw reset snapshot. Normal callback
calls exclude no input regions. Frame relationships and owner arguments are
asserted before running the recovered child.

Overlapping inherited/child declarations are coalesced into physical unions,
so writes to reused stack bytes have one backing region. All live owner, parent,
vector, catalog, table, provider and ancestor regions are checked before child
execution. Raw prologue/saved-register bytes in expired stack storage are scoped
scratch inputs, not a proof that Rust implements the Windows register-save ABI.
The child's saved-register restoration and whole address space remain outside
the value/caller model.

Provider-private backing is initialized at the outer scenario boundary. Actual
finish entry preserves it and all inherited memory without reseeding or
reconstructing the owner, catalog or table. Each completed Rust child executes
its recovered body against explicit provider events; it does not substitute a
recorded final snapshot for that body. Only declared leaf effects retain their
controlled scope. Updated bytes return to each enclosing caller before its next
instruction; nested reset/callback scopes resume in LIFO order.

## A controlled fixture invariant exposed by composition

The first combined native run exhausted its instruction budget in the original
catalog loop. Existing controlled effects changed begin and end independently
and, unlike a standalone proof, a later finish consumed both. Combining pointers
from different catalog allocations made the equality-terminated loop unbounded.
This was a synthetic producer contract error, not evidence of a simulator bug.

In the new mode, a controlled reset catalog replacement publishes both begin
and end for its three mapped records. A finish shift/growth effect changes end
to the current begin, preserving the intended cached-end test while leaving a
valid empty range for a later caller. It changes the next mapped record hash too.
The loop still consumes its originally cached bounds and fresh record hashes.
No original caller branch is bypassed or supplied a terminating instruction.
Legacy modes retain their original producer contract and corpora. Arbitrary
inconsistent descriptors and native nontermination/fault parity are unverified.

Standalone inputs explicitly start with valid empty selection/output vectors;
factory inputs inherit the constructor's own initialized vectors. The composed
profiles do not inject the standalone finish harness's artificial prepopulated
vector. They exercise real append/reuse/growth paths;175 in-place shifts remain
covered separately by the560 direct finish cohort.

## Recursive reset and callback proof

528 outer original resets execute55 actual nested resets:583 reset callers and
583 actual initial callbacks, with maximum observed reset depth2 and one
suspended callback. All627 finish callers now execute:583 tail calls plus44
normal callback calls. The controlled finish and recursive-reset counters stay
zero. Source and original match all declared bytes, provider boundaries and
normal outer return.

Finish reaches350 ready /277 inactive branches,317 record assignments,
567 direct appends,264 growth requests,417 work/output paths and1321 record
destructors. Actual finish-scope gate/query/primary/resource getter returns are
312/185/154/778. Reset-scope returns are583 gates,636 queries,432 tag queries and
92 primary/resource getter calls. Reset reaches419 assignments,253 fallbacks,
254 optional cleanups,496 frees and842 notifications. Different counts from the
old controlled-finish corpus reflect real intervening finish state changes.

Initial helper returns are56 tag /43 payload;25 float requests and43 payload
copies occur. Real finish may change the owner float/record/vector state before
the callback resumes. The old artificial dirty/pending finish effects are gone.
Observed depth2 does not establish arbitrary callback termination or thread
safety. The16-level harness limit remains an unsupported-profile guard.

## Whole factory and constructor binding

118 complete original factories retain all four actual constructors and prior
root/nested/context/weather/fifth refresh chains. They execute11 nested resets,
129 reset/initial bodies and351 actual finish bodies:104 callback +129 tail
+118 constructor calls. All590 retained list entries, callback graph, current
owner/parent links, sentinels and20 inherited table regions match.

A fallible `finish_checked` seam now carries current image, both sentinels and
callback context. Its legacy default calls the old service. The explicitly
bound factory replay retains live non-stack memory from the actual reset,
validates current callback/list publication, runs recovered finish and publishes
image/sentinel/resource results. Constructor finish does not consume a fresh
parent template. Local harness stack regions expire when reset returns and are
not presented as live constructor memory.

Finish reaches199 ready /152 inactive branches,169 assignments,397 direct
appends,59 growth requests,232 work/output paths and735 record destructors.
Its actual gate/query/primary/resource returns are175/175/102/452. Reset-scope
query/gate/primary/resource/tag counts remain258/129/75/75/75; initial helpers
return129/57. Reset reaches94 assignments,55 fallbacks,9 cleanups and112 frees.
Factory notification lists are fresh/empty. Controlled factory effects do not
mutate prior parents or table descriptors; this cohort's preservation checks do
not establish general registry mutation/ownership behaviour.

## Arithmetic and remaining provider limits

Every child finish row explicitly declares **U214**. The common scope refuses
an unaudited Unicorn version. Whole-chain source replay applies the same
**test-only** both-NaN propagation override documented in
[UI_SELECTION_FINISH.md](UI_SELECTION_FINISH.md). Production scalar values retain
the separately tested SSE policy, verified under Rosetta and native x86_64
Linux/Windows CI; whole-chain NaN arithmetic is not independently re-proved by
these raw Unicorn results. Flags, non-default MXCSR and broader NaN-sensitive
oracles are unverified. Patches to the read-only .rdata divisor remain synthetic
inputs, not actual simulator producer behaviour.

Record/token/readiness/predicate/float/work/scalar-output/notification/CRT/growth
and other leaf bodies remain controlled as documented by each caller report.
Actual record initializer141a3c750 is not yet bound in this finish composition.
Complete dialog/layout/widgets/Main Menu, registry/state producers, callback RB
backing, original Alt/Esc/viewer/GPU and matched screenshot/GIF/video acceptance
remain unfinished. Compatibility estimates were reviewed and remain unchanged.

## Reproduction and regression scope

Numeric corpora stay local; the executable and aircraft/resources are not
published. In the audited Unicorn2.1.4 environment:

```sh
set -e
python tools/gen_ui_selection_reset_vectors.py /path/to/X-Plane.exe --native-finish > /tmp/reset-finish-new.txt
mv /tmp/reset-finish-new.txt /tmp/reset-finish.txt
OPENXPLANE_UI_SELECTION_RESET_FINISH_VECTORS=/tmp/reset-finish.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_local whole_recursive_reset_callback_and_real_finish_publish_inherited_state -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-finish > /tmp/factory-finish-new.txt
mv /tmp/factory-finish-new.txt /tmp/factory-finish.txt
OPENXPLANE_UI_FACTORY_SELECTION_FINISH_VECTORS=/tmp/factory-finish.txt cargo test --offline --locked -p xp-app --test ui_factory_composed_local native_factory_with_actual_recursive_reset_callback_finish_and_all_refresh_chains -- --ignored
```

Both composed replays passed explicitly. The large corpora stream one row at a
time instead of loading the whole file. Regenerated legacy recursive reset rows
are byte-identical (528 rows; row SHA256
`c0e3972dff35ef97e81ff710187603b0cf115bf9593c8d31e3af00e593b4b193`).
The older controlled-finish factory proof retains its separate118-case scope.

Local validation:204 ordinary xp-app tests passed,91 local-proof tests ignored
by default. Both new full-chain replays and the prior118-case controlled-finish
recursive factory replay passed explicitly. Workspace/all-targets Clippy with
warnings denied passed; formatting and diff whitespace checks passed. These
local original-dependent corpora are not present in hosted CI.

Subsequent predicate evidence: [UI_RECORD_COMPARE.md](UI_RECORD_COMPARE.md)
ports equality/negation and binds it in new direct finish/callback cohorts.
Earlier controlled comparisons, including recursive/factory cohorts, retain
their explicitly separate scope.
