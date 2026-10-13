# Native focused-element keyboard input

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Entry and order

`0x1412c6300` reads the focused-element pointer at `0x145a0dfc8`. A null pointer returns zero. Otherwise it forwards the raw character, virtual-key and event-flag words to virtual slot +0x50. The base element vtable `0x142674610` and observed menu-button vtable `0x142608980` select `0x1412ee8a0` for that slot.

The base dispatcher first calls the non-null child at element+0x10 through its slot +0x50. A nonzero child byte result becomes canonical 1 and immediately returns. If the child is absent or declines, the dispatcher calls acceptance filter `0x1412e5a90` on the original element. If accepted, it invokes the original element's virtual slot +0x38 and preserves that callback's raw byte return, including noncanonical values.

The acceptance mask at element+0x18 is read after the child callback; a child can therefore change which keys the parent accepts. The Rust dispatcher uses a mask accessor at the same point, rather than capturing a stale mask before invoking the child.

## Acceptance mask

Named virtual keys take precedence over character classification:

| Keys | Accepted mask bits |
|---|---|
| Tab | 0x1 |
| Return, 0xbb, 0xbc | 0x2 |
| Backspace, Delete | 0x4 |
| Escape | 0x8 |
| PageUp/PageDown/End/Home/arrows | 0x10 |
| ASCII letters | 0x20 |
| ASCII digits | 0x40 |
| Plus character | 0x80 |
| Minus character | 0x2080 (either bit) |
| Space character | 0x100 |
| Other printable characters or character >= 128 | 0x200 |
| Control character with Ctrl+C/V/X virtual key | 0x400 |
| Remaining classifier inputs | 0x800 |

The two-level native jump table starts at `0x1412e5bc0`/`0x1412e5bd8` and covers virtual keys 8..188. Printable classification calls the baseline CRT function `0x142309e80` with mask 0x157. The original function and its mapped initial classification globals execute unchanged in the local replay. Runtime locale changes and Windows invalid-parameter behavior are not reproduced. For non-special keys, the port supports characters -1 (EOF sentinel) through 255; unsupported classifier inputs return an error. Named-key shortcuts do not invoke character classification.

## Original-code replay

`tools/gen_ui_input_vectors.py` executes the native wrapper, base dispatcher, complete acceptance filter and CRT. Only the two virtual handler endpoints are replaced by logging stubs. Those verify every forwarded input word. The child endpoint can optionally change the actual native parent mask, so the comparison exercises post-child mask reads. Graph/vtable replacement is not modeled.

7120 cases compare exact returned bytes, final mask words and ordered child/own callback calls. They include all reviewed named-key groups, every individual low acceptance bit, zero/full masks, control/printable/high-byte/EOF character samples, random event flags, missing focus/child, noncanonical callback bytes and child mask mutation.

```sh
python tools/gen_ui_input_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-input.txt
OPENXPLANE_UI_INPUT_VECTORS=/private/tmp/openxplane-ui-input.txt cargo test --offline -p xp-app --test ui_input_local -- --ignored
```

The original-output corpus stays local; the comparison is explicitly ignored in ordinary CI. This verifies the dispatcher/filter under explicit callback outcomes and mask changes, not the concrete widget handlers, native focus selection, full graph lifetime or OS input. `xp-app::ui_input` is not yet connected to the viewer.

## Alt/Esc and visual limits

Escape is admitted by a widget's 0x8 acceptance bit; this does not identify the callback's action or replace the separate game command show_menu (ID 3). Alt-only menu-bar activation still needs recovery. The active viewer still exits on Esc.

No screenshot or motion capture verifies any original menu yet. The final fidelity protocol requires paired original/openXplane images and GIF/video at matched resolution, DPI, state and timing. Logic replays do not count as visual evidence. Next: recover widget mask assignment and slot +0x38 callbacks for actual menu/window elements, real focus selection and the remaining window action bodies; then connect input and original resources to the renderer.
