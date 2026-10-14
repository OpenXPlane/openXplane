# Concrete menu fallback request flow and property-2 gate

Date: 2026-10-14. EXE SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Actual concrete property and gate

Panel vtable `0x1426064f0` own Esc/Enter handler requests fallback slot +0x170,
`0x140f00310` (see `UI_MAIN_MENU_KEY.md`). The fallback first queries property 2
through panel+0x28 interface slot +0x10. Its actual table `0x1426066b0` resolves
to `0x140efa740`. **For this table** property 2 returns float 0 iff engine byte
at `0x142f2d770+0x525` is zero and enum at `0x145a12118+0x18` equals 2; otherwise
float 1. The native byte getter `0x1417fb320` and enum predicate `0x1414ca5d0`
are pure. This property is not treated as opacity. The generic fallback contains
an unordered-NaN path, but this concrete property produces only 0/1 and the port
makes no claim for other virtual tables/NaN providers.

Property 0 returns immediately; the caller's Esc own handler still consumes the
event. Otherwise the fallback rereads engine byte. Nonzero invokes panel+0x170
interface slot +0x08 with int 0. Original thunk `0x140ef6f10` adjusts this by
-0x170 and calls `0x140f0a2c0(panel,0)`. Its full control flow and scalar guards now have 694 direct native comparisons
(`UI_MENU_DIRECT.md`); actual window/document/dialog callee effects remain
unported.

With zero engine byte, query A (`0x140f0a1b0`) runs. Only a nonzero result requests
query B (`0x1417e2640` on `0x1460fc940`); if B is nonzero the same direct action
runs. Otherwise A runs **again**, without caching the first result. The second
nonzero result submits a null result; the second zero result constructs/resolves
a temporary object before submission. Query names A/B deliberately do not imply
a pending/unsaved condition: their full meanings and bodies are unverified.
Service mutation after the first A does not repeat the earlier engine gate.

## Temporary-object requests

The second-A-zero branch requests allocation 0x320, constructs two empty
wrappers with actual helper `0x1410c7d20` (RDX/R8/R9 all zero), then requests
`0x140ec9c10(allocation,null,second,first)`. It cleans second wrapper before first
through `0x1410c8430`; their original empty-tree allocation/cleanup bodies run in
the native replay. Wrapper +0x10/+0x18/+0x20/+0x60 are asserted zero before the
controlled constructor consumes them. No wrapper callback is invoked.

The caller then requests manager `0x1417e9f50`, input conversion `0x140d569e0`
from temporary object+0x2d0, resolution `0x1417e8a60(manager,input)`, and object
virtual destruction +0x0 with flag 1. It submits pointer-to-int32 0 and the
resolved handle at panel+0x170 to `0x140ef67c0`. The destruction precedes submission.
A null resolved handle is permitted. This sequence is reproduced by
`ui_menu_action::dispatch`; actual UI constructor/resolver/destructor/submission
and query effects remain explicit services, not guessed state changes.

## Direct and joined original-code comparisons

`tools/gen_ui_menu_action_vectors.py` executes the complete fallback,
original property2/engine/enum query bodies, actual direct thunk, and empty
wrapper constructors/destructors unchanged. Query A/B, original UI construction,
manager/conversion/resolution/destruction/submission and allocator bodies are
controlled endpoints. Three complete routes run for each case:

1. direct fallback (return registers are not compared; zero placeholder only);
2. original menu own Esc callback with +0x169 byte zero, delegating to the actual
   fallback without its earlier whole-function logger;
3. original focused wrapper/base/filter and own Esc callback, with explicit
   panel focus, no child and mask 0xfff, delegating to the actual fallback.

746 cases / 2238 native executions compare exact own/focused return, final engine
byte and ordered query/allocation/wrapper/construction/cleanup/manager/conversion/
resolution/destruction/submission requests. Cases cover raw engine bytes,
signed enum boundaries, independent raw A/B results, changed engine after A,
short-circuit order, null/arbitrary uint64 resolved handles and all reached
branches. A normal regression checks A=0 then A=1 despite changing engine to 255,
followed by complete property2 suppression at enum 2. Native comparison is
opt-in/ignored in normal CI. Original artifacts and numeric corpus stay local.

```sh
python tools/gen_ui_menu_action_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-menu-action.txt
OPENXPLANE_UI_MENU_ACTION_VECTORS=/private/tmp/openxplane-ui-menu-action.txt cargo test --offline -p xp-app --test ui_menu_action_local -- --include-ignored
```

## Remaining action/visual acceptance

Allocation and construction succeed, wrapper roots are empty, and synthetic
service changes are limited to the declared engine-byte update. Rust rejects
null allocation/constructor results instead of inventing the native null-memory
behavior; failures/exceptions/unwind, populated wrapper lifetime, real UI/cache/
manager content, focus/child graph and concrete actions are unverified. The port
currently establishes request control flow, not opening/closing/resuming a menu.

Direct `0x140f0a2c0` now has recovered request control flow. Next recover its
close/preparation/document/binding effects, submission `0x140ef67c0`, constructor
`0x140ec9c10`, query/manager/cache bodies, kind-11 creation/registry lifecycle and
actual show_menu ID-3/Alt entry. Original layout/fonts/GPU rendering and matched
screenshots/GIF/video remain outstanding. The active viewer still exits on Esc
and does not draw original menus; whole-game X-Plane 12 1:1 remains unproven.
