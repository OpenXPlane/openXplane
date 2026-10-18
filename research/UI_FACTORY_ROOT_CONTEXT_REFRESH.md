# Whole factory with actual root and context refreshes

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

118 full original factory1410d6ca0 calls now contain all four actual child
constructors, inline fifth/hash initialization, root141138cf0, both nested
14113a300 calls, payload/string/vector helpers and context1411396d0. Each reaches
the original normal return with RAX equal to the bundle identity. The context
refresh runs during its actual constructor, before retaining its parent callback.
No second standalone original call or substituted refresh body generates this
combined evidence.

## Shared index and source bridge

Context assembly1411396f7 reads shared global142640504 after its engine/global
gate and passes it in EDX to141a64260 at141139704. Like the previously corrected
selection child, the old context port used a constant0. ui_context_refresh::State
now carries resource_index and reloads that field after services.engine. The new
refresh_image_with_index bridge transfers image/global/index/list state, including
partial state on errors. The existing refresh_image remains a documented
index0 compatibility entry for earlier fixtures/callers.

The source factory transfers root runtime global/index into selection and then
context construction, runs recovered context refresh through that indexed bridge,
and publishes returned state before continuing callback registration/weather
construction. Both selection and context have34 lookups at index88 and11 at77.
Context resource absence at77 takes the default path. Context providers keep the
shared index unchanged in these profiles; arbitrary provider index mutations are
not established by this corpus.

## What is compared

The prior root and factory checks remain active: root live memory/global/index,
provider arguments/operand reads/writes, outer allocation/child-return/final bundle
and declared heap snapshots, receiver links/counts and final logical callback
peers.649 retained entries survive their temporary cleanup. Context adds a nested
provider trace using the existing context replay format. At every leaf entry it
compares the complete96-byte context object, global and live next links, plus
logical arguments, payload/context arrays, native string raw/storage bytes and
free requests. Initial image, shared global/index, final image/global/index/links
and all provider-event consumption are checked through the indexed source bridge.
Incidental native stack spills and callback RB backing remain outside parity.

The native scope dispatches shared engine/resource/options/free endpoints to
context providers only while original1411396d0 executes; root and other-child
endpoints retain their earlier behavior. Its providers construct valid synthetic
records/strings and produce controlled parse/conversion results. This establishes
caller arithmetic and cleanup order, not real CRT/calendar/resource calculations.

## Observed coverage and limits

The118 root/config/resource cases also vary context Zulu/day presence, payload
modes0/1/2/255, negative/finite/infinite/NaN parse results, negative zero and day
2^31 boundaries.40 cases use inline strings,39 ordinary heap and39 aligned large
strings. There are34 alternate and84 default context branches,168 option checks,
82 option fetches/parses,52 sized frees,42 context-day and26 context-hour calls.
A controlled day-string free can change the mode to1, exercising the caller's
post-cleanup mode reload. Full context raw objects and later factory snapshots
remain exact across those effects.

This is a sampled joint matrix, not the Cartesian product of independent corpora.
Context receiver lists are fresh/empty and parent identities stay valid/stable;
no context notifications occur before other children attach. Earlier independent
972-case context evidence covers mutable nonempty listener traversal, wider option
boundaries and controlled parent changes; earlier972-case constructor/refresh
composition is retained. Heap strings here belong to controlled option providers,
not factory-owned engine resources. Constructor failure/exception paths, preexisting
interface chains and native callback RB layout/free-pointer order remain unverified.

## Reproduction and remaining scope

```sh
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context > /tmp/factory-root-context.txt
OPENXPLANE_UI_FACTORY_ROOT_CONTEXT_VECTORS=/tmp/factory-root-context.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_root_and_context -- --ignored
```

Earlier118 factory/root-chain rows regenerate byte-for-byte unchanged; the new118
rows regenerate identically too. Original executable, assets and corpora stay
local. Only source, generator support and this report are published.

Context engine/resource/default/alternate/options/CRT/calendar/free endpoints
remain controlled. Root leaf dependencies, selection property/config/reset/finish,
weather population/altitude/selection/refresh/special and fifth refresh141139460
remain controlled as previously documented. Next recover fifth and other refresh/
provider bodies, full dialog/layout/widget bindings/singleton/Main Menu and teardown.
Live Alt/Esc, GPU pixels and paired screenshots/GIF/video remain unverified. Shares
reviewed and unchanged:41.4% implemented/28.2% verified overall; Rendering/UI verified0.

Validation:204 ordinary tests pass;60 opt-in tests are ignored in that run.
The new118-case root/context factory replay passes, as do the earlier216-case
controlled-refresh factory and118-case actual-root factory. Both prior972-case
context and context-constructor compositions pass. Old118 factory/root rows and
new118 root/context rows regenerate identically. Formatting, Python syntax,
all-test-target compilation and diff checks pass. No live viewer/GPU run or paired
visual capture was performed.


2026-10-18: [fifth refresh and full chain](UI_FIFTH_REFRESH.md) now has528 standalone native cases and118 whole-factory calls containing actual root/context/fifth bodies. Range erase and other providers remain controlled; selection/weather refreshes and full dialog/live GPU are next. Estimates unchanged.


2026-10-18: [actual payload gate and shared settings](UI_FIFTH_PAYLOAD_GATE.md) now has528 new whole-fifth and118 whole-factory native comparisons with14113b9a0 executing originally. Root/fifth settings identity and bytes are checked; original lookup/provider production and live GPU remain pending. Earlier opaque-predicate evidence retains its explicit scope. Estimates unchanged.
