# Complete keyboard entry gates and focused-input integration

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Native control flow

`0x140c627a0` receives a binding-vector owner, signed character, full virtual-key word and event flags. It clamps signed characters greater than 127 to zero; negative characters remain unchanged. It calls manager query `0x1412e3470` on global `0x145a0e308`.

If that query returns a nonzero byte, the function sends the event directly to focused UI via `0x1412c6300`. If it returns zero, the function first calls `0x141913800(character_byte, flags, key_byte, phase 0, stack 0)`. A zero interceptor result stops the event. A nonzero result proceeds to focused UI; nonzero focused-UI return stops the event. If UI declines, a second interceptor call with phase 2 runs; zero stops the event. The nonzero-query path skips both interceptor calls.

After these calls, the function reads the mode through pointer global `0x142f01eb0` and the byte at `0x145a0df02`. Exact mode 1000, virtual key 0, or any nonzero block byte stops command routing. These globals are read after callbacks, so callback changes affect the gate. Otherwise the function executes the recovered binding loop `0x140c62868..0x140c628ea` and issues ordered Begin/End requests.

The UI path preserves the full character/key/flag words. Only interceptor arguments truncate the character and virtual key to unsigned bytes. Native negative interceptor returns count as nonzero. This preserves distinctions that would be lost by using one shared byte input for all stages.

`xp-app::keyboard_dispatch` connects the service boundaries to `xp-dataref::key_event::route`. It accepts an implementation of the query/interceptor/focused-input services, rather than inventing their runtime results. Begin/End callbacks remain inert request outputs. No viewer integration or command effects are implied.

## Query and interceptor boundaries

Static inspection of `0x1412e3470` shows collection helper `0x1412dc3c0` with argument 2, window registry matching by token +0x138, and positive cumulative opacity of registered widgets. The collection producer is now recovered as an exact signed +0x170 field filter (see `UI_CLASSIFICATION_QUERY.md`); `has_visible_capture` remains a service label, and no particular modal/menu meaning is assigned to class 2. The interceptor wrapper and observed registered adapter are now recovered (`KEY_INTERCEPT_WRAPPER.md`); external export/context bodies and full module/registration lifecycle remain unrecovered. Recover these bodies before treating the service implementation as original behavior.

## Combined local evidence

`tools/gen_keyboard_dispatch_vectors.py` executes the complete EXE entry, native manager query/collection, interceptor wrapper/adapter, focused wrapper/filter/initial mapped CRT and binding loop unchanged. The paired replay also calls the real Windows DLL export/body. Nonmutating hooks record boundary arguments. Plugin sniffer bodies, the own widget callback and Begin/End effects remain logging endpoints, with controlled validation/clock/context/allocator services. Sniffer results and mutations are explicit inputs; DLL interceptor results are computed by original code. See `KEYBOARD_PAIRED_REPLAY.md` for the exact synthetic registration and module limits.

2152 cases compare exact callback order and arguments, all active fields, release suppression, and final mode/block state. A 1152-case matrix covers both query paths, interceptor outcomes, raw own-handler results, named/zero keys, acceptance masks, mode gates, block bytes and signed-character clamping. Seeded cases add arbitrary flags, full-word keys (including a byte alias), missing focus, suppression, negative interceptor returns and mode/block mutations inside interceptor/own handlers. Characters below -1 are tested only without focus or with the named Escape shortcut, which bypasses the unsupported CRT classification domain. This does not extend the earlier CRT locale/input guarantee.

```sh
python tools/gen_keyboard_dispatch_vectors.py /path/to/X-Plane.exe /path/to/XPLM_64.dll > /private/tmp/openxplane-keyboard-dispatch.txt
OPENXPLANE_KEYBOARD_DISPATCH_VECTORS=/private/tmp/openxplane-keyboard-dispatch.txt cargo test --offline -p xp-app --test keyboard_dispatch_local -- --ignored
```

The corpus remains local; the comparison is ignored in ordinary CI. All 2152 individual comparisons passed; an initial count assertion was corrected from an incorrectly estimated 4456 to the actual generated 2152 cases. The first broader input attempt also reached unsupported CRT inputs in the Rust filter; the retained corpus explicitly constrains those inputs as described above.

## Remaining whole-input and visual work

The live viewer still uses its independent event lifecycle and exits on Esc. The new dispatcher is not wired to it. Actual UI graph, classification assignments and registry lifecycle, external export/context effects, command callback effects, game show_menu action and Alt activation remain unverified. Main-menu button and observed constructed layout widget vtables both use the false own-key callback `0x1406d4ba0`; the next handler search must follow actual focus graph and action-bearing elements, not infer keyboard actions from visual nesting.

Next: recover classification assignments, registry lifecycle and external export/context bodies and actual widget callbacks, connect the verified services/stateful dispatch to the live UI graph, then recover layout/font roles and rendering. Paired original/openXplane screenshots and GIF/video at identical conditions remain required for visual fidelity; these control-flow comparisons are not visual evidence.
