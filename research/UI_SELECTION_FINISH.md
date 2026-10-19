# Whole selection finish caller and independent scalar oracle

Reference: X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
The main unwind interval is14113d660..14113db49 (1257 bytes). Evidence is
original-code CPU emulation on synthetic mapped state, strict Rust replay, and
an independent scalar instruction probe. No live simulator or GPU is covered.

## Recovered caller

`ui_selection_finish::finish` now implements the whole caller through its real
RET at14113db48. It clears both output-vector ends and the checksum, obtains a
manager/readiness result, and handles both ready and inactive branches. The
ready branch compares the primary record and floating value, assigns on change,
and binds a temporary when the live selection vector is empty. The inactive
branch initializes/assigns/cleans a temporary and walks the cached catalog
bounds, XORing each live record hash and inserting its pointer. Direct append,
in-place overlapping shift and growth requests preserve the original operands.
The source's100000-visit guard rejects unverified nonterminating profiles; it
is not a recovered original termination rule or fault-parity claim.

The initial token is stored in an incoming home slot and loaded again only
after those branches. Controlled bind/cleanup effects can change that slot;
the late load, rather than the original getter result, selects the remaining
path. Nonzero token constructs the600-byte work object and optionally copies
from the second parent. Manager/readiness is queried again independently. The
actual selection gate chooses resource index0 or20, then the caller performs
its separate scalar operations and two fresh indexed resource lookups. The
base value is cached across providers; table pointers are reloaded. The final
comparison selects the first raw output only when it is strictly greater;
equal values, signed zero and unordered comparisons select the second word.
The output receives the cached token and both vector addresses, followed by
temporary and primary cleanup. Primary cleanup also runs on zero-token paths.

The first float getters consume their returned pointer, which may alias external
or cached owner storage. The later getter instead consumes its designated stack
output and ignores the returned pointer. These contracts are tested separately.
Stack arguments and narrow AL/R9b conditions retain their original widths.

Four scalar literals retain their reference bit patterns:12=`41400000`,
254=`437e0000`,100=`42c80000`,10000000=`4b189680`.
The divisor at142650450 is `4ba83308` (22046224), in PE **read-only .rdata**
(characteristics40000040). The model reads that mapped word. Synthetic harness
patches exercise zero/infinity/NaN divisors and late reads; they do not establish
that the real simulator mutates this constant or accepts such production state.

## Complete caller proof and controlled boundaries

560 completed original calls (432 systematic +128 deterministic random) match
source provider order, operands, return values and every declared byte at each
provider boundary and final return. Readiness queries vary independently and
exercise low-byte values0/1/255/256/257, null/present parents, aliases, live parent
changes, pending flags, vector layouts, cached catalog bounds with live hashes,
late token changes, signed zero, infinities, quiet/signalling NaNs, subnormals
and nontrivial finite mantissas. The vector shift provider uses overlap-safe
copying. Growth is a controlled insertion into mapped backing. Boundary effects
are explicit synthetic inputs, not recovered helper implementations.

Observed counts:284 ready /276 inactive,212 assignments,122 direct appends,
175 shifts,63 growth requests,305 work/output paths and1141 record destructors.
The156 scalar paths reach45 index0 and111 index20 selections. Actual original
entry/RET pairs match for130 engine queries,156 gates,106 primary lookups and
418 indexed resource lookups; these bodies are not replaced by endpoint returns.

Record copy/init/assignment/destruction, token query1417f9060, manager/readiness,
record predicate141a3cbc0, float getter1411363f0, secondary14113ff10, CRT/string
comparison, vector growth, work/scalar-output and final output bodies remain
controlled. The existing fresh-record initializer is not yet bound here. The
finish body is **not yet composed into the recursive reset/callback/factory
proofs**; their recorded finish endpoints retain the scope documented in
[UI_SELECTION_RECURSIVE_RESET.md](UI_SELECTION_RECURSIVE_RESET.md).

Frame declarations include only main-owned stack ranges: with post-prologue
SP=entrySP-648, SP+20..SP+5e7 and SP+650..SP+667. Saved registers/XMM slots and
return address are excluded. The temporary, primary and work objects begin at
SP+50, SP+1f0 and SP+390. Every provider snapshot includes owner, declared stack,
parents, catalog, vector backing, controls, external aliases and selected global/
resource-header regions. Whole address-space equality, valid ownership, native
faults and arbitrary recursion are not established.

## Why the arithmetic has two evidence scopes

An isolated original ADDSS with two quiet NaNs exposed a difference between
Unicorn2.1.4 and an independent x86 instruction process under Rosetta. For
`7fc12345 + ffc54321`, Unicorn selected `ffc54321`; the independent instruction
selected the destination `7fc12345`. Mixed signalling/quiet inputs differ too.
Unicorn's [versioned softfloat source](https://github.com/unicorn-engine/unicorn/blob/2.1.4/qemu/fpu/softfloat-specialize.inc.c)
contains the x87 quiet/larger-significand/tie selection used by this emulator.

The production scalar helper uses destination-NaN priority, quiets a selected
NaN, preserves separate f32 rounding steps, and returns the x86 negative
indefinite pattern for invalid operations without a NaN operand. The raw whole-
caller corpus is explicitly marked **U214**; only its replay provider overrides
both-NaN propagation to match that emulator. The generator refuses an unaudited
Unicorn version. Therefore the560 raw caller matches do **not** independently
prove the production NaN policy.

`tools/probe_ui_selection_sse.c` provides an independent MULSS/DIVSS/ADDSS oracle:
22 raw values squared times three operations =1452 results, including both NaN
orders, signed zeros, infinities, subnormals, rounding-sensitive finite values and
the caller constants. Production scalar values match all1452 results locally.
This process was compiled for x86_64 and executed under **Rosetta on macOS ARM**;
it is not execution on the original Windows PC. An ordinary x86_64-only Rust
unit test repeats the1452 comparisons against inline SSE instructions with
masked, nearest MXCSR and restores the prior MXCSR afterward. The [hosted x86_64 Linux job](https://github.com/OpenXPlane/openXplane/actions/runs/39289045864/job/118184560786)
completed successfully, and its log explicitly records this test as passed on
`stable-x86_64-unknown-linux-gnu` (Rust1.99.1). This is separate native x86
instruction evidence. The [Windows x86_64 job](https://github.com/OpenXPlane/openXplane/actions/runs/39289045864/job/118184560784)
also completed with the same test explicitly passed on
`stable-x86_64-pc-windows-msvc`. Neither job executes the complete original
simulator. All three platform test jobs and warnings-denied lint passed for
commit88c5b3cff093d2b0d1a9be63913d059157a72744. Exception
flags, non-default rounding, DAZ/FTZ and broader NaN-sensitive oracles remain
outside this value-only verification.

## Reproduction and validation

Numeric corpora remain local; no executable, aircraft assets or raw corpus is
published. Use the audited Unicorn2.1.4 environment for original-code generation:

```sh
set -e
python tools/gen_ui_selection_finish_vectors.py /path/to/X-Plane.exe > /tmp/finish-new.txt
mv /tmp/finish-new.txt /tmp/finish.txt
OPENXPLANE_UI_SELECTION_FINISH_VECTORS=/tmp/finish.txt cargo test --offline --locked -p xp-app --test ui_selection_finish_local whole_original_finish_vector_insert_live_fields_scalar_bits_and_cleanup -- --ignored
# macOS with Rosetta; record the actual execution environment on other hosts.
clang -arch x86_64 -O0 tools/probe_ui_selection_sse.c -o /tmp/selection-sse-probe
arch -x86_64 /tmp/selection-sse-probe > /tmp/selection-sse.txt
OPENXPLANE_UI_SELECTION_SSE_VECTORS=/tmp/selection-sse.txt cargo test --offline --locked -p xp-app --test ui_selection_finish_local default_scalar_values_match_independent_x86_instruction_oracle -- --ignored
```

Local validation:560 full callers and1452 independent scalar results passed;
204 ordinary xp-app tests passed,89 local-proof tests ignored by default.
Workspace/all-targets Clippy with warnings denied passed. The new inline-assembly
unit test is excluded on the local ARM host. The completed Linux and Windows CI logs prove
its separate native x86 runs, rather than inferring it from the local204 count.

Next: compose actual finish with recursive reset, initial callback and all
factory refreshes, then recover controlled providers and real state producers.
Full dialog/widgets/Main Menu, Alt/Esc live integration, GPU and paired screenshot/
GIF/video acceptance remain unfinished. Compatibility estimates were reviewed
and remain unchanged; whole-game1:1 parity is unproven.

Integration constraints for the next stage: reset's tail JMP reaches finish at
finish-frame=reset-frame-570; a normal callback call reaches it at callback-
frame-650. Finish must inherit current parent/catalog/table/owner state, suspend
ancestor gate/helper observers while its own scope is active, and publish all
caller regions only after its real RET. A normal callback finish must not be
misclassified as reset tail completion. Recursive reset and suspended callback
frames must then resume in LIFO order. Constructor composition additionally
needs a fallible finish bridge carrying current image/sentinel/callback state.
These are implementation constraints from the recovered callers, not claims
that this composition already executes.
