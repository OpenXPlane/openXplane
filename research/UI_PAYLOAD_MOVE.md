# Payload move1410faec0 and owned buffers

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered behavior

The nested refresh uses1410faec0 to assign its temporary into member+40. The
port covers this entire function, optional string move1405de290, vector move
1410fa780 and vector growth1408b0660. Allocation/free and standard memcpy are
controlled in the native harness; the four function bodies execute unchanged.

Leading0x2c bytes and scalar region60..a4 copy without float arithmetic. Padding
is retained. Optional string flags use nonzero semantics: both present moves the
string after releasing destination backing; newly present copies raw ownership,
resets source length/capacity/first byte and sets destination flag1. Source flag
is retained. Removing an option releases its destination string and resets only
metadata/first byte/flag. Heap releases use capacity+1 bytes; buffers4096 bytes
or larger recover the aligned allocation header and add39 bytes. String/self
moves preserve the original alias check and residual bytes.

A heap source vector transfers its three pointers after destination release;
source resets all three to its inline address. An inline source uses current
length/capacity: shrink/copy, grow within capacity, or allocate a larger buffer.
Growth capacity is max(requested, first power of two strictly greater than
old_capacity+2). The old length is captured before allocation; old buffer/end
are reread after allocation, then its backing is released. Source inline length
becomes0 while its capacity stays unchanged. Exact self-move skips nested string
and vector transfers and remains byte-for-byte unchanged.

Mapped State stores object/backing bytes with reference identities, not callable
host pointers. Backing blocks remain mapped after free so fixtures can compare
residual bytes; the service owns free bookkeeping. Valid nonoverlapping objects
or exact self-alias, aligned bounded vectors, valid string ownership and successful
allocations apply. Corrupt/partial-overlap/failed-allocation and exception cleanup
profiles are not established. Mapped guards deliberately reject invalid input.

## Native evidence

720 whole original assignments reach normal return and return the destination.
Comparison covers both complete0xe0-byte objects, every declared backing block,
and complete allocation/free entry and effect snapshots. Service effects can
change source scalar bits during allocation/free, checking read order. Counts:
72 self-moves,144 vector allocations,216 vector frees and216 string frees.

Matrix: two poison patterns; source/destination optional flags0/1/255; four string
storage pairs (inline, small heap and4095-capacity aligned backing); ten vector
profiles covering empty, shorter/longer, spare capacity, growth, inline/heap
ownership transfer, null source storage and self-alias. Capacity0 heap storage is
retained as a zero-length mapped identity. Padding/residual and aligned-header
bytes are compared. No weather numeric or GPU equivalence is inferred.

Generate tools/gen_ui_payload_move_vectors.py with local exe and Unicorn, save
stdout outside the repository and set OPENXPLANE_UI_PAYLOAD_MOVE_VECTORS for
cargo test --offline -p xp-app --test ui_payload_move_local -- --ignored.
Only source, generators and research notes are published.

## Nested refresh composition

432 additional complete native14113a300 runs now execute actual1410faec0 and its
string/vector helpers.27 runs reach the assignment branch; the others verify
gating and listener paths. Combined assignment uses inline optional strings and
inline vectors of length0/2/8 with destination capacity8. It therefore does not
exercise heap release or growth inside the composed refresh; the standalone
720-case corpus above covers those paths. Temporary/provider construction,
resource/context effects, optional destruction and notifications remain controlled.

The assign_nested adapter replaces the assignment service with the actual source
port. It synchronizes nested object, temporary and backing state, retaining partial
state on errors within the service contract. Combined replay checks full image,
temporary, globals/index/links and service order. Earlier432 cases with assignment
controlled continue to pass. Generate gen_ui_nested_refresh_vectors.py EXE
--native-payload and set OPENXPLANE_UI_NESTED_PAYLOAD_VECTORS for the ignored
combined test in ui_nested_refresh_local.

Final root refresh141138cf0 still needs a port: inspection shows resource and CLI
string/record parsing branches, vector ownership and callback/listener updates.
This change supplies a real dependency of its nested refresh, not that full body.
Remaining record/config/provider functions, full factory/dialog/layout/singleton/
menu, Alt/Esc viewer behavior and paired GPU screenshots/GIF/video are unverified.
Compatibility estimates stay41.4% implemented/28.2% verified overall; Rendering/UI
verified0. No visual progress is claimed from these CPU comparisons.

Validation:204 ordinary tests pass;54 opt-in tests are ignored in that run.
The720-case payload replay and both432-case nested-refresh replays pass
separately, including the combined run through assign_nested. Formatting,
Python syntax and diff checks pass. No live viewer/GPU check was performed.
