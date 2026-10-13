# Native selection and keyboard-focus transition

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Complete transition

Function `0x1412f7fe0` receives a widget pointer and a 32-bit mode. The selected pointer is global `0x145a0def0`, its mode is `0x145a0def8`, and the keyboard-focused pointer is `0x145a0dfc8` (the same global read by the recovered input wrapper).

If the requested pointer equals selected, the function returns without changing mode or focus. Otherwise it caches whether this is mode 0→1 or 1→0. For a non-null previous selection it calls `0x1412e4680(previous, byte 1)`, except on 0→1. Only after this notification does it read keyboard focus. If focus differs from the requested pointer, it follows the focused object's +0x10 chain to its last non-null element and calls that element's virtual +0x40. Then it stores the requested keyboard focus. Finally it stores the selected pointer and requested mode, and calls `0x1412e47a0(requested, byte 1)` for a non-null target, except on 1→0.

The mode comparisons are exact equality, not truthiness. No higher-level meaning is assigned to mode 0/1. `Leave`, `Blur` and `Enter` in the Rust port name notification endpoints, not established complete UI effects. Leave and enter helper bodies include recursive widget/listener processing and are not yet ported. Selection hit-testing and construction of the widget graph are not established by this function.

`xp-app::ui_focus::FocusState::select` reproduces this order. Notification callbacks can mutate the three state fields: later focus reads see leave changes, stores overwrite blur changes, and the skip-enter decision retains the initially cached mode. The supplied leaf resolver represents native traversal of an acyclic +0x10 chain. Cyclic/dangling graph handling, concrete helper bodies, destruction and OS focus are outside this port.

## Exact local comparison

`tools/gen_ui_focus_vectors.py` executes the entire native transition unchanged. Only the leave/enter service bodies and leaf virtual +0x40 endpoint are replaced with logging callbacks. Chains have 1–3 elements. 2024 cases compare all three final state fields and ordered notifications, including the state observed at each callback. The deterministic matrix covers null/equal/different pointers, independent selected/focused pointers, modes 0/1/2/0xffffffff and all their transitions. Seeded cases additionally mutate focus, selection and mode inside leave/blur endpoints.

```sh
python tools/gen_ui_focus_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-focus.txt
OPENXPLANE_UI_FOCUS_VECTORS=/private/tmp/openxplane-ui-focus.txt cargo test --offline -p xp-app --test ui_focus_local -- --ignored
```

The corpus remains local and the comparison is opt-in, ignored in ordinary CI. This proves the transition with explicit endpoint effects, not the complete focus lifecycle. It is not yet connected to the viewer and supplies no visual evidence.

## Menu handler observation

The observed main-menu button vtable `0x142608980` has input dispatch slot +0x50 = `0x1412ee8a0`, but its own input callback +0x38 = `0x1406d4ba0`. That callback is `xor al, al; ret`: it declines input. Therefore the next search must follow actual child elements and their callbacks; assigning Enter/Esc effects to this button would invent behavior. The base vtable `0x142674610` has a different +0x38 target (`0x1422e8f5c`); it is not evidence of a concrete menu action.

Next: recover leave/enter helper effects and focus selection callers, follow the menu button's real child construction, then connect the verified state/input behavior to original resources and rendering. Alt activation, show_menu/Esc action, menu layout and paired screenshots/GIF/video remain unverified.
