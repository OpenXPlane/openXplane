# Actual string append and growth callers

Reference: X-Plane.exe 12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Static evidence: LLVM disassembly of append140624040..1406240c6 and
growth1405eca90..1405ecc14. Execution evidence: actual original instructions
under Unicorn2.1.4, with controlled allocation1422e7bf0, overlap-safe
move1423c0280 and sized free1422e7c2c. Those primitive bodies are not recovered
by this work. Executable, resources and raw numeric corpora remain local.

## Recovered order

Append caches the original length/capacity and uses unsigned wrapping64
capacity-length to choose its path. Even a zero-byte in-place append invokes
move. It writes length before calling move, selects the original inline or heap
destination before that call, and writes the terminator at the cached address
afterward. Provider changes to object length/capacity/pointer survive; they do
not redirect that terminator. The returned full RAX is the object address.

For growth it copies an incoming shadow byte into R8 and puts the count in its
fifth stack argument. That byte is not interpreted as an application flag.
The recovered growth body does not consume R8. Growth amount and the fifth
argument's source-copy count are separate operands and are not forced equal.

Growth caches old length and capacity. Normal-path capacity is the larger of
`(old_length + amount) | 15` and `old_capacity + (old_capacity >> 1)`, with the
original signed-maximum saturation tests and wrapping operands retained.
Requests of at least4096 bytes add39, align `(allocation + 39) & ~31`, and store
the base pointer at aligned-8. Smaller requests use the returned pointer directly.

After allocation the body writes new length/capacity while leaving the old
pointer in the object. In the heap branch it reads that pointer only now, copies
the cached old byte count, copies the new source count, writes NUL, then frees
the cached old buffer. Large-free header bytes are read after both moves; sized
free consumes wrapping old_capacity+1 and, for large buffers, an extra39.
The unsigned data-base-8 check must be <=31. Only after free does the body publish
the new pointer. Metadata changed by providers is not restored afterward.
SSO growth follows the same two-copy/terminator sequence without freeing old
storage. These distinctions are preserved in `ui_string_append`.

## Whole original comparisons

The direct corpus contains3936 completed normal-return calls:1023 outer appends
and2913 direct growths. Appends include407 nested real growth calls, giving3320
actual growth entries/returns. All1023 append returns and3320 growth returns
reach original RET sites; no append/growth endpoint is replaced with a post-state.
There are3320 controlled allocation calls (1990 large),7256 moves and2487 frees.
Strict Rust replay compares full RAX, every declared byte, and each primitive's
function, arguments, pre-state and post-state. All3936 rows pass.

Cases cover inline/heap storage, empty/zero-byte operands, capacity boundaries
15/31/63/4094/4095/8191, binary data, self/metadata/new-buffer aliases, distinct
growth/copy counts, arbitrary ignored-R8 values and provider mutations at
allocation, move and free. Mutation probes replace the object pointer before
old-buffer selection, overwrite length/capacity, and alter the aligned-free
header between copies. Saturation cases use synthetic huge capacity metadata;
a controlled allocator supplies only the small backing actually accessed.
They prove request/free operand arithmetic, not successful huge allocations,
valid real string invariants, allocator ownership or OS memory behaviour.

## Actual class-string composition

Production class generation now defaults to recovered append/growth rather than
opaque string endpoints. `with_memory` introduces only missing append scratch,
rejects partially overlapping declarations, preserves live provider effects and
removes only the temporary declarations it created. Class-to-append and
class-to-growth post-prologue frame deltas are -40 and -50 hexadecimal.

A separate original cohort executes960 complete class-string builds and60
label constructors with actual append/growth. It retains the signed tree shapes,
empty/full/mixed masks, root changes during label allocation, poisoned storage,
output/record alias and low-address-byte variation. It uses normal string
semantics: historical controlled append tail edits and growth-result aliases
are not claimed to be real growth behaviour. Their old corpus remains separate.

This new cohort completes4018 label returns (3958 nested plus60 direct),768
actual short resizes,3958 append returns and2419 growth returns (432 direct
separator growths and1987 nested append growths). Its4390 string subrows match
arguments and exact frame relationships, verify all inherited input bytes and
execute recovered source bodies. Expanded allocator declarations propagate back
to the class caller; callee-only stack scratch does not. String scope records
2419 allocations,6809 moves and1318 frees; class scope adds432 label allocations
and858 temporary frees. Every normal outer return, declared boundary and final
memory matches strict replay.

An ordinary production-default test constructs a nonempty tree for keys1/8.
Only allocation, real overlap-safe copying and sized free are provided by the
host; an opaque append/growth request fails the test. The result is
`General Aviation/Science Fiction`, length32/capacity47. Four allocations,
three matching frees and six moves leave exactly the final output allocation;
new nested stack maps expire. This checks the production bridge and temporary
ownership for this host profile, not the original allocator implementation.

The earlier960+60 controlled-string class replay also passes unchanged in scope.
Earlier comparison/finish/callback corpora explicitly override append/growth and
retain their controlled-string proof boundary. The prior recursive/factory
cohorts still have their documented controlled comparison boundaries. New source
defaults do not retroactively upgrade any of those original-code corpora.

## Reproduction and limits

```sh
set -e
python tools/gen_ui_string_append_vectors.py /path/to/X-Plane.exe > /tmp/strings-new.txt
mv /tmp/strings-new.txt /tmp/strings.txt
OPENXPLANE_UI_STRING_APPEND_VECTORS=/tmp/strings.txt cargo test --offline --locked -p xp-app --test ui_string_append_local -- --ignored
python tools/gen_ui_record_class_string_vectors.py /path/to/X-Plane.exe --native-strings > /tmp/classes-strings-new.txt
mv /tmp/classes-strings-new.txt /tmp/classes-strings.txt
OPENXPLANE_UI_RECORD_CLASS_NATIVE_STRINGS_VECTORS=/tmp/classes-strings.txt cargo test --offline --locked -p xp-app --test ui_record_class_string_local whole_class_generation_with_actual_append_and_growth -- --ignored
cargo test --offline --locked -p xp-app --test ui_class_string_defaults
```

Local validation:207 ordinary xp-app tests passed;100 original-dependent tests
are ignored by default. Workspace/all-targets Clippy with warnings denied and
formatting pass. Direct replay took18.02s; class/string replay took91.66s;
the legacy class replay took8.60s. Local raw corpus SHA256 values:

- strings: `643db1a88c7d3ee2f2e3c0e57baf3aef48bce7720b856c12647648b626119909`;
- class/string composition: `9d4d003d7393163f21fe042267bacb0c80a6ba6408d579d333c4b686a82d32be`.

Both local gzip archives were fully decompressed and checked against those
hashes. This protects the saved evidence; it does not expand the proof scope.
The generator checks the reference SHA256, instruction-count bound and normal
return. Local corpora are absent from hosted CI; its ordinary tests and lint are
separate evidence. Exceptional length/overflow/invalid-parameter paths are
explicit source errors, not a port of native exception/unwind behaviour. Real
CRT/allocator/free bodies, failure semantics, record/tree producers, arbitrary
ownership/reentry and lifetime remain unverified. Other builds are untested.

Next: carry actual strings through equality, finish, callback and recursive
factory cohorts; recover remaining record/secondary/float/CRT producers and
bodies; complete dialog/widget/layout/menu rendering and viewer key integration.
Alt/Esc UI, GPU and paired original screenshots/GIF/video remain unverified.
No visual change is claimed. Compatibility estimates were reviewed and left
unchanged: isolated instruction proofs do not establish whole-simulator parity.
