# Whole selection initial callback and reset/factory composition

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Evidence is disassembly and completed original-code CPU emulation with declared
synthetic inputs and controlled dependencies. There is no live UI/GPU capture.

## Complete1410fe0f0 caller

This is the controlled-recursion checkpoint. The subsequent
[actual recursive-reset proof](UI_SELECTION_RECURSIVE_RESET.md) executes those
requested bodies while keeping the remaining provider limits explicit.

The callback receiver is selection owner+38. It tests the full64-bit key against
0x1e1b9; every other key returns without reading object state or allocating its
frame. The third incoming integer argument is not used. The source returns a
void result with mapping errors, not a reconstruction of unspecified native RAX.

With a non-null parent at receiver+90 it requests141a3cbc0(receiver+f8,parent+20).
A nonzero low AL takes the update path. Otherwise it reloads the parent, requests
1411363f0(parent+268,frame+a8), and compares the owner's live float at
receiver+290 with the float through the returned address. Unordered UCOMISS
also updates; signed zero compares equal. Operand bytes, pointer aliases and
post-provider field changes are checked; FP exception status is outside scope.

An update copies begin at receiver+d0 into end at+d8 when different, then calls
finish14113d660(owner). The store precedes finish. Parent is reloaded again for
the second phase, including after finish can change it. A second141a3cbc0 request
and tag query1417fb370 control access through1417f8940(receiver+18).

The caller caches two16-byte words and a trailing qword in nonvolatile XMM
registers. It stages the third word in local scratch, writes its first8 bytes
zero and its next4 bytes integer2, and forces the tail's low byte to1. Dispatch
uses the live signed variant tag at receiver+80. Tag3 stores directly; other
valid tags -1/0/1/2/4 store and change the tag to3, with optional cleanup only
for old tag0 and flag+78 set. The third word is reloaded from scratch **after**
cleanup, while the first/second/tail retain their cached bytes. This distinction
is verified with aliasing and cleanup writes to both the owner and scratch.

Finally it reads the live pending byte at receiver+88. If nonzero, it calls
secondary14113ff10 with a fresh parent pointer; nonzero AL then clears owner+c0
**before** requesting reset14113a460(owner,1). There is no extra clear after that
request returns. These recursive reset requests are controlled dependencies in
the present proofs; they are not complete executions of the recursive body.

## Two helper contracts and reachable scope

Complete1417fb370 returns AL=(byte[receiver+68]==3), preserving upper RAX bits
outside its AL contract. Complete1417f8940 returns full RAX=receiver for tag3,
otherwise zero. Both are read-only. Source Services defaults use these actual
mapped helpers. With these defaults in single-threaded execution, the payload
getter returns its receiver and old tag remains3 until assignment; thus the
callback's generic old-tag cleanup branch is not reached. This is a bounded
reachability result for these actual helper bodies, not a concurrent-state claim.

The direct corpus explicitly marks a second mode replacing these helpers with
controlled endpoints. That mode exercises the generic caller dispatch, external/
self/partially-overlapping payload aliases, and cleanup effects under all valid
old tags. It does not claim such helper mutations occur in the actual functions.
Invalid jump tags and native fault/ownership behaviour are not established.

## Direct and composed original-code comparisons

712 completed original callback calls cover both helper modes, full-width key
mismatches, parent/null-parent and live rebind paths, raw low-byte predicate
returns, float +/-0, finite equality/difference, infinities and quiet/signalling
NaNs. There are572 first/571 second predicate requests,241 float requests,
442 finish requests,197 payload copies,10 cleanups,485 secondary requests and
291 controlled recursive-reset requests. Actual helpers return133/60 times;
per-case entry and real-RET counters agree. Every dependency operand/result,
declared input/output byte and main-owned scratch region matches the port.
Scratch maps are frame+40..57 and frame+a8..ab; saved registers/FP flags are not
included. Other bytes outside declared maps are not a whole-heap claim.

Another768 no-stub helper pairs exercise all256 tags with three poisoned
receiver images:1536 completed original helper calls. The predicate's AL and
accessor's full RAX match, and every receiver byte remains unchanged.

528 completed actual reset14113a460 calls now include the actual callback and
its actual helpers. They reach38 additional finish and55 controlled recursive-
reset requests inside the callback, plus37 payload copies. The outer reset
performs326 assignments,200 fallbacks,165 temporary cleanups,385 frees and816
notifications, then528 tail-finish requests. Old controlled-initial reset scope
remains a distinct corpus rather than being silently relabeled.

118 whole original factories likewise include both actual callback and reset,
all four constructors and the previously recovered refresh chains. Initial
callback has118 predicate pairs,59 float requests,93 finish requests,51 payload
copies and23 secondary requests, of which11 request controlled recursive reset.
Its helpers return118/51 times. No callback cleanup is reached with real helpers.
Outer reset reaches91 assignments/52 fallbacks/8 cleanups/107 frees; actual
query/gate/primary/getter/tag returns are236/118/64/64/64. Finish requests are
93 initial +118 outer-tail +118 explicit-constructor =329; all finish bodies
remain controlled. The11 recursive reset requests do not execute their bodies
or add their eventual finish effects in this proof.

All current parent/list state, both sentinels,20 inherited table regions,
globals/index/catalog and590 retained callback entries remain checked. Nested
callback rows use a separate escaping layer inside reset subrows. Native callback
stack must equal reset-frame-a0. Callback-only scratch/auxiliary maps expire at
return; every original caller map receives its resulting bytes. Generic Services
retains the legacy initial-callback boundary by default; the recovered factory/
reset adapters explicitly select and execute the source callback body.

A strict stack-map mismatch exposed an instrumentation bug: Unicorn still
notified the reset entry hook after another scope returned a controlled recursive
request. The hook now checks that RIP actually equals that entry before preparing
its frame. Full comparisons passed after this fix. Old528 reset and118 reset-
factory corpus files regenerate byte-for-byte with the new mode disabled.

## Correction of the previous reset-only report

Direct inspection of both factory corpora shows reset-entry owner+c0=0 in95
cases and1 in23, not zero in all cases as the earlier report said. The earlier
code/state comparison was valid; the explanation of absent early-skip coverage
was wrong. No early skip is reached in that cohort, but it must not be attributed
to an always-zero byte. Its fresh notification lists remain empty. The report
and compatibility text are corrected from these snapshots; estimates stay fixed.

## Reproduction and remaining work

Raw corpora are local and are not shipped:

```sh
set -e
python tools/gen_ui_selection_initial_vectors.py /path/to/X-Plane.exe > /tmp/initial-new.txt
mv /tmp/initial-new.txt /tmp/initial.txt
OPENXPLANE_UI_SELECTION_INITIAL_VECTORS=/tmp/initial.txt cargo test --offline --locked -p xp-app --test ui_selection_initial_local -- --ignored
python tools/gen_ui_selection_reset_vectors.py /path/to/X-Plane.exe --native-initial > /tmp/reset-initial-new.txt
mv /tmp/reset-initial-new.txt /tmp/reset-initial.txt
OPENXPLANE_UI_SELECTION_RESET_INITIAL_VECTORS=/tmp/reset-initial.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_local whole_reset_with_actual_initial_callback_and_read_only_helpers -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-initial > /tmp/factory-initial-new.txt
mv /tmp/factory-initial-new.txt /tmp/factory-initial.txt
OPENXPLANE_UI_FACTORY_SELECTION_INITIAL_VECTORS=/tmp/factory-initial.txt cargo test --offline --locked -p xp-app --test ui_factory_composed_local native_factory_with_actual_initial_callback_reset_and_all_refresh_chains -- --ignored
```

Real predicates141a3cbc0/14113ff10, float helper1411363f0, finish14113d660,
cleanup and the recursively requested reset bodies are still controlled here.
Some synthetic null-parent inputs only verify caller forwarding to controlled
services, not legal whole-simulator state or real-callee fault behaviour. Real
registry/query/scratch producers and lifetime, RB backing, complete dialog/
widgets/Main Menu, original Alt/Esc/viewer, GPU and paired screenshot/GIF/video
acceptance remain unfinished. These CPU caller proofs do not establish UI or
whole-game parity. Weighted estimates are reviewed and unchanged.

Validation:204 ordinary xp-app tests passed,84 opt-in local-proof tests ignored
by default.712 callback calls plus768 helper pairs,528 callback/reset calls and
118 initial/reset factories passed explicitly. Legacy528 reset and118 reset-
factory replays also passed; their regenerated corpus files are byte-identical.
Workspace/all-targets Clippy with warnings denied passed.

Subsequent predicate evidence: [UI_RECORD_COMPARE.md](UI_RECORD_COMPARE.md)
ports equality/negation and binds it in new direct finish/callback cohorts.
Earlier controlled comparisons, including recursive/factory cohorts, retain
their explicitly separate scope.
