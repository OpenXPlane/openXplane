# Combined child, original refresh and retained parent registration

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Gap closed

Earlier216 child comparisons controlled the entire refresh1411396d0; the972
refresh comparisons executed it independently. Ordinary integration already
composed the ports, but did not prove their original combined behavior.

`tools/gen_ui_child_refresh_combined_vectors.py` now executes original child
1410d7490 through normal return, including its **actual original refresh body**,
then actual temporary wrapper construction, peer registration, parent receiver
141149460, wrapper copy and temporary base teardown. Neither child nor refresh
is stubbed. Observation hooks assert refresh entry and its return to child+0x92 (0x1410d7522);
execution resumes with retained refresh effects before callback construction.

Only leaf engine/resource/context/option lookup/CRT/time conversion/notification
providers and allocator/free endpoints are controlled. Native refresh branches,
casts/clamps/two float divisions, option cleanup and callback/list bodies execute
unchanged. This closes a composition-evidence gap, not all provider behavior.

## Compared profile

972 cases reuse the standalone refresh's gate/CLI/float-boundary/NaN/string
profiles. The fresh child's own observer list is empty, as initialized by its
constructor; nonempty/mutable observer behavior remains independently tested
by the standalone refresh replay. Parent callback lists start empty or with
retained copies (first list0/1/3, second0/0/2), with both parent arguments and
three poison patterns. The constructor's bound temporary has null custom callable.

The combined profile keeps both parents valid and does not use the standalone
free endpoint's synthetic parent+1 mutation; free may still change mode. That
avoids treating a dangling parent as valid constructor input. Prior standalone
and controlled-refresh child cases retain their separate mutation coverage.

Rust executes the recovered child with `refresh_image` invoking the recovered
refresh caller, then the recovered callback/link/list/teardown models. It matches:

- All96 final object bytes and120 base-sentinel bytes.
- Raw base/callback allocation snapshots and refresh entry/return state.
- Every leaf service's object/global/listener snapshot, arguments and request order.
- Parent next/previous/count fields and all live callback parameters/ordered peers.
- Full combined allocation/free request order and absence of the temporary.

All972 combined cases pass. Callback-tree balancing/padding/heap bytes and
free-pointer order remain opaque; this is not complete native heap-byte equality.
Actual CRT parsing/calendar/context providers, nonnull callable lifetime,
allocation failures, reentrancy and other FP environments remain unverified.

## Shared replay verification

`ui_context_refresh_harness.py` and the Rust support replay share provider
fixtures between independent and combined tests. Regenerating the independent
corpus after extraction produced exactly the same972 C rows byte-for-byte.
The standalone972-case Rust test passes again with the shared support code.
The sharing does not replace original execution with a generated Rust answer.

Combined corpus stays local at `/private/tmp/openxplane-ui-child-refresh-combined.txt`;
opt-in replay uses `OPENXPLANE_UI_CHILD_REFRESH_COMBINED_VECTORS`. Executable,
assets and numeric corpus are not published; generators/support/test/report are.
No new production behavior is claimed for this evidence-only composition step.

## Remaining work

Recover actual remaining nested constructors1410d6a40/1410d6fc0/1410d75a0 and
resource/provider bodies, then compare a complete original factory with recovered
nested components. Finish dialog140ec9c10/layout/widget bindings, singleton/Main
Menu construction and teardown, opening ID3/Alt and live viewer integration.
Original GPU output and paired screenshots/GIF/video remain missing; Esc still
exits the viewer. Overall estimates remain41.4% implemented/28.2% verified and
Rendering/UI verified0. Whole-game/visual parity remains unproven.
