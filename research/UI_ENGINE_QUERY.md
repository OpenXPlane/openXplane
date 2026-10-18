# Read-only engine query in the fifth dialog child

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The complete original1417fb320 consists of MOVZX EAX, byte[RCX+0x525]
and RET. It returns an unsigned byte zero-extended through EAX into full RAX;
it does not write memory or update the resource index, active global or override.
The fifth refresh141139460 and availability predicate14113b9a0 pass receiver
142f2d770, so the source byte is142f2dc95. Its meaning, producer and lifetime
are not established. In particular this evidence does not establish an aircraft
engine-running or simulator-ready semantic.

`ui_engine_query` reads mapped logical memory at the version-bound receiver or
an explicit receiver. `ui_fifth_refresh::Services::engine_query` now uses this
implementation by default at both call sites. Legacy replay modes explicitly
override the endpoint to preserve their controlled provider effects. New
EngineReplay inherits the production query, predicate and indexed lookups;
a controlled engine I event cannot silently stand in for the actual query.

## Direct and composed original-code comparisons

768 complete original calls test every byte0..255 against three independently
poisoned0x600-byte receiver images (0/a5/ff). RAX starts at all ones; its full
returned value matches the port, and every receiver byte remains unchanged.
No callee is stubbed. Other registers/flags and unmapped-pointer faults are
outside this proof; the port returns a mapping error for an unmapped byte.

528 complete original fifth-refresh executions now include the actual read-only
query, actual availability predicate and both actual table getters, alongside
the existing hash-clear dispatcher/bulk/bucket-fill bodies. Entry/real-RET hooks
agree per case and require exactly two actual query returns:1056 total. Resource
and settings accessors return259 and35 times. Original normal return is asserted.
All declared object/memory/global/index/override states, provider arguments,
local operand reads/writes and final bytes remain compared. The query byte is
part of every initial, provider-entry/exit and final snapshot.

Initial engine/global/index/override values are now prepared before entry.
There are no modeled side effects at query entry or return. In the controlled
sort endpoint, selected profiles write the query byte, active global, index and
table bases between the outer query and the payload predicate.54 sorts occur;
15 change the query byte. This verifies fresh subsequent reads under recorded
intervening effects; it does not establish that the real sort has those effects.
Initial query bytes0/1/255 and initial signed indices0/77/88/-1 are exercised.
Distinct real-stride resource/settings cells remain in use.144 cases have no
external provider events at all after removing the query stub; the trace format
now represents their empty event list explicitly as `-`.

118 complete original factory1410d6ca0 calls contain all four constructors,
the actual root/nested/payload/context/fifth refresh chains and the actual query
inside fifth refresh only. They execute236 actual query returns,90 fifth
resource and34 fifth settings returns, plus118 actual constructor index20
resource returns. Initial fifth object bytes, inherited global/index, root
settings word and two table descriptors are checked, as are subsequent declared
heap/bundle/list snapshots and649 retained callback entries.

The query byte and override values are synthetic inputs prepared at the fifth
entry boundary. This does not prove their production or propagation from root.
Root/nested/selection/context engine queries and root/selection/context table
getters remain controlled. Registry loading/lifetime, sparse erase, iterator,
growth, sort, payload, free and notification effects also remain dependencies;
selection/weather refresh bodies, complete dialog/layout/widget/singleton/menu
construction, viewer wiring and GPU parity remain unfinished.

## Reproduction

```sh
python tools/gen_ui_engine_query_vectors.py /path/to/X-Plane.exe > /tmp/engine-query.txt
OPENXPLANE_UI_ENGINE_QUERY_VECTORS=/tmp/engine-query.txt cargo test --offline -p xp-app --test ui_engine_query_local -- --ignored
python tools/gen_ui_fifth_refresh_vectors.py /path/to/X-Plane.exe --native-engine > /tmp/fifth-engine.txt
OPENXPLANE_UI_FIFTH_ENGINE_VECTORS=/tmp/fifth-engine.txt cargo test --offline -p xp-app --test ui_fifth_refresh_local native_fifth_refresh_with_actual_read_only_query_and_tables -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context --native-engine > /tmp/factory-engine.txt
OPENXPLANE_UI_FACTORY_FIFTH_ENGINE_VECTORS=/tmp/factory-engine.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_actual_fifth_query_and_record_tables -- --ignored
```

All corpora/executable/assets stay local; only source, generators and reports
are published. Ordinary xp-app tests:204 pass,70 opt-in tests ignored. The768
direct,528 actual-query fifth and118 actual-query factory cases pass. All four
fifth replay modes and the prior118 actual-table factory cases pass. Formatting,
Python AST and diff checks pass. No live viewer/GPU run or paired capture was
performed. Reviewed estimates stay41.4% implemented/28.2% verified overall,
Rendering/UI verified0; this is CPU behavior evidence, not visual parity.


Follow-up CI maintenance: kept the recovered time-clamp comparison order with a
local manual_clamp annotation, collapsed equivalent range/settings guards and
removed redundant iterator/borrow syntax in replay adapters. No expected data or
behavior checks were removed. Local `cargo clippy --offline --locked --workspace
--all-targets -- -D warnings` passes with Rust/Clippy1.96.0; hosted stable uses a
newer toolchain, so its outcome is checked separately.

The complete local workspace suite passes310 tests with81 opt-in tests ignored;
the original972 context-refresh cases also pass after this maintenance.
