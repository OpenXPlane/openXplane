# Whole fresh root constructor with actual refreshes

Reference X-Plane.exe12.4.3.11, SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

118 original calls enter1410d6a40 with a poisoned0x3e0 object and execute to
normal return, asserting RAX equals the object identity. No preparatory constructor
or separate refresh call is used. Both nested14113a300 calls, final root141138cf0,
payload1410faec0/string1405de290/vector1410fa780 helpers and callback/list bodies
execute in the same original run. Leaf computations remain controlled as below.

## State transfer

The root constructor supplies its newly created callback graph and receiver
lists to final refresh through refresh_root_with_context. Existing controlled
providers keep their context-free default implementation. Final refresh publishes
next/previous receiver links when their sentinel is mapped; opaque callback RB
node storage remains represented by the logical callback graph.

The replay host shares object and constructor blocks with the existing root/
nested/payload memory adapter. All seven constructor allocations are checked at
entry and after allocation (sizes120,32,120,72,72,120,40), including allocation
poison. Six blocks are compared as raw bytes. The40-byte callback sentinel's
internal RB backing remains opaque, as do later callback RB allocations and the
payload of retained receiver entries; their16-byte next/previous links and
logical callback peers are checked. This does not claim native callback-tree ABI.

Reference stack identities are checked at the actual constructor and root-refresh
prologues. The constructor's first nested temporary is S+0x1e0 and the final
refresh's nested temporary is S-0x100, where S is the final root frame. Initial
scratch and later live leaf operand reads/writes are checked. Incidental machine
stack spills are excluded from snapshot parity.

## Evidence and limits

The118 cases retain the root/config/resource provider matrix. Eight full-width
constructor flags include0/1/128/255/256/257 and two large values, checking the
original low-byte behavior. Fresh construction always clears the bound flag;
each final refresh registers one retained callback. Lists start empty, so existing
standalone refresh evidence still supplies nonempty/mutating-list coverage.

There are826 constructor allocations,118 retained receiver entries and354 engine
checks (first nested, root, final nested).68 settings calls and34 temporaries reach
actual payload assignment, with inline optional strings and vectors of lengths
0/2/8. No payload heap growth/release is exercised by this composition; separate
payload evidence covers those paths. Fresh receiver lists produce no listener
notifications and fresh item vectors produce no item releases. Provider-selected
root vectors/string ownership branches still execute, including heap/aligned
root string profiles.

Engine/resource/config/registry/options/settings lookups, record parsing/application/
destruction, root string/vector effect bodies, nested resource assignment,
temporary construction/optional destruction, enumeration, release and notification
endpoints remain controlled. Memcpy and allocator/free endpoints are controlled.
This comparison proves caller ordering and state transfer over those endpoints,
not their missing calculations, error/exception behavior or full runtime state.

Generate locally:

```sh
python tools/gen_ui_root_refresh_vectors.py /path/to/X-Plane.exe --construct > /tmp/root-constructor.txt
OPENXPLANE_UI_ROOT_CONSTRUCT_CHAIN_VECTORS=/tmp/root-constructor.txt cargo test --offline -p xp-app --test ui_root_refresh_local native_whole_root_constructor -- --ignored
```

The corpus and original bytes/assets are not published. The earlier independent
root118 rows and root/nested/payload118 rows regenerate byte-for-byte unchanged.
Both earlier192-case root constructor replays also pass after the context change.

Next compare the complete child factory/dialog and recover remaining providers,
layout/widget bindings/singleton/Main Menu and teardown. These modules are not
wired into original live menu rendering. Alt/Esc behavior, GPU pixels and paired
screenshots/GIF/video remain unverified. Compatibility estimates reviewed and
unchanged:41.4% implemented/28.2% verified overall, Rendering/UI verified0.

Validation:204 ordinary tests pass;57 opt-in tests are ignored in the ordinary
run. All three118-case refresh/constructor replays pass when explicitly enabled;
both earlier192-case constructor replays pass too. Formatting, Python syntax and
diff checks pass. No live viewer/GPU run or paired visual capture was performed.


2026-10-17: [whole all-child factory composition](UI_FACTORY_COMPOSED.md) now has216 native runs with every constructor body, shared receivers and logical callbacks. Refresh/provider computations remain controlled here; composition with actual recovered refreshes and full dialog/live GPU is next. Estimates unchanged.
