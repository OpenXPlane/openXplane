# Child context refresh1411396d0

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered whole caller

The child constructor1410d7490 calls this refresh after initializing its payload
and parent. `ui_context_refresh` now ports the whole caller through normal return.
Engine query1417fb320 runs first; the global145a10798 is read afterward. Only a
nonzero engine byte, null global and nonnull resource0 field+6188 select the
alternate32-byte context1417f74f0. Otherwise manager1414d1170/default1414cda30
supply the context copied to object+38..+58. Actual provider bodies remain services.

Default-context paths query the literal options `--zulu_time` and
`--day_of_year` separately, in that order. Alternate context skips both options.
The Zulu path obtains parent context141126f60, calls1417f9180 for its integer
context code, obtains/parses the option and narrows double to float. It clamps
seconds to0..86400 with x86 comparison/MINSS NaN and signed-zero behavior,
divides by60 **twice** with float rounding at each step, stores code/hour and
sets mode+50=2 before option cleanup.

The day path narrows parsed double to float, reproduces CVTTSS2SI32 invalid
conversion (NaN, infinities or out-of-range become INT_MIN), then clamps the
integer to0..366. In particular positive infinity/float-rounded2^31 produce0,
not366. Heap option cleanup precedes the mode test. Mode1 replaces only the
first integer; other modes reload parent+58, call1417f8960 for hour bits, replace
24 payload bytes with day/hour/zeros and set mode1. Time/context conversion
bodies and actual CRT number parsing remain services; these results are controlled
inputs, not proof of complete calendar or textual-number compatibility.

Heap cleanup uses the previously verified string ownership path, including
large aligned backing headers. Its local metadata reset is not a claim about
native temporary stack bytes. Allocator/free behavior remains controlled.

Finally flag+18=1 brackets listener traversal. Each callback receives node+16;
its next link and object head are reread after the callback. Mutations can skip
listeners, append a previously unvisited node or change the head. Callback flag
mutations survive until final flag+18=0. The model uses stable link identities;
missing/nonterminating graphs return unsupported-profile errors. No native
failure/cycle/exception equivalence claim is made.

## Whole original comparison

`tools/gen_ui_context_refresh_vectors.py` executes original1411396d0 unchanged.
Only engine/resource/context providers, option lookup, CRT parsing, sized free,
time conversion and notify endpoints are controlled. Native branches, casts,
clamps, both float divides, cleanup and live-link traversal execute unchanged.

972 cases include384 gate/option/mode/string combinations,288 numeric-boundary
profiles and300 deterministic randomized cases. Inputs include signed zeros,
NaNs with payload/sign and signaling forms, infinities, limits near86400,
day366 and float-rounded2^31. Option objects cover inline length5, heap length16
and aligned length4096. Parsed doubles are supplied by a controlled CRT endpoint;
synthetic option text does not establish actual text-to-number parsing behavior.

Every service entry compares the complete96-byte object, global and all listener
links; service arguments, full payload/context values and ordered requests match.
Final object/global/links also match. Free effects exercise mode/parent reloads,
engine effects exercise the post-query global reload, and notification effects
exercise retained next/head reloads. All972 cases pass. Default FP rounding is
used; FP status/exception flags and other rounding modes are not compared.

`refresh_image` bridges the recovered caller to constructor refresh services.
An ordinary integration test runs the real child constructor, this refresh,
callback copying/parent-list registration and temporary teardown together, with
explicit provider endpoints. A day366.9 input produces366 while retaining the
parent callback copy. A [972-case combined original constructor-plus-refresh replay](UI_CHILD_REFRESH_COMBINED.md)
now separately verifies their original composition, with leaf providers still
controlled. The ordinary test is not presented as that native evidence.

Corpus stays local at `/private/tmp/openxplane-ui-context-refresh.txt`, replayed
via `OPENXPLANE_UI_CONTEXT_REFRESH_VECTORS`. Executable/assets and corpus are not
published. Source, generator, test and report are published.

## Remaining work

Recover actual engine/context/option lookup/CRT/time conversion/notify bodies
and compare combined constructor/factory behavior. Remaining nested children,
dialog140ec9c10/layout/bindings/singleton/Main Menu construction and teardown are
unfinished. Opening ID3/Alt, live original viewer/GPU output and matched
screenshots/GIF/video remain missing. Esc still exits the viewer. Overall rubric
stays41.4% implemented/28.2% verified; Rendering/UI verified remains0. Whole-game
and visual parity remain unproven.


2026-10-18: [factory with actual root/context refreshes](UI_FACTORY_ROOT_CONTEXT_REFRESH.md) now has118 whole original calls with both refreshed children, shared resource index and full callback/list composition. Context reads88/77 after gating; leaf/CRT and remaining refreshes/providers/full dialog/live GPU remain pending. Estimates unchanged.
