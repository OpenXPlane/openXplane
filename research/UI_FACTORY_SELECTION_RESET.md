# Whole child factory with actual selection reset

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
This is completed original-code CPU emulation with synthetic state and controlled
provider bodies, not a live simulator/UI/GPU capture.

## Composition

118 complete original1410d6ca0 calls now execute all four child constructors,
root/nested/payload/context/weather/fifth refreshes, both selection exits
(alternate14111bb90 or changed14113ba10), and the whole reset14113a460 with
actual gate14114d330, query1417fb320, tag1417fb390, primary14113fff0 and resource
getter141a64260. Reset reaches the original constructor's continuation through
its tail-finish service, and the factory reaches its normal return. No early
cutoff at the reset tail jump stands in for completed execution.

The kind1 host adapter invokes the recovered source reset through
`Services::reset_checked`. This boundary now supplies the owned base/record
sentinels and current callback/list context. The adapter constructs its expected
input map from the prior factory blocks, current selection image and sentinels,
then publishes the **current source logical** callback/list links and peer
counts before comparison. This matters because constructor registrations have
already changed parent lists since the previous child returned. Those bytes
cannot be copied from an old factory snapshot or accepted from the original
corpus without checking their source construction.

All known prior parent blocks, newly published16-byte entry links, current
selection image, full120-byte base and32-byte record sentinels are checked.
All20 resource/settings input regions inherit unchanged from prior real
refresh/table comparisons. Active global145a10798 and index142640504 are
checked against the source state. Catalog begin/end and its full504-byte input
are checked against the constructor's existing catalog values/template. No
parent, index, descriptor, resource record, catalog, selected object, sentinel
or listener list is overwritten by reset preparation in this composed mode.
The standalone harness retains its separate explicit synthetic setup.

The query byte is supplied at the reset entry boundary from the case profile,
as in earlier weather/fifth cohorts. Its real producer/lifetime and propagation
between callers are still unproven. Auxiliary provider strings, heaps and
variants are also declared synthetic inputs, not recovered registry production.

The strict reset subrow is an `S` event inside the whole factory row. It verifies
all main-owned stack locals and every provider entry/exit, operands and result.
The host uses production gate/query/primary/getter defaults. After reset, prior
parent/list/table/catalog inputs are checked unchanged in this cohort; updated
selection image and both owned sentinels return to the constructor. The outer
constructor then consumes a separate finish event. Raw declared heap/bundle
snapshots, all lists/counts, interface links and the logical callback graph
continue to match at all existing outer boundaries.

## Evidence and scope

All118 reset calls complete:236 actual query returns,118 gate returns and64
each of primary comparison/resource getter/tag query. Actual entry and real
RET counts agree per case. Primary reaches34 controlled memcmp and34 secondary
predicate requests. There are70 record assignments,52 invalid-index fallbacks,
8 optional temporary cleanups and91 sized frees. Option/default string getters
are reached35 times each. Three string and three variant-return aliases remain
in use. The inherited catalog's first names are empty in this factory template;
fallback forwards the actual zero length, whereas standalone reset proofs also
cover nonempty names and live base changes.

The constructor leaves selection+c0 zero, so the gate's early-skip branch is
not reached here.54 zero-engine cases enter catalog handling directly;16 of64
nonzero-engine cases pass the actual tag test, and48 go directly to notifying.
Fresh selection receiver lists remain empty, so no actual notification request
is reached in this cohort. The separate528-call reset corpus retains its817
notification requests and mutation coverage; this factory comparison does not
replace that evidence.

Each case makes one controlled finish request from reset and one from the
constructor:118+118. The source and original traces require both. The initial
1410fe0f0 dependency remains controlled and does not make an additional finish
request in these profiles; its real conditional nested call remains unported,
so two is not a whole-application upper bound. The final finish body is also
controlled, not recovered by observing its two call sites.

590 retained callback entries match, as in the previous actual-alternate cohort.
Real RB backing, non-null callable implementation and full wrapper payload ABI
remain outside the logical graph proof. Allocator, config/default, copy/parser,
string assignment, variant visitor/cleanup, record assignment, initial callback,
secondary predicate and final finish bodies remain controlled. CRT and other
refresh providers retain their prior documented scope. The new reset is bound
in the recovered factory host/replay; the generic Services default remains a
legacy compatibility seam, and this does not establish live viewer integration.

## Reproduction and regressions

Raw numeric corpora remain local and are not shipped:

```sh
set -e
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-reset > /tmp/factory-reset-new.txt
mv /tmp/factory-reset-new.txt /tmp/factory-reset.txt
OPENXPLANE_UI_FACTORY_SELECTION_RESET_VECTORS=/tmp/factory-reset.txt cargo test --offline --locked -p xp-app --test ui_factory_composed_local native_factory_with_actual_selection_reset_and_all_refresh_chains -- --ignored
```

`--native-reset` implies the preceding alternate/changed/weather/child/root/
engine/table modes. Reset has a reusable scope-aware harness: outside its
active call it forwards prior stubs, and after tail finish it relinquishes the
scope before the constructor's second finish request. Selection getter-entry
observations from the old constructor prefix exclude the new reset scope;
reset independently audits its actual getter entries/returns. Legacy118
alternate-factory and528 standalone-reset corpora regenerate byte-for-byte.
The standalone reset replay also passes again after harness extraction.

Full initial/finish effects, registry/query/stack producers, runtime ownership,
complete dialog/layout/widgets/Main Menu, original Alt/Esc, live GPU and paired
screenshot/GIF/video acceptance are still unfinished. No UI/graphics or whole-
game parity claim follows from this composition. Compatibility estimates are
reviewed and unchanged.

Validation:118-case actual-reset factory replay and528-case standalone reset
replay passed explicitly.204 ordinary xp-app tests passed,81 local-proof tests
ignored by default. Workspace/all-targets Clippy with warnings denied passed.
Earlier full corpus files regenerated byte-for-byte as stated above.
