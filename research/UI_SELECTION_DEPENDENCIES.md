# Selection mode, header and singleton dependencies

Date: 2026-10-25; REA verification completed2026-10-26. Reference X-Plane.exe12.4.3.11 SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
This continues [the secondary predicate](UI_SELECTION_SECONDARY.md).

Static instruction review uses Capstone5.0.7 and completed REA6.7.0 GNU objdump
Evidence for all three functions on the same original PE. The transparent
`tools/rea_bounded_objdump.rs` adapter adds fixed start/stop addresses and records
the backend path and bounds in captured stderr. It leaves the target unchanged.
The installed backend reports GNU Binutils2.46.0.20260210; its executable SHA256
is `0ec6c3c15420e29ca28a51b793f6862b5cf8db7a4ec67517c88f72df7f409f7e`.
REA labels its provider version unknown; the backend version is a separate local
observation, not an inferred REA version probe.

| Function | Half-open address range | REA Evidence |
|---|---|---|
| Mode |1414cedd0..1414cef70 (416 bytes)|ev_a463e0fc9c4a3943e611077c356097a5101d723d37eb77e23c376f5aac744690|
| Header |1411222e0..14112240c (300 bytes)|ev_9d06666be79c834e0df4349c62a558320ee560ab841f900bf597b3a9e074104b|
| Singleton |1414cc7f0..1414cc814 (36 bytes)|ev_b23f03a3371a7bdaddd88fd9429443cec37b66870ae59d8b1e2a0684229d836c|

Each result reports exit0, complete output, no truncation, unchanged input and
the reference SHA256 before/after. Every emitted byte was independently checked
against the mapped original PE: all752 bytes match and cover the complete named
ranges, including error tails/padding. Reviewing instructions confirms the
ordered calls, result widths, temporary cleanup and lazy singleton behavior
described below. These are bounded disassembly observations, not decompilation,
whole-binary analysis or proof of the supplied leaf implementations.

The unrestricted REA/objdump attempt hit its16MiB output limit before these
functions and is not used as their evidence. REA/Ghidra12.1.4 full-PE analysis
twice reached its1800000ms deadline. Thread snapshots identified Scalar Operand
References and later OperandReferenceAnalyzer after RTTI processing. A third
profile disabled Scalar Operand References, Reference and Windows x86 PE RTTI
Analyzer; its owned process was explicitly stopped after the bounded REA route
completed. It produced no successful decompilation. A raw-x86_64 request was
rejected by the Windows provider, which admits native PE and raw6502 only.
Source-byte provenance, complete Evidence and original listings remain local;
no original instructions or listings are published.

## Recovered behavior

`1414cc7f0` reads the pointer at145a12340. A nonzero pointer returns unchanged.
Otherwise it requests exactly one byte from1422e7bf0 and stores the complete
returned pointer, including NULL. It does not construct an object or preserve
a different pointer published by the allocator. Allocation/ownership, races and
real singleton initialization remain outside this comparison.

`1414cedd0` ignores its incoming receiver. It obtains settings through1414d45e0,
builds inline strings `_glider_tow` (length11) and `1` (length1), and calls
1414d45f0 with a separate output descriptor. The returned descriptor can differ
from that output. It reads the returned length and data pointer, compares exactly
one byte with `1` only for length1, and tests low32 of the CRT result. It returns
zero-extended0/1, independent of upper CRT/cleanup result bits.

Cleanup addresses the local output, key and default, in that order, rather than
the descriptor returned by the provider. Each capacity/pointer is read when its
turn arrives. The output and key are reset to length0/capacity15/leadingNUL after
their release; the default is not reset. Heap releases pass capacity+1 bytes;
at4096 bytes and above the preceding alignment header supplies the allocation
base and39 extra bytes. The allowed displacement is8..39. Native invalid-header
termination/unwind is unverified; the Rust mapped interface returns an error.

`1411222e0` checks actual class-membership(receiver+20,2). Absence returns full
RAX0 without touching its string temporary. Presence obtains the recovered
singleton, requests a string through1414ccc80 and ignores that call's return
identity. It fetches a catalog through1417e9f50 and looks up the temporary with
1417e8a60. A NULL result triggers successive pair lookups1417e89c0 for
`Piper PA-18 Super Cub` (21 bytes) and `Cessna Skyhawk` (14 bytes), fetching a
fresh catalog before each request. A third failure fetches the catalog again and
requests index0 through1417e9a40. The last result, including NULL, is cached
across temporary release and returned without canonicalization. The temporary
uses the same heap/alignment release rule, without a reset.

## Native execution comparison

`tools/gen_ui_selection_dependencies_vectors.py` maps the reference executable
in pinned Unicorn2.1.4 and requires normal outer returns within10000 instructions.
The Rust replay compares provider order and complete argument widths, declared
memory before/after every boundary, named local operand bytes and final memory.
Standalone mode/header also compare their complete local data regions, excluding
saved registers, shadow space and return slots. Whole secondary/gate compare
declared non-stack maps and reached local string/pair operands; unused stack
bytes and saved-register/ABI effects are not claimed.

| Outer call | Cases |
|---|---:|
| Singleton1414cc7f0 |48|
| Mode1414cedd0 |116|
| Header1411222e0 |160|
| Secondary14113ff10 with actual dependencies |1536|
| Gate14114d330 with actual secondary/dependencies |128|

All1988 cases match. Entry counts equal observed RET counts:4438 singleton,
3965 mode,621 header,1664 secondary and128 gate bodies. No singleton, mode,
header, membership, primary, resource getter, secondary or gate endpoint is
stubbed. Controlled leaves are settings singleton/accessor1414d45e0/1414d45f0,
header-string producer1414ccc80, catalog singleton/lookups1417e9f50/1417e8a60/
1417e89c0/1417e9a40, allocation/free1422e7bf0/1422e7c2c and CRT1423c0180.
These bodies, their real data production and real ownership remain unverified.
Gate results compare consumed AL only; the other four outer modes compare full
RAX. No broader native register/stack ABI equivalence is implied.

The corpus covers all three-query Boolean sequences; four temporary storage
profiles (inline, ordinary heap, aligned4096 and larger aligned buffers); empty,
one-byte, embeddedNUL, binary and unequal-length values; returned descriptor
aliasing; independent local-output cleanup; all catalog fallbacks; missing
membership; allocation NULL and a reentrant allocator store. Provider mutations
replace pending cleanup descriptors, change globals between queries, replace
late index/table state and change cached resource contents during header lookup.
CRT return probes distinguish low32 zero from nonzero despite upper bits.
Read-only reference index words are patched only as explicit synthetic inputs.

An initial whole-secondary fixture returned NULL from every header lookup.
The original subsequently dereferenced NULL in equality. Standalone header
retains all-NULL coverage; whole-secondary/gate use valid returned descriptors.
This is a fixture correction, not a skipped native body or a fault-parity claim.

## Source composition

`ui_selection_dependencies` implements the three bodies with explicit mapped
memory and local-stack addresses. `ui_selection_secondary::with_dependencies`
composes them with the existing actual primary/member/resource/equality chain.
Only missing local regions are introduced; partial overlaps fail explicitly,
inherited maps persist, and owned scratch is removed on success or provider error.

Standard reset/finish/callback bridges now use this chain. Five ordinary tests
cover those bridges, cached receiver/resource and full-RAX propagation, preserved
provider side effects, error cleanup and partial-map rejection. Their stack
positions follow the native call/prologue chain: secondary is0x60 bytes below
reset/finish's post-prologue RSP through the gate, or0x30 below
the callback's. This source integration is not whole-original reset/finish/
callback/factory composition with these new dependencies. Historical endpoint
replays retain their earlier controlled scope.

## Reproduction and remaining work

The successful REA route uses the installed CLI6.7.0, caller-supplied GNU objdump
and the transparent range adapter. Build the adapter locally and select one of
`mode`, `header`, `singleton`. Use absolute paths for backend, adapter and target:

```powershell
rustc tools/rea_bounded_objdump.rs -o research/local/rea-bounded-objdump.exe
$env:REA_OBJDUMP_COMMAND = "$PWD/research/local/rea-bounded-objdump.exe"
$env:OPENXPLANE_OBJDUMP = '/absolute/path/to/objdump.exe'
$env:OPENXPLANE_REA_RANGE = 'mode'
rea inspect-with-objdump "$PWD/Xplane12/X-Plane.exe" disassemble --json > research/local/rea-mode-bounded.json
```

REA records the adapter as its configured command. Its captured stderr discloses
the actual GNU backend and added bounds; do not describe it as an unmodified
REA invocation or whole-binary decompilation. No REA/client installation or
configuration repair is required. A SHA256 identifies bytes, not authenticity.

```powershell
python tools/gen_ui_selection_dependencies_vectors.py 'Xplane12/X-Plane.exe' > research/local/selection-dependencies.txt
$env:OPENXPLANE_UI_SELECTION_DEPENDENCIES_VECTORS = "$PWD/research/local/selection-dependencies.txt"
cargo test --offline --locked -p xp-app --test ui_selection_dependencies_local -- --ignored
cargo test --offline --locked -p xp-app --test ui_selection_secondary_defaults
```

The186721348-byte local corpus has SHA256
`aad193d6186fac4ec901a1b0b384401eb1f7c333632e6653aba05575127d5a7d`.
The original executable, corpus, provider listings and resources remain local.
Formatting, workspace tests and workspace/all-targets Clippy with warnings denied
pass. The original-dependent replay is explicitly ignored in ordinary CI.
The earlier7128-row secondary/primary/equality/gate corpus was regenerated and
passes unchanged under its explicitly controlled dependency endpoints.

Next recover settings/accessor and header-string/catalog producers, and compose
these actual dependencies through complete reset/finish/callback/factory runs.
Real data lifetime/reentry, native faults/exceptions, complete dialog/layout/
widgets/Main Menu, live Alt/Esc/viewer/GPU and paired original screenshots/GIF/video
remain unverified. No visible menu change or whole-model parity is claimed;
compatibility estimates remain unchanged.
