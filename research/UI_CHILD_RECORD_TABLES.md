# Indexed resource queries in selection and context

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Selection constructor1410d6fc0 and context refresh1411396d0 now have a shared
production resource-presence implementation: read the descriptor, select the
signed-low32 index with stride6668 in hex, then test the full64-bit string length
at record+6188. ui_indexed_records::resource_present operates on logical mapped
memory. Both Services traits offer a default resource_memory/provider binding
and default presence implementation using this helper. Legacy adapters explicitly
retain their controlled endpoint; new factory adapters select the mapped helper.
They do not infer availability from the old anchor-record flag.

The presence service now returns Result<bool>. Callers propagate mapping errors
through their existing Result/partial-state paths instead of silently treating an
unmapped reference as an absent resource. Native invalid-memory fault/exception
semantics are not established; comparisons below use valid mapped inputs only.
The fallback resource_memory method returns an explicit provider-required error.
This does not populate a real game registry or wire these models into the viewer.

## Actual original getter bodies and shared state

The explicit --native-child-tables generator mode implies native root/fifth table
and fifth query/predicate modes plus actual context refresh.118 complete original
factory1410d6ca0 calls execute all four constructors and the previously recovered
root/nested/payload/context/fifth/hash/predicate chain.141a64260 now executes
originally inside selection and context as well, rather than returning the
controlled record88 pointer.

Real entry/RET observations count45 selection and45 context returns: each caller
requests index88 in23 cases and77 in22. Selection keeps a pending entry/return
stack, empty at each factory return, and at most one returned query per case.
Context entry/RET counts must match per refresh. Normal original factory return
and the existing child-return/allocation/object/declared heap/bundle/callback checks
remain asserted, including649 retained list entries.

At each selection getter entry, an observation J records the live index, whole
selection image and all20 root-derived table input regions. The source adapter
checks those bytes before calling the production mapped helper; J does not replace
the native body or inject its result. Context subrows add one explicit field
containing the20-region snapshot at refresh entry. The source checks it against
its inherited root map, binds that map to the replay and performs the production
lookup at the resource index reloaded after engine gating. There are118 such
context snapshots. Getter R/I mock events are absent in the new mode.

All nine full resource strings, nine settings words and both descriptors remain
inherited into fifth execution and are checked there too. The fixtures contain
valid distinct per-index inline strings plus record88's inline/heap/aligned
profiles. Allocation, string ownership, registry construction and real table
population/lifetime remain outside the proof; see UI_ROOT_RECORD_TABLES.md.
No array-producing or asynchronous registry mutations are modeled here.

## Resulting branch coverage and limits

Selection takes23 alternate and95 changed paths. Context takes23 alternate and95
default paths, with95 controlled parses and65 frees. Thus the new caller decisions
follow the actual indexed cell instead of the prior shared anchor availability.
These are synthetic profile counts, not occurrence rates in X-Plane. Root/nested
getter counts remain226 resource/46 settings, fifth counts90 resource/23 settings,
with236 fifth query returns and118 actual constructor index20 queries. Fifth
retains23 collection/sort,11 payload-assignment and12 mark-lookup requests.

Root/nested/selection/context engine queries still use controlled effects, while
the fifth read-only query body is original with a supplied input byte. Actual
selection alternate/changed/reset/final and weather refresh/provider effects,
context parsing/calendar/selection providers, sparse hash erase/iterator/growth/
sort/payload/free/notification dependencies and callback RB backing remain
controlled or incomplete. No full dialog/layout/widget/singleton/Main Menu lifetime,
opening Alt/Esc, viewer wiring, live GPU or paired visual evidence is established.

## Reproduction and validation

```sh
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --native-child-tables > /tmp/factory-child-tables.txt
OPENXPLANE_UI_FACTORY_CHILD_TABLES_VECTORS=/tmp/factory-child-tables.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_selection_and_context_resource_tables -- --ignored
```

All118 composed cases pass, including45 selection getter-entry memory comparisons
and118 context input-map comparisons.204 ordinary xp-app tests pass;72 opt-in tests
are ignored by default. Earlier972 context and920 selection constructor cases
pass after the Result API change. The previous118 root-table factory replay also
passes, and regeneration preserves all118 of its C rows byte-for-byte. Workspace/
all-target Clippy with -D warnings, Python AST, formatting and diff checks pass.
Executable/assets and numeric corpora stay local; source, generators and reports
are the published artifacts.

No live viewer/GPU or paired screenshots/GIF/video were run or produced. Estimates
reviewed and unchanged:41.4% implemented/28.2% verified overall, Rendering/UI
verified0. Next recover actual query-byte/registry production and remaining
selection/weather/provider bodies, complete the dialog/menu and connect the
verified state and resource pipeline to live rendering before claiming parity.
