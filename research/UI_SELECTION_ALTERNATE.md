# Whole selection alternate and both constructor exits

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

ui_selection_alternate recovers the complete14111bb90 caller. The first actual
settings lookup tests the32-bit word at+24c. A zero selects a56-byte current
record, cached before any optional cleanup. The existing tag3 fast path copies
it directly; other valid tags switch to3, with cleanup only for active tag0.
Caching matters when a provider result aliases owner data and cleanup mutates it.

A nonzero word initializes the result slots and a temporary context, checks
readiness, loads two raw64-bit settings fields through separate cached pointers
and override calls, and passes them with fixed float bits41f00000 to the search
provider. The vector path preserves the cached end passed into erase while
reloading live begin/end for later search/comparison. Found records and fallback
selection produce the scalar/secondary/record24 branches. Aligned allocation
headers and sized-free operands, including the4096-byte threshold, are recovered.
Search/vector/data/allocator bodies themselves remain controlled.

The assignment branch decodes the signed tag byte using the original tables at
14111c074/14111c08c. Supported inputs are0/1/2/3/4/ff (signed-1); inactive payloads
do not acquire invented cleanup calls. Metadata writes precede cleanup for tag0.
The simple temporary destructor140e20d50 is also recovered: it tail-calls optional
cleanup only when its trailing byte is nonzero. The native wrapper executes in
the comparisons; actual string cleanup remains a service.

The common exit writes only three low bytes at+c0/+c1/+c2 as1/0/0. The+c3 byte
has a separate earlier store on the nonzero-settings path, and a later temporary
context destructor can change it. The port preserves those effects and does not
replace the sequence by a32-bit flag reset. Floating values travel as raw bits;
no host arithmetic substitutes for the unported calculation providers.

## 528 complete original executions

432 systematic and96 seeded profiles cover both major paths, all six valid tags,
optional states, three result forms, readiness/vector/search outcomes, returned
record aliases and mutable callee effects.244 calls select the current-record
path and284 the context/search path.1324 real settings getter returns occur,
with per-case entry/RET equality and normal outer return asserted.

There are215 vector requests,122 filter/erase/search calls,121 found-data requests,
245 fallback-data requests,91 secondary assignments and101 record24 assignments.
Sized frees cover8/32/4088/4135 bytes and nonnull zero-capacity profiles. These are
caller requests under supplied allocator/provider effects, not allocation or
real search occurrence statistics. The declared arena projection includes the
header/used prefix; the unused capacity and real allocation lifetime are not
compared as a complete heap object.

The real temporary destructor is entered92 times;31 optional cleanup requests
include28 owner and3 temporary-payload requests. Providers can change the live
index, cached settings fields, vector end, result slots, tag and final flag byte.
Whole declared object/map/global/index states and every provider boundary match,
as do logical integer/floating/stack arguments and live operand reads/writes.
Raw NaN/infinity/signed-zero words remain unchanged where the caller just copies.

Scratch/home bytes are explicit inputs. In particular the byte at frame+628 is
forwarded into a filter before a later output writes that slot; its real producer
is not established by this comparison. The standalone harness supplies it before
entry, and composed runs capture the original caller frame. Only the initial
scratch window and live provider operands are compared, not complete register/
stack ABI or incidental spills. Invalid tags/alignment/faults and exception paths
remain outside the valid mapped profiles and produce source errors where checked.

## 118 complete factories

--native-alternate includes the prior root/nested/context/weather/fifth and
selection-changed chains and all reached real table getters.23 original selection
constructors now execute actual alternate, while95 execute actual changed.
Alternate splits into12 current-record and11 context/search branches, including
all three result forms. It executes56 settings returns and2 temporary destructors.
All previous full child-return/allocation/declared heap/bundle/list/callback checks
remain active. The source bridge validates the initial image, inherited global/
index and20 core table regions, then publishes them back to subsequent callers.
The settings+24c and+390/+398 words are supplied at alternate entry as new fixture
inputs; real registry/state production is still unproven.

Neither actual handler directly assigns the second parent+d0, and these supplied
leaf effects do not rebind it. Therefore the23 alternate cases also lose the old
opaque parent-rebinding effect: all118 use only the primary selection parent.
The composed result now has590 retained entries instead of the earlier601
(controlled alternate) or649 (both endpoints controlled). This is a profile
consequence, not a universal binding count for the live game. Earlier cohorts
keep their explicit scopes and regenerate unchanged.

## Reproduction and remaining work

```sh
python tools/gen_ui_selection_alternate_vectors.py /path/to/X-Plane.exe > /tmp/selection-alternate.txt
OPENXPLANE_UI_SELECTION_ALTERNATE_VECTORS=/tmp/selection-alternate.txt cargo test --offline -p xp-app --test ui_selection_alternate_local -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-alternate > /tmp/factory-alternate.txt
OPENXPLANE_UI_FACTORY_SELECTION_ALTERNATE_VECTORS=/tmp/factory-alternate.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_selection_alternate_changed_and_weather_chain -- --ignored
```

All528 direct and118 composed cases pass.204 ordinary xp-app tests pass with78
opt-in tests ignored. Workspace/all-target Clippy with -D warnings passes. Earlier
920 selection constructors and118 changed-factory replays pass; regeneration
preserves all118 previous C rows byte-for-byte. Python AST, formatting and diff
checks pass. Executable/assets and generated corpora remain local; only source,
generators and reports are published.

Selection reset/final, real context/readiness/override/search/filter/erase/data/
string-cleanup/allocator bodies, actual notify/effects1410c9d10, registry/engine
producers, ownership, callback RB backing and complete dialog/widget/layout/menu
lifecycle still need recovery. No live viewer/GPU or original Alt/Esc/paired
screenshots/GIF/video evidence was produced. Reviewed shares remain41.4%
implemented/28.2% verified overall, Rendering/UI verified0.
