# Actual secondary selection predicate and full-RAX string predicates

Reference X-Plane.exe12.4.3.11 SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Static evidence: LLVM disassembly of secondary14113ff10..14113ffe4 (212 bytes),
primary14113fff0..141140050 (96 bytes) and string equality14073ba40..14073ba7c
(60 bytes). Execution evidence uses their original instructions under
Unicorn2.1.4, plus actual membership141a400f0, resource getter141a64260,
outer gate14114d330 and engine query1417fb320. Singleton1414cc7f0,
predicate1414cedd0, header1411222e0 and CRT memcmp1423c0180 remain controlled.

## Ordered state machine

Secondary caches its receiver across all dependencies. It first calls primary;
nonzero AL returns false. Next it calls actual class membership on receiver+20
at key2; zero AL returns false. The remaining steps are ordered:

1. Fetch a singleton and call its predicate. If AL is nonzero and either
   DWORD145899fd8 or145899fdc is nonzero, return true. Reads short-circuit.
2. Fetch the singleton again and call the predicate again. If AL is zero and
   either DWORD145899fd0 or145899fd4 is nonzero, return true.
3. Fetch and query again. Zero AL returns false. Otherwise read secondary index,
   obtain and cache its resource record, then call header1411222e0 on the cached
   original receiver. Compare that returned string with resource+6178 and invert AL.

The predicate's three results and singleton pointers are independent. Caching
one result would change behaviour. The resource address remains cached across
header production, but its descriptor is read after that call. A provider may
change the table/index and the cached record's bytes; the former must not
redirect the record, while the latter must be observed.

The normal branches replace AL only. Upper RAX bits survive from the last
reached dependency. Primary retains the selected right data address on unequal
lengths, sets AL1, and on equal lengths preserves the entire memcmp result when
low32 is zero; otherwise it sets AL1. Equality retains right length on unequal
lengths with AL0; for equal lengths it replaces only AL with low32 memcmp equality.
These are full-RAX source functions. The older primary gate interface remains
explicitly canonical-AL; only AL is consumed there.

## Read-only indices and synthetic mutations

PE section inspection confirms142640504 and142640508 belong to read-only
`.rdata` (characteristics40000040); their reference DWORD values are0 and1.
The four state DWORDs145899fd0..fdc are in writable `.data` (c0000040).
These index words are not asserted to be mutable engine state.

288 new calls start with the literal0/1 pair and distinct resource0/resource1
strings;96 of them leave those words unchanged. Other profiles patch0/1/-1
indices, or modify them from controlled providers, as synthetic instruction/data
flow tests. Their table bases keep the accessed slots mapped. These mutations
prove ordered loads and cached-address behaviour under injection, not real
producer behaviour or permission-valid writes to `.rdata` in the original game.

## Original runs and strict replay

| Outer mode | Completed calls | Compared result |
|---|---:|---|
| Secondary |4968|Full64-bit RAX|
| Primary |72|Full64-bit RAX|
| String equality |72|Full64-bit RAX|
| Gate with actual secondary |2016|AL only|

All7128 rows match strict Rust replay in16.90s. The gate cohort also executes actual
engine and primary bodies. Source replay uses actual primary, membership,
resource and equality defaults inside secondary, rather than restoring a whole
child's recorded post-state. Every declared non-stack byte and every primitive
function/argument/pre-state/post-state matches.

Native entry/RET counters cover6984 actual secondary completions (4968 direct
and2016 nested),9072 primaries,6930 memberships,1575 equalities and10575 resource
getters. Each entry count equals its observed normal RET count. Controlled
boundaries are12672 singleton/predicate pairs,1503 header requests and10315
memcmp calls. No opaque secondary/primary/membership/equality/getter endpoint
replaces those bodies. The corpus includes all three-query Boolean sequences,
16 initial state masks, live state changes, binary/embedded-NUL/empty/long/mixed
SSO/heap strings, distinct singleton returns, signed index operands, pointer
replacement, late resource descriptor changes and poisoned unused bytes.

Additional probes use predicate results0x100,0xff,0x102 and high-bit values,
plus memcmp results0xffffffff,0x100000000,0x7654321000000100 and all-ones. The
caller checks AL for predicates and low32 for memcmp, retaining the other RAX
bits exactly. Empty heap strings can have a NULL data pointer; a zero-byte
memcmp still occurs without dereferencing either input.

An initial fixture incorrectly paired a NULL heap pointer with a nonempty
post-header string. Original execution faulted, identifying invalid fixture
backing. NULL is now used only for empty strings; no original body is skipped
or made to return artificially. Native fault/invalid-object parity is not claimed.

## Production bindings and retained proof boundaries

Reset and finish gate bridges now default to recovered secondary; the initial
callback uses it too. All provider effects are published back into shared caller
memory. Three ordinary tests exercise these production bridges with only the
four primitive services, actual membership/primary/equality and original0/1
indices. They require three fresh singleton/predicate pairs, preserve a cached
receiver despite replacement of owner+c8, and retain the cached resource across
a table replacement while reading its changed descriptor. Full result
cafe000000000000 survives the callback bridge. No nested scratch maps escape.

Historical reset/finish/callback replays explicitly override the new secondary
method to retain their controlled endpoint.528 resets,560 finishes and712
callbacks still pass with their prior scope. The older recursive/factory
cohorts are not relabelled: native full parent/factory composition with actual
secondary is still pending. The new ordinary bridge tests do not replace that
missing original-code composition evidence.

212 ordinary xp-app tests pass;103 original-dependent tests are ignored by
default. The legacy finish/reset/callback replays pass in11.58s/3.80s/1.56s.
Workspace/all-targets Clippy with warnings denied and formatting pass.
Hosted CI does not have the original-dependent corpus. The final local replay
uses the same pinned Unicorn2.1.4 after restoring a temporary environment whose
config/package files had been removed; it does not change the emulator version.

## Reproduction and remaining work

```sh
set -e
python tools/gen_ui_selection_secondary_vectors.py /path/to/X-Plane.exe > /tmp/secondary-new.txt
mv /tmp/secondary-new.txt /tmp/secondary.txt
OPENXPLANE_UI_SELECTION_SECONDARY_VECTORS=/tmp/secondary.txt cargo test --offline --locked -p xp-app --test ui_selection_secondary_local -- --ignored
cargo test --offline --locked -p xp-app --test ui_selection_secondary_defaults
```

The generator checks reference SHA256, instruction bound, normal outer returns
and balanced native entry/RET counts. Source and generator remain public;
executable, resources and numeric corpus remain local. The430593180-byte raw
corpus has SHA256
`b8875c5245b782f9f720218503143a3a99428675364c50b2863825df5509d43a`.
Its8003030-byte gzip archive was fully decompressed and hash-checked locally.

Singleton/predicate/header/CRT implementations, actual record/tree/header
producers, real ownership/lifetime/reentry, native faults, exception/unwind and
saved-register/stack ABI effects remain unverified. The memory proof is limited
to declared non-stack maps. No new SSE/FP/MXCSR result is claimed. Next: recover
header1411222e0 and predicate1414cedd0/its singleton, compose actual secondary
through full callback/reset/finish/factory callers, then remove remaining
float/provider/producer dependencies. Whole dialog/layout/widgets/Main Menu,
Alt/Esc live integration, viewer/GPU and paired original screenshots/GIF/video
remain unverified. No visible menu change is claimed; estimates are unchanged.
