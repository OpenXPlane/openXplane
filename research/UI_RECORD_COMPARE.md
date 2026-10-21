# Whole record equality and selection caller bindings

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Unwind ranges:141a3c910..141a3cbb4 (676 bytes), and141a3cbc0..141a3cbd0
(16 bytes). Completed original CPU calls and strict Rust replay establish the
normal-return caller contract below; no live simulator or GPU output is claimed.

## Equality is an ordered caller, not a whole-record byte comparison

The source `ui_record_compare` implements equality141a3c910 and its negated
wrapper141a3cbc0. The outer equality caller compares strings at offsets0,20,60,
then invokes derived-string141a3df30 once per record, compares those returned
strings, checks DWORD+a0, and compares strings at+a8,+80,+108 in that order.
Every failure short-circuits further comparisons. Other bytes are not directly
compared by this caller. This does not mean the real derived-string producer
ignores them: it reads record/tree state and its body remains controlled here.

Each string chooses inline or pointer storage using unsigned capacity>15 and
passes exactly its declared length to memcmp1423c0180. Zero length still requests
the comparison; embedded NUL does not shorten it. The right string is RCX and
the left is RDX. Only low32 of the provider result is tested: a nonzero full
RAX with zero EAX compares equal. Later fields reload their descriptors after
prior services; there is no initial bulk snapshot of the entire record.

The first derived return pointer is cached across the second request, but its
descriptor is read afterward. Returned pointers may alias caller storage or
either physical temporary; cleanup still refers to the two physical stack
objects, independently of those returned addresses. The stack flag word is
written0 then1; local cleanup state is not a later read of that word. The final
boolean is cached across cleanup and the actual original returns canonical
full RAX0/1, including through the wrapper's XOR AL,1.

## Reverse cleanup and allocation operands

The right physical temporary at SP+20 is cleaned first, then its length/capacity
are set to0/15 and byte0 is cleared, even for inline storage. The left physical
temporary at SP+40 is freed when needed but is not reset. Each capacity and data
pointer is read when that cleanup runs; a controlled first free can replace
the second descriptor. The source therefore does not cache both free operands
before calling either provider.

For capacity>15, byte count is capacity+1 wrapping64. Counts>=4096 load the
allocation base at data-8, add39 to the free byte count, and require
unsigned(data-base-8)<=31. Both valid alignment endpoints (data-base8/39),
capacity4094/4095 and UINT64_MAX wrap-to-zero counts are exercised. These are
synthetic mapped descriptors with a controlled free endpoint, not evidence
that impossible capacities satisfy the real allocator/ownership contract.
Invalid alignment returns an explicitly unverified-profile error in source;
native invalid-parameter termination, exceptions and unwind cleanup are not
reproduced or claimed by these normal-return cases.

## Production defaults and local scratch lifetime

`ui_selection_finish::Services::different` and
`ui_selection_initial::Services::different` now default to the recovered body.
The equality frame sits caller-SP-a0 through the negated wrapper. The bridge
shares live caller memory and restores it even when a dependency returns an
error. When child scratch is absent, it creates only the64-byte temporary region
and4-byte flag region and removes that newly created storage when the call ends.
Existing mapped child scratch is preserved. Partial overlapping declarations
are rejected as unverified rather than shadowing live caller storage.

Two ordinary tests call these actual defaults with no predeclared child frame:
they verify a derived-provider change to live caller memory survives while the
new scratch expires. This is a value/memory model, not a reproduction of Windows
register saves, arbitrary address-space faults, concurrency or stack contents.

The original-dependent caller replays explicitly distinguish `P` subrows from
legacy controlled `I` predicate events. `P` executes the recovered Rust child,
checks left/right/negation/frame relation and every live inherited region, and
publishes resulting bytes before the outer caller resumes. It does not replace
the predicate body with its recorded post-state. Controlled legacy corpora keep
their original scope and explicit service override.

## Direct and composed evidence

1500 completed original calls include750 direct equalities and750 negated
wrappers. All1500 actual equality entry/RET pairs and750 wrapper pairs match;
6246 memcmp,1458 derived-string and992 free requests match exactly at every
provider boundary and final declared-memory snapshot. Return counts are749
zero and751 one. Inputs cover each mismatch stage, length mismatch, zero length,
embedded NUL, non-UTF8 bytes, inline/heap storage, aliasing of returned descriptors,
low32 result masking, cached result across free and reloaded second cleanup.
The derived-string, memcmp and free bodies are controlled throughout this proof.

560 complete original finish callers now execute284 actual comparisons. Their
comparison scope reaches1508 memcmp,568 derived requests and452 frees, with124
false /160 true negated results. Source and original match all declared caller
and child memory/provider boundaries. Existing record initialization/copy/
assignment bodies are still controlled; the new mode supplies inline synthetic
records instead of the older poisoned descriptors. In-place shifts/growth remain
reached (175/63). Actual gate/query/primary/resource returns are156/129/105/417.
Raw scalar arithmetic retains the explicit Unicorn2.1.4 replay scope documented
in [UI_SELECTION_FINISH.md](UI_SELECTION_FINISH.md); this adds no independent
NaN/MXCSR proof.

712 complete original initial callbacks execute1143 actual comparisons, before
and after their controlled finish requests. These reach6045 memcmp,2286 derived
requests and1842 frees, with491 false /652 true negated results. The caller
reaches246 float requests,416 controlled finishes,14 cleanups,508 secondary
requests,314 controlled recursive resets and153 payload copies. Actual tag/
payload helper returns are143/22; other cases explicitly retain controlled
helper mode. The existing768 no-stub helper pairs also complete in generation.

This predicate is not yet bound into the actual recursive reset/finish/factory
cohort from [UI_SELECTION_FINISH_COMPOSED.md](UI_SELECTION_FINISH_COMPOSED.md).
That cohort retains controlled comparisons; neither its counts nor its proof
scope are relabelled here. Next: recover derived-string141a3df30 and its tree/
string dependencies, bind the predicate through those recursive/factory callers,
and recover the float/secondary/token/readiness/work/output/CRT/growth providers.
Real state producers, registry/ownership/lifetime, complete dialog/layout/menu,
Alt/Esc live integration, GPU and paired screenshots/GIF/video remain unverified.
Compatibility estimates are reviewed and unchanged.

## Reproduction and validation

Executable, resources and numeric corpora remain local:

```sh
set -e
python tools/gen_ui_record_compare_vectors.py /path/to/X-Plane.exe > /tmp/compare-new.txt
mv /tmp/compare-new.txt /tmp/compare.txt
OPENXPLANE_UI_RECORD_COMPARE_VECTORS=/tmp/compare.txt cargo test --offline --locked -p xp-app --test ui_record_compare_local -- --ignored
python tools/gen_ui_selection_finish_vectors.py /path/to/X-Plane.exe --native-compare > /tmp/finish-compare-new.txt
mv /tmp/finish-compare-new.txt /tmp/finish-compare.txt
OPENXPLANE_UI_SELECTION_FINISH_COMPARE_VECTORS=/tmp/finish-compare.txt cargo test --offline --locked -p xp-app --test ui_selection_finish_local whole_finish_with_actual_record_comparison_and_cleanup -- --ignored
python tools/gen_ui_selection_initial_vectors.py /path/to/X-Plane.exe --native-compare > /tmp/initial-compare-new.txt
mv /tmp/initial-compare-new.txt /tmp/initial-compare.txt
OPENXPLANE_UI_SELECTION_INITIAL_COMPARE_VECTORS=/tmp/initial-compare.txt cargo test --offline --locked -p xp-app --test ui_selection_initial_local whole_initial_with_actual_record_comparison_before_and_after_finish -- --ignored
```

All three new replays passed. Local ordinary xp-app tests:206 passed,94 local-
proof tests ignored by default. Workspace/all-targets Clippy with warnings denied
passed. Native corpora are absent from hosted CI, so its green checks cannot
substitute for these explicit local runs.

The prior560-finish and712-callback controlled-predicate replays also passed
explicitly after switching production defaults; their own scope is unchanged.

Subsequent class-string evidence: [UI_RECORD_CLASS_STRING.md](UI_RECORD_CLASS_STRING.md)
executes actual derived/label/reached-resize bodies in new equality/finish/callback
cohorts. Earlier controlled-derived and recursive/factory cohorts keep their scope.


## Subsequent actual-string composition

A separate1500-comparison cohort now executes1458 actual class-string bodies
and8932 actual append/direct-growth subrows; its4899 total growth returns include
nested append growth. Strict replay passes with all provider boundaries and
memory, together with an ordinary nonempty-tree production/ownership test.
[Actual-string equality evidence](UI_RECORD_COMPARE_STRINGS.md) describes the
729 preserved first outputs,720 distinct heap-result pairs and allocator-pool
lifetime/expanded-map publication. Earlier controlled-string cohorts retain
their original scope. Finish/callback/recursive-factory actual-string composition,
real CRT/producer/lifetime and all live Alt/Esc/GPU/pixel claims remain unverified.
