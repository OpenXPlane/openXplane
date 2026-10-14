# Direct menu-panel action: guard, kind-11 close and document/dialog requests

Date: 2026-10-14. EXE SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Full original action control flow

The fallback's direct branch reaches original `0x140f0a2c0..0x140f0a53e` through
thunk `0x140ef6f10`; see `UI_MENU_ACTION.md`. Panel byte +0x2c9 is a re-entry
guard: nonzero returns immediately without changing it. Otherwise native stores
1 before virtual preparation +0x118, obtains controller `0x14150d020`, rereads
panel+0x2c8, and requests `0x14150ce20(controller,11,flag_c8==0)`. The close flag
is normalized 0/1 even for raw noncanonical panel bytes.

After that close request, further processing requires argument !=12, global UI
mode +0x18 at `0x145a12118` equal to 1 (actual predicate `0x1414ca5e0`), and signed
argument <=0. Mode is queried **after** close; callback changes affect the branch.
The common successful epilogue clears the current +0x2c9 byte. Native null/error/
exception paths and return-register contract are not inferred from that epilogue.

Further processing calls `0x140658af0` with constant `0x142609340`, then query A
`0x140f0a1b0`. Its nonzero result requests owned document `0x140efe0d0`, builds
string_view for `0x140efdda0`, and passes the returned raw validation byte to
`0x1417fa7a0(global 0x142f2d770,owned document,validation)`. A zero load byte calls
`0x140f0a210`. String cleanup precedes testing the captured load result. These
service names document the observed data flow; their actual format/effects and
query semantics remain unverified.

Missing or failed load allocates 0x320 and constructs two wrappers through
`0x1410c7d20`, bound to panel+0x48 with values 0 then 3 (fourth argument zero).
Native binding helper `0x1410c8de0` is called; its actual observer/list effects
remain a controlled boundary. Constructor `0x140ec9c10` receives allocation,
current panel+0x1a8 source, second wrapper and first wrapper. Wrapper cleanup is
second then first. Numeric wrapper value 3 is not proof of show_menu command
registration.

The action stores global byte `0x1457770c6` =1 before requesting
`0x1411103e0(object+0x2c8 pointer)`, `0x1410c9d10(object+0x260)`,
`0x14111bb90(object+0x2d8 pointer)`, then virtual destruction +0x0 with flag 1.
The exact effects of these component services are not guessed.

`xp-app::ui_menu_direct` now ports all these decisions, rereads, normalized/raw
byte handling, successful state stores and ordered requests on explicit state.
Services can mutate preparation/close state; scalar post-call reads observe it.
Nonzero guard suppresses all requests and retains its raw value. The source
rejects null allocation/constructor results instead of claiming unsafe native
null memory behavior.

## Original-code comparison

`tools/gen_ui_menu_direct_vectors.py` executes the complete native direct action,
original enum predicate and wrapper constructors/destructors unchanged. It
replaces preparation, controller/close, query/document/validation/load, nonempty
binding, UI construction/component/destruction and allocator service bodies.
The owned wrapper roots therefore remain empty after the controlled binding;
actual populated listener lifetime is not claimed. Constructor arguments and
stored wrapper values 0/3 are asserted, and nonmutating hooks observe wrapper
creation/cleanup and successful document cleanup.

An optional synthetic preparation thunk calls the actual native direct action
again on the same stack. Its already-set guard causes the recursive entry to
return without further requests. The synthetic code cache is invalidated after
rewrites. No original instruction or branch is rewritten and no nested Python
Emulator.call is used inside callbacks.

694 scenarios compare final raw guard/c8/global byte, signed UI mode word and
complete ordered request traces. Inputs cover nonzero guard preservation,
argument 12, positive/zero/negative/extreme signed arguments, mode changes during
close, c8 changes during preparation, raw query/validation/load bytes, load
success/failure, inline documents of lengths 0/1/8/15, opaque uint64 source values,
initial global bytes, and guarded native re-entry. Observed paths include 356
kind-11 close requests, 44 document/load requests, 5 load-failed requests, and 13
construction requests. Final guard clearing and global flag-before-component
calls are checked explicitly. An ordinary regression preserves recursive guard
suppression, c8 rereading, post-close mode selection and noncanonical guard
retention. Native comparison is opt-in/ignored in normal CI; its numeric corpus
and original artifacts stay local. xp-app clippy passes locally.

```sh
python tools/gen_ui_menu_direct_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-menu-direct.txt
OPENXPLANE_UI_MENU_DIRECT_VECTORS=/private/tmp/openxplane-ui-menu-direct.txt cargo test --offline -p xp-app --test ui_menu_direct_local -- --include-ignored
```

## Close helper and remaining acceptance

Static inspection of full `0x14150ce20..0x14150cf7d` confirms the requested close
helper queries the recovered window registry/opacity, calls window services,
optionally clears the controller kind-map value for kinds listed at controller
+0x90..0x98, and calls final `0x14150fee0` even when its initial query is false.
That helper body is still controlled in the direct replay; clearing/hiding actual
windows is not proven by the close request. No pixel or running-menu outcome is
claimed.

Next port concrete close/registry/map effects, preparation, document/query/load,
binding/constructor/component and submission bodies and join them with the
fallback/own/focused routing. Allocation succeeds, strings are inline, and
source handles are compared only as forwarded opaque values; heap-string/error/
unwind, actual controller/window/observer lifetime, live focus/child graph and
recursive operations beyond the guarded preparation are unverified. Actual
show_menu ID-3/Alt entry, original layout/fonts/GPU renderer and paired screenshots/
GIF/video remain outstanding. The viewer still exits on Esc and draws no original
menu; whole-game X-Plane 12 1:1 remains unproven.
