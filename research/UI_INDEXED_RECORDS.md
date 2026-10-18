# Signed record accessors and real tables in the fifth child

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

Original141a64250 (settings) and141a64260 (resource) have the same four-instruction
shape: sign extend EDX, multiply by a fixed stride, add the64-bit base stored at
[RCX], and return RAX. Settings stride is0x433c0 and resource stride0x6668.
There is no count/capacity check or record dereference. Index high32 bits are
ignored; low0xffffffff means-1 and computes the previous record address, not the
last record. Arithmetic wraps64 bits. The resource/settings caller descriptors
are global146124398/1461243b0 respectively; only their first pointer word is proved.

ui_indexed_records exposes typed kinds, pure address calculation, generic
lookup_at over a mapped descriptor and lookup at the version-bound globals.
These are logical addresses, never unsafe host pointer dereferences. Default
ui_fifth_refresh service methods now call this port for its resource/settings
queries. Legacy replay adapters explicitly retain controlled getter endpoints;
new TableReplay inherits the production predicate and both production lookups.

## Direct original-code evidence

872 complete original calls have no callee stubs:360 systematic cases cross both
kinds, six base words, ten low-index boundaries and three high argument patterns;
512 seeded cases use arbitrary64-bit bases/arguments. Full RAX matches and the
descriptor word stays unchanged. Cases cover signed32 boundaries, high-bit discard,
carry/wrap and noncanonical computed values. Only the descriptor is dereferenced
by these kernels. This does not prove those results are valid records, that a
caller permits negative indices, or error behavior when a descriptor is unmapped.
Other registers/flags are outside this comparison.

## Fifth refresh with original getters

528 complete fifth141139460 executions include actual availability predicate
14113b9a0, hash-clear dispatcher/bulk/bucket fill, and both original getter bodies.
The scope dispatcher yields to original code for getters; entry/real-RET counters
must agree per case.297 resource and45 settings getter returns are observed.
They are not replaced by a Python address calculation. All existing object/
declared memory/global/index/override snapshots, other provider arguments/live
reads/local writes, cleanup and notification checks remain active.

Synthetic arrays use the original strides, with mapped fields for indices
-1/0/1/5/20/77/78/88/89. A96-record allocation deliberately positions the base one
record in, so the tested-1 cell has valid backing; this does not assert that real
engine tables reserve such a sentinel. Only needed scalar words and descriptors
are projected into Rust memory. Whole record contents, count/capacity metadata,
construction, strings/ownership and actual registry population are not proved.

Resource presence differs across selected cells; neighboring settings cells have
opposite zero/nonzero modes. Some second engine callbacks shift both descriptor
bases by one stride. Default lookup must therefore read the current descriptor
and select the correct signed-index cell instead of returning one aliased record
or caching a previous base. Engine and other provider effects remain controlled.
The array fixtures are data inputs, not real engine occurrence frequencies.

## Whole factory and constructor query20

118 complete original factory calls contain all four child constructors and the
actual root/nested/payload/context/fifth chains and availability predicate. Getter
code is original inside the fifth refresh and for its constructor's index20
resource request. A separate real-RET counter requires exactly one such constructor
return per case; it excludes calls while fifth refresh is active. The caller now
stores resource record20 rather than the old controlled record88 alias. Rust
binds this constructor endpoint to the same public lookup port using inherited
table descriptors. Subsequent object/heap snapshots check the stored pointer.

There are90 fifth resource returns,34 fifth settings returns and118 constructor
resource returns in this composition. Other fifth counters retain34 collection/
sort branches,22 growth requests,44 iterator advances,17 payload branches and17
mark lookups. Root record88 settings mode remains the shared word. Root's known
word and both descriptor bytes are inherited and compared before fifth execution;
all prior allocation/child-return/final heap/bundle, receiver and callback checks
remain active, including649 retained entries.

Root, selection and context getter bodies still use controlled providers in this
new mode. The root fixture points at array record88, so reached fifth settings
queries read the same physical record/word, but this does not certify root lookup
arithmetic or registry mapping for all its other requested indices. Port those
caller adapters and their full resource-string inputs next. The new proof is
explicitly fifth/constructor access, not all lookup calls throughout the factory.

## Reproduction and remaining work

```sh
python tools/gen_ui_indexed_records_vectors.py /path/to/X-Plane.exe > /tmp/record-accessors.txt
OPENXPLANE_UI_INDEXED_RECORDS_VECTORS=/tmp/record-accessors.txt cargo test --offline -p xp-app --test ui_indexed_records_local -- --ignored
python tools/gen_ui_fifth_refresh_vectors.py /path/to/X-Plane.exe --native-tables > /tmp/fifth-tables.txt
OPENXPLANE_UI_FIFTH_TABLES_VECTORS=/tmp/fifth-tables.txt cargo test --offline -p xp-app --test ui_fifth_refresh_local native_fifth_refresh_with_actual_signed -- --ignored
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --factory --native-context --native-tables > /tmp/factory-tables.txt
OPENXPLANE_UI_FACTORY_FIFTH_TABLES_VECTORS=/tmp/factory-tables.txt cargo test --offline -p xp-app --test ui_factory_composed_local native_factory_with_real_fifth_record_tables -- --ignored
```

Original executable/assets and generated corpora stay local. Only source,
generators and reports are published. Earlier528 actual-predicate/controlled-
getter rows and118 shared-gate factory rows regenerate byte-for-byte unchanged.
New modes have explicit replay selection; a controlled getter event cannot
silently stand in for production default lookup calls.

Next apply real lookups to root/selection/context, recover actual engine query,
registry data production/ownership and remaining selection/weather refreshes and
other providers. Full dialog/layout/widget bindings/singleton/Main Menu/teardown
and original live Alt/Esc/GPU remain incomplete. No paired screenshots/GIF/video
were produced or validated by these checks. Shares reviewed and unchanged:41.4% implemented/28.2% verified overall;
Rendering/UI verified0.

Validation:204 ordinary tests pass;67 opt-in tests are ignored in that run.
All872 direct accessor,528 actual-table fifth and118 factory/constructor table
cases pass. The three prior/current fifth replay modes and earlier118 shared-gate
factory replay pass too. Earlier528/118 controlled-getter C rows regenerate
unchanged. Formatting, Python syntax, all-test-target compilation and diff checks
pass. No live viewer/GPU run or paired visual capture was performed.
