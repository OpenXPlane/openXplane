# Equality with actual class generation, append and growth

Reference X-Plane.exe12.4.3.11 SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
This extends [class-string proofs](UI_RECORD_CLASS_STRING.md) and
[string append/growth proofs](UI_STRING_APPEND.md) into the complete
141a3c910 equality and141a3cbc0 negation callers. The source production defaults
were already bound; this change strengthens original-code composition evidence.
It does not add a rendered menu or alter visible viewer behaviour.

## Allocation lifetime across the two class results

Both derived results must be live together before comparison. Resetting the
synthetic string allocation cursor for each derived call would let the second
string reuse storage still owned by the first result. That is an invalid fixture
contract even if port and original happen to agree on its bytes.

The new mode resets allocation pools only at the outer standalone comparison
boundary. Class entry with inherited memory does not reset the cursor. Growing
pool declarations escape from each actual string child to class and then to
comparison. Source replay publishes the expanded declaration only for pool
addresses explicitly identified by that child's string rows; stack-only child
scratch stays private. Inherited input bytes are checked before executing every
class and string body. No whole child is replaced with its recorded final state.

The original fixture asserts the first result's full descriptor and contents are
unchanged after the second class body in all729 reached pairs. Both results use
heap storage in720 pairs, where their pointers must also be distinct. These are
assertions about this controlled allocator profile, not the original allocator's
reuse policy or ownership rules. Allocation, move, memcmp and sized free bodies
remain controlled and have explicitly recorded effects.

## Original execution and strict replay

1500 original equalities complete with1500 normal equality returns, including
750 real negated wrappers/returns. Result counts are749 zero and751 one. They
execute1458 actual class bodies,8209 actual labels and1456 actual short resizes.
Class/string bodies are original instructions, rather than opaque endpoints.

The8932 actual string subrows consist of8209 append callers and723 direct
separator growths. Appends add4176 nested growth calls, for4899 total original
growth entries/returns. Their primitive boundaries contain4899 allocations,
13831 overlap-safe moves and2735 frees. Class scope adds796 label allocations
and1519 temporary frees. Outer equality scope reaches6066 memcmp and1441 free
requests, including dirty cleanup reloads from the earlier controlled profile.

Strict Rust replay passes in1097.21s. Its test asserts exactly
1500 comparisons,1458 class subrows and8932 actual string subrows; it compares
all declared bytes, every provider function/argument/pre-state/post-state,
full string RAX and canonical comparison0/1 results. Frame deltas remain-a0
comparison-to-class and-40/-50 class-to-append/direct-growth in hexadecimal.

Case inputs retain binary/embedded-NUL/empty strings, inline and heap records,
4094/4095/UINT64MAX capacity metadata, equality short circuits, low32-only
memcmp results, ignored record fields, independently seeded class trees,
live changes at provider boundaries and reverse cleanup. Old controlled string
tail edits and external/overlapping growth returns are disabled in this mode:
actual growth returns the receiver. Their earlier corpora retain separate scopes.
Saturation and invalid metadata cases are synthetic operand tests; they do not
prove successful huge allocations or valid original data production.

An ordinary production-default test uses nonempty equal and unequal category
trees and provides only real allocation, overlap-safe copying, sized free and
memcmp. Any opaque derived/class/append/growth endpoint fails the test. The
same-class case keeps two distinct heap strings live at comparison; all8
allocations are freed. The different-class case frees all6. Both cases retain
an intentional live record-byte change and return exactly to their original
memory-map keys, proving nested scratch expires for this host profile.

209 ordinary xp-app tests pass,102 original-dependent tests are ignored by
default, and workspace/all-targets Clippy with warnings denied passes.
The earlier1500 comparisons with actual class/label/shrink but controlled
append/growth also pass in132.59s; they are not relabelled by this mode.

## Reproduction and stored evidence

```sh
set -e
python tools/gen_ui_record_compare_vectors.py /path/to/X-Plane.exe --native-strings > /tmp/compare-strings-new.txt
mv /tmp/compare-strings-new.txt /tmp/compare-strings.txt
OPENXPLANE_UI_RECORD_COMPARE_NATIVE_STRINGS_VECTORS=/tmp/compare-strings.txt cargo test --offline --locked -p xp-app --test ui_record_compare_local whole_equality_with_actual_classes_append_growth_and_live_first_output -- --ignored
cargo test --offline --locked -p xp-app --test ui_record_compare_defaults
```

The generator verifies reference SHA256 and normal RET/instruction bounds.
The new raw corpus contains6744047680 bytes, SHA256
`266013cf1387c5c2ff02560d189475d0f828fec70f43d20690d012c792243e43`.
Its179674459-byte local gzip archive was fully decompressed and hash-checked.
The executable, resources and raw numeric rows remain local. Hosted CI does not
have these corpora; its ordinary tests/lint are separate evidence.

## Remaining work

Carry actual strings through finish/callback and recursive reset/factory
cohorts, respecting allocator lifetime at the outermost active caller. Earlier
finish/callback class corpora still control strings; earlier recursive/factory
cohorts still have their documented controlled comparison boundaries.
No source default retroactively broadens those original-code proofs.

A static follow-up inspected secondary14113ff10..14113ffe4 (212 bytes).
After primary14113fff0 and class membership141a400f0 at key2, it can call
singleton1414cc7f0 and predicate1414cedd0 up to three times. The predicate
results cannot be cached across calls. Reads of DWORDs145899fd8/fdc and
145899fd0/fd4 are conditional and ordered. The final path reads secondary
index142640508, caches the resource address across header-getter1411222e0,
then negates string-equality14073ba40 against resource+6178. Primary instead
uses index142640504. That initial trace now has a recovered secondary body with7128 original-code
comparisons and ordinary production bindings; see [secondary evidence](UI_SELECTION_SECONDARY.md).
Actual singleton/predicate/header/CRT bodies and whole parent/factory composition
remain unverified.

Real CRT/allocator/free bodies, record/tree/header producers, arbitrary lifetime,
exception/unwind and fault semantics remain unverified. Float/secondary/provider
bodies and further reentry are unfinished. Whole dialog/layout/widgets/Main
Menu, live Alt/Esc integration, viewer/GPU and paired original screenshots/
GIF/video remain unverified. No visual parity or screenshot result is claimed.
Compatibility percentages are reviewed and unchanged.
