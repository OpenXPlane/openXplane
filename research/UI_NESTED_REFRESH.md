# Nested root refresh14113a300

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered body

The first child invokes this function on its+268 member (0x158 bytes). It clears
byte130, captures vector begin/end140/148 and releases each non-null pointer via
140f59550. End is captured before calls; later slots are read live. Once the old
range is visited, it reloads begin and stores that into end. An initially empty
range does not perform that store.

Engine1417fb320 returns a byte. Only nonzero engine and zero active global enter
the resource branch. Resource141a64260 is looked up again after availability at
+6188 succeeds. Assign141142ce0 runs, followed by settings141a64250 and a zero
mode at+dac. Each lookup rereads the global index142640504. A third resource
lookup supplies temporary1417f7610. Assignment1410faec0 targets object+40. The
outer body then reads the temporary vector pointer+a8: if it differs from inline
buffer identity+c0 it calls142301120, including a null pointer. Optional cleanup
1408ab620 targets temporary+30 and follows that free.

The listener dirty flag18 becomes1. Initial next comes from the current head8;
notify1410c9d10 targets entry+16. Next and current head are reread after each
notification, so live mutations can skip entries or change termination. The flag
is cleared after traversal. Rust bounds malformed/nonterminating input rather
than claiming native exception or corrupted-memory behavior.

## Evidence and boundaries

432 whole original runs reach the normal return address. Source comparison checks
all0x158 object bytes, active global, resource index and declared item/list-link
memory at every provider call and final state. Temporary identity/raw0xe0 bytes
are compared across assignment and optional cleanup. Provider mutations are
replayed; they are not evidence of the provider body's behavior.

Matrix: three poison patterns; engine0/255; post-engine active0/7; resource absent/
present; settings mode0/7; vector lengths0/1/3; listener profiles empty/stable3/
mutated-to-stop-after1. Release effects change vector begin/end and a future slot,
checking captured end, live slot reads and reloaded begin. Resource assignment
changes the global index to77. Temporary assignment selects inline/heap/null
vector pointers before cleanup. Cases contain360 item releases,27 temporary
assignments/destructions,18 vector frees (9 heap/9 null) and576 notifications.

Controlled bodies: item release, engine, resource/settings lookup, resource
assignment, temporary construction, payload assignment, temporary free, optional
cleanup and notification. Successful valid states, bounded aligned ranges,
retained nodes during callbacks and default FP environment apply. Native provider
computation, heap ownership/error recovery and callback invocation remain pending.

Generator tools/gen_ui_nested_refresh_vectors.py requires local reference and
Unicorn. Save stdout outside the repo; set OPENXPLANE_UI_NESTED_REFRESH_VECTORS
and run cargo test --offline -p xp-app --test ui_nested_refresh_local -- --ignored.
The local numeric corpus and proprietary files are not published.

## Constructor composition

192 additional whole native root1410d6a40 runs execute original14113a300 without
a stub, with controlled engine0 and final root refresh141138cf0. The port calls
ui_nested_refresh::refresh_image on the actual initialized member through the
constructor refresh adapter. Full root/all seven heap blocks and allocator/
refresh entry snapshots match. Fresh vector/list sentinels and engine0 are the
combined profile; the standalone matrix above covers other refresh branches.

Generate with tools/gen_ui_dialog_child_root_vectors.py EXE --native-nested.
Set OPENXPLANE_UI_ROOT_NESTED_REFRESH_VECTORS and run the ignored combined test
in ui_dialog_child_root_local. The earlier192 cases with both refresh effects
controlled still pass separately. This does not establish complete native factory,
final root refresh, dialog/layout/singleton/Main Menu or live GPU behavior.

Next recover141138cf0 and remaining provider/effect bodies, compare complete
factory/dialog composition and finish original opening/Alt/Esc and GPU captures.
No paired screenshots/GIF/video or visual-parity evidence is added here.

Validation:204 ordinary tests pass;52 opt-in tests are ignored in that run.
The432-case standalone refresh,192-case native constructor composition and
earlier192-case controlled-refresh constructor replays pass separately.
Formatting, generator syntax and diff checks pass. No live GPU test was run.


2026-10-17: [payload assignment](UI_PAYLOAD_MOVE.md) now has720 native cases;432 additional refresh runs execute that original body (27 reached assignments, inline buffers). Earlier controlled-assignment evidence remains scoped as above. Final root refresh and remaining providers are still pending.
