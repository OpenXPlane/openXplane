# Selection reset predicates and primary string comparison

Reference: X-Plane.exe12.4.3.11, AMD64 PE base140000000, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Evidence is original machine-code disassembly and completed Unicorn calls on
synthetic mapped state. No native analysis provider or live X-Plane UI is used.

## Recovered caller order

Complete14114d330 caches qword[owner+c8]. With a nonzero receiver it calls
1417fb320 at142f2d770, then conditionally14113fff0 and14113ff10 with the
**cached** receiver. The three checks short-circuit: a zero engine byte or a
nonzero AL from either predicate blocks reset. Only after all reached calls
it reads the **live** byte[owner+c0]. Its final AL is one exactly when that
byte is nonzero and the blocking condition is false. With a null receiver,
none of those three calls occurs, but+c0 is still read. Owner and receiver
are different objects; the receiver must not be reloaded after a dependency.
The source returns a boolean; stale upper native RAX bits are outside this gate
contract.

Complete14113fff0 loads global142640504, calls the actual resource getter
141a64260 using descriptor146124398, then compares the string at the returned
record+6178 with receiver+20. Each32-byte string supplies its length at+10 and
capacity at+18. Capacity<=15 selects inline bytes; larger capacity selects
qword[data]. Unequal lengths return nonzero AL without invoking memcmp. Equal
lengths call1423c0180 on the selected pointers and cached length, and normalize
its zero/nonzero result. Embedded NULs do not terminate this comparison.
The source's default primary predicate uses the recovered signed-low32,
wrapping64 getter and mapped byte equality. The returned0/1 is canonical AL,
not a reconstruction of unspecified upper RAX bits. It neither frees nor
modifies these strings itself.

Complete1417fb390 zero-extends byte[receiver+68], subtracts2 in AL and tests
unsigned AL<=1. It returns full RAX0/1 for exactly the original tags2/3. This
helper is a read-only tag test; no new semantic label or ownership is inferred.
It is exported for the pending reset implementation, not wired into an
approximate viewer path.

## Original-code comparison

`gen_ui_selection_reset_gate_vectors.py` asserts the reference digest and normal
return of every call. It executes original14114d330,14113fff0,1417fb320 and
141a64260. Only secondary predicate14113ff10 and CRT memcmp1423c0180 are replaced.
Counters at real entry/RET boundaries agree:897 engine calls,752 primary calls,
and752 getter calls. Both primary return sites are observed. There are635
memcmp and518 secondary requests. Dependency-entry and exit snapshots, pointer
and count operands, final declared memory and gate AL are checked by the Rust
replay, which inherits production engine/primary/getter defaults.

1536 gate cases comprise1152 systematic cases,256 cases spanning every engine
byte and128 deterministic random cases. They cover null/non-null receiver,
zero/nonzero+c0, low-byte secondary results0/1/0x100/0x101 plus random64-bit
results, inline/heap/mixed strings, unequal lengths, embedded NULs and empty
strings including null heap pointers with zero comparison length. Real-stride
resource cells at indices0/1/-1 are supplied with valid backing; this does not
prove that a negative index is a valid application state. Poisoned images are
compared byte-for-byte in the declared input regions.

676 controlled dependency effects set+c0, replace+c8 with a decoy and change
142640504. These stress cached receiver and late flag reads. They do **not**
claim that real memcmp or the real secondary predicate performs those writes.
762 cases return skip=true. The replay requires every original dependency
request and every input/output byte to match, and rejects leftover events.

Another768 completed original1417fb390 calls exercise all256 tags against
three independently poisoned receiver images. RAX starts at all ones, its full
result matches the source, and every receiver byte remains unchanged. No
callee is replaced for this tag helper.

The local corpus is not published. Reproduce explicitly with a local reference:

```sh
python tools/gen_ui_selection_reset_gate_vectors.py /path/to/X-Plane.exe > /tmp/reset-gate-new.txt
# Rename only after the generator exits successfully.
mv /tmp/reset-gate-new.txt /tmp/reset-gate.txt
OPENXPLANE_UI_SELECTION_RESET_GATE_VECTORS=/tmp/reset-gate.txt cargo test --offline --locked -p xp-app --test ui_selection_reset_gate_local -- --ignored
```

## Remaining reset and integration work

These predicates are preparatory components. Whole reset14113a460 is still an
opaque dependency in the selection constructor and composed factory. Its
catalog string fallback, aliasable temporary variant, live base reload,
assignment141146740, listener loop and tail-call finish14113d660 have not yet
been compared as a complete function. Static control flow shows that reset
ends with a jump to finish while the constructor subsequently calls finish
again. Both explicit calls must survive the future composition. In addition,
the inspected prefix of the initial dependency1410fe0f0 contains a conditional
call to finish at1410fe16d. Thus two is not a proved upper bound on finish
invocations; that dependency cannot be treated as a callback unregistration
without recovering its body. This prefix observation is static evidence, not
a completed original-code comparison of1410fe0f0.

Secondary14113ff10, actual CRT implementation/side effects, registry and engine
byte producers/lifetime, callback dispatch/backing, complete dialog/layout/
widgets/Main Menu, Alt/Esc integration, GPU and paired screenshots/GIF/video
remain unverified. Mapping errors and malformed/oversized string profiles do
not establish native fault or invalid-parameter equivalence. No compatibility
percentage or whole UI/graphics claim follows from these isolated CPU proofs.

Validation for this change:204 ordinary xp-app tests passed,79 local-proof
tests ignored by default; the new2304-case opt-in replay and prior118-case
whole-factory alternate/changed/weather replay passed explicitly. Workspace
Clippy with all targets and warnings denied passed. The new gate is not yet
composed into that prior factory replay.
