# Root, nested refresh and payload assignment composed

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The new chain executes complete root141138cf0 with actual nested14113a300 and
payload assignment1410faec0/string1405de290/vector1410fa780 helpers in the same
normal-return original run. Callback construction/list copy/binding/destruction
also remain original. The root generator's --native-nested mode enables this;
ordinary mode is retained, and its118 C rows are byte-for-byte unchanged by SHA256
comparison after regeneration.

## Source bridge

ui_refresh_chain::refresh_nested replaces the root service call to14113a300.
Nested object bytes, global/index and live pointer slots are published into root
mapped memory before each leaf call and captured afterwards. Other root fields
and backing blocks remain shared. Declared external qword slots provide live
next/item reads; scratch storage and the root object are excluded from that
blanket extraction. In-object item slots are included from a valid item range.

Item release can temporarily change begin past the captured end. Pointer-slot
capture therefore tolerates transient range metadata; ui_nested_refresh still
checks its initial range, then follows its cached end and live slot reads, and
reloads begin when publishing the cleared end. This avoids prematurely rejecting
an observed provider mutation or overwriting nested dirty flags with stale links.

Payload assignment uses the real ui_payload_move port on the shared root/scratch
mappings. The caller supplies a mapped temporary identity; reference stack
addresses are fixture identities, not host pointers. Payload allocation/free
providers preserve shared memory/global effects. Native exception/error recovery
and arbitrary malformed provider mappings are not established.

## Evidence

118 whole composed executions match through the root refresh_image bridge. Full
root/declared memory/global/index snapshots, leaf identities/arguments at logical
widths, live operand reads/local writes, final receiver links and callback peers
match. Initial scratch memory is captured; incidental stack spills are excluded
from later comparisons. Reference callback RB backing and free-pointer order
remain outside the logical graph proof.

The same root/config/resource matrix now calls the real nested function twice
as many engine checks in total (236).34 nested settings lookups and17 temporary
constructions reach actual payload assignment and optional cleanup. Root and
nested receiver lists include empty, stable-three and mutated-stop-after-one
profiles, producing322 notifications in total. Both root and nested item vectors
have lengths0/1/3 and release effects can zero a future slot and change begin.
Resource assignment can change the shared index again, to88.

Combined payloads use inline strings with presence0/1/255 and inline vectors of
length0/2/8 with capacity8. This composition does not exercise payload heap growth
or release; the separate720-case payload corpus covers those paths. Root string
profiles still cover inline,31-capacity heap and4095-capacity aligned backing.

Controlled bodies remain engine/resource/config/registry/option and settings
lookups, record parsing/application/destruction, root string/vector effects,
nested resource assignment/temporary construction/optional destruction,
enumeration bodies, item release and callback notification. Standard memcpy
and callback allocation/free are controlled. This is stronger composition evidence,
not proof of the calculations inside those leaf providers.

Generate tools/gen_ui_root_refresh_vectors.py EXE --native-nested, keeping stdout
outside the repository. Set OPENXPLANE_UI_REFRESH_CHAIN_VECTORS and run cargo test
--offline -p xp-app --test ui_root_refresh_local -- --ignored. Set the earlier
OPENXPLANE_UI_ROOT_REFRESH_VECTORS too to run both118-case replays. Local corpus,
exe and original assets are not published.

## Remaining scope

Each fixture is initialized by original root construction with refreshes controlled
before a separate composed refresh invocation. The entire root constructor with
both real refreshes is still to be compared in one native run; then the complete
child factory/dialog/layout/widget bindings/singleton/Main Menu and teardown.
Remaining provider computations and live viewer wiring also need recovery.
Original Alt/Esc/GPU and paired screenshots/GIF/video remain unverified.
Compatibility shares reviewed and unchanged:41.4% implemented/28.2% verified
overall, Rendering/UI verified0.

Validation:204 ordinary tests pass;56 opt-in tests are ignored in that run.
Both118-case root replays pass separately, including the native composed chain.
Earlier independent C rows regenerate byte-for-byte. Formatting, Python syntax
and diff checks pass. No live viewer/GPU test was performed.
