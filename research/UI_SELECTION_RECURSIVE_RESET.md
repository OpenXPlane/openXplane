# Actual recursive selection reset and signed catalog arithmetic

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Evidence is completed original-code CPU emulation and strict source replay on
explicit synthetic state. No live simulator or GPU output is claimed.

## Removing the recursive-reset endpoint

The new mode actually executes reset14113a460 when callback1410fe0f0 requests
reset(owner,1). That reset executes the actual callback again, its gate/query/
primary/tag/getter bodies and the complete remaining reset caller. The source
replay likewise calls the recovered Rust reset recursively, rather than applying
a recorded post-reset snapshot. The callback source is unchanged: its service
request now binds to the recovered body in this mode.

Native reset contexts and suspended callback contexts are kept in LIFO order.
The recursive body inherits **all current caller regions**, global descriptors,
query byte, object/sentinel/list state and provider storage. Its preparation
does not poison, reseed or rebuild those inputs. Allocation regions are inherited
as physical declarations, not inferred from potentially inconsistent begin/end
metadata. Inner completion restores the outer instrumentation context while
publishing resulting memory through every original caller region. Callee-only
scratch expires; globals and aliased storage remain live. All suspended callback
contexts must be gone when the outer reset completes.

The source callback replay explicitly binds a recursive handler for `R` subrows.
Each contains the actual nested reset trace as hex-encoded text, avoiding nested
separator collisions. Owner and argument1, cleared pending byte and stack
relationships are asserted: callback-frame=reset-frame-a0, recursive reset-frame=
callback-frame-e0. The nested row must contain its own actual callback subrow.
Every caller input region is checked before executing the child, and every
caller region receives its resulting bytes afterward. The source callback then
returns through its remaining code; the enclosing reset continues with live
state. Reset's unspecified RAX is not part of this void-call contract.

This corrects the earlier **controlled recursive endpoint** scope without
relabeling its old corpus. In recursive mode the initial harness forwards the
request into the original body; its controlled reset counter stays zero. Normal
outer return, per-body real query/getter entry/RET counts and full source traces
are required. The existing RIP-at-entry check still excludes endpoints actually
returned by another controlled scope in legacy modes.

## Why catalog arithmetic had to change

With a real nested body, existing controlled visitor/cleanup effects can change
the catalog base before the outer reset computes its count. The prior source
rejected negative, empty, unaligned and too-large distances as outside its old
three-record profile. The original has no such validation. A strict new replay
failed at that source guard; input states were kept and the port was corrected.

The original SUB wraps64 bits, the signed magic division by168 truncates toward
zero, and its comparison consumes low32 of quotient-1 as signed i32:

```
distance = (end - begin) wrapping64, interpreted as i64
last = low32(trunc_toward_zero(distance / 168) - 1), interpreted as i32
if parsed_i32 < 0 or parsed_i32 > last: use index0 and first-record name
```

This is now the source calculation. It does not validate vector ownership or
prove that inconsistent descriptors are valid whole-game state. Supplied begin
records/names still have mapped backing; end is used as arithmetic data.

504 complete original reset calls independently cover21 signed distances,
indices -1/0/1/2, option/default strings and three string aliases. Distances
include negative/zero/partial records, +/-168 boundaries, i64 MIN/MAX, and count
boundaries around2^31/2^32. These test truncation and signed low32 last-index
wrap, not merely nonnegative division. Native pointer operands, fallback/name
requests and every declared byte/provider boundary match. Mapping errors and
unbounded native fault behaviour remain outside the proof.

## Direct recursive composition

528 outer original calls now execute55 real nested reset bodies:583 complete
reset callers and583 actual initial callbacks. Maximum observed reset depth is2
with one suspended callback. Initial callbacks reach44 controlled finish
requests and43 payload copies; no recursive-reset stub runs. The reset bodies
reach419 assignments,270 fallbacks,254 temporary cleanups,496 frees and842
notifications, plus583 controlled tail-finish requests. Source and original
normal return, every provider operand/result and all declared caller/child
memory match, including listener mutations and catalog-base changes.

These cases preserve the earlier stress effects rather than disabling them to
avoid reentry. The native harness rejects profiles exceeding16 nested resets as
unverified; this is a verification guard, not a simulator termination rule.
Observed depth2 does not establish termination or parity for arbitrary callbacks,
threads, real finish effects or resource producers.

## Whole factory composition

118 original whole factories execute11 real nested reset bodies:129 actual
reset/initial callers while all four constructors and prior refresh chains
remain actual. Source input checking still includes current logical callback/
list publication, prior parent memory, both sentinels,20 inherited table regions,
globals/index/catalog and590 retained entries. No parent template replaces live
state on reentry.

Actual query/gate/primary/getter/tag returns are258/129/75/75/75, with per-body
entry/RET agreement. Initial helper returns are129/57, its finish requests104,
and controlled recursive-reset requests0. Reset reaches94 assignments,55
fallbacks,9 optional cleanups and112 frees. Selection notification lists remain
fresh/empty in these factories. There are104 initial +129 tail +118 constructor
finish requests=351, whose bodies are still controlled. Earlier329 finish
requests belonged to the controlled-recursion cohort and retain that scope.

Actual nested reset now leaves the pending byte as produced by that body, rather
than the earlier stub's artificial7e effect. Native/source traces compare the
result before resuming each enclosing caller. Finish, record/float predicates,
config/string/variant/cleanup/free/notification providers retain their documented
controlled scope; real finish may change pending state and reentry behaviour.

## Reproduction, regressions and limits

Raw numeric corpus files remain local:

```sh
set -e
python tools/gen_ui_selection_reset_vectors.py /path/to/X-Plane.exe --native-recursive > /tmp/recursive-new.txt
mv /tmp/recursive-new.txt /tmp/recursive.txt
OPENXPLANE_UI_SELECTION_RESET_RECURSIVE_VECTORS=/tmp/recursive.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_local whole_reset_callback_and_actual_nested_reset_restore_caller_frames -- --ignored
python tools/gen_ui_selection_reset_vectors.py /path/to/X-Plane.exe --catalog-boundaries > /tmp/catalog-new.txt
mv /tmp/catalog-new.txt /tmp/catalog.txt
OPENXPLANE_UI_SELECTION_RESET_CATALOG_VECTORS=/tmp/catalog.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_local original_catalog_signed_division_low32_last_index_and_fallback -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-recursive > /tmp/factory-recursive-new.txt
mv /tmp/factory-recursive-new.txt /tmp/factory-recursive.txt
OPENXPLANE_UI_FACTORY_SELECTION_RECURSIVE_VECTORS=/tmp/factory-recursive.txt cargo test --offline --locked -p xp-app --test ui_factory_composed_local native_factory_with_actual_recursive_selection_reset_and_all_refresh_chains -- --ignored
```

Recursive mode implies actual-initial/reset and all preceding composed modes.
Old528 initial/reset and118 initial/factory files regenerate byte-for-byte;
legacy source replays pass. New recursive files also regenerate byte-for-byte
with assertions that every suspended context unwinds completely.

Full finish/predicate/float/cleanup bodies, real registry/query/scratch production
and ownership/lifetime, callback RB backing, full dialog/widgets/Main Menu,
original Alt/Esc/viewer, GPU and paired screenshot/GIF/video acceptance remain
unfinished. These are caller/CPU proofs, not whole-game or visual parity.
Compatibility estimates are reviewed and unchanged.

Validation:204 ordinary xp-app tests passed,87 local-proof tests ignored by
default.528 recursive outer calls/55 nested bodies,118 factories/11 nested
bodies and504 catalog boundary cases passed explicitly. All four reset replay
modes and the prior controlled-recursion factory replay passed. Workspace/all-
targets Clippy with warnings denied passed.

Subsequent direct finish caller evidence is documented in
[UI_SELECTION_FINISH.md](UI_SELECTION_FINISH.md). That proof does not replace
the controlled finish endpoints in these recursive/factory cohorts.

Subsequent composition: [UI_SELECTION_FINISH_COMPOSED.md](UI_SELECTION_FINISH_COMPOSED.md)
executes actual finish through recursive reset/callback and whole factories in
a new explicit mode. Earlier controlled endpoint cohorts keep their own scope.
