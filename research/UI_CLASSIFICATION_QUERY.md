# Native window classification collection and keyboard capture query

Date: 2026-10-13. Baseline SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Collection producer

`0x1412dc3c0(implementation, output_vector, classification)` walks the registry list whose sentinel is implementation+0x150. It skips null node payloads (+0x10) and appends each entry pointer with a signed +0x170 field exactly equal to the requested classification. It preserves list order and duplicate tokens. The vector starts empty and uses native growth/copy helpers.

`WindowState::classified_indices` represents those entry identities by stable indices in the Rust registry. `RegisteredWindow::classification` supplies the raw field; its initialization/lifecycle and semantic names remain unrecovered. Existing menu presence tests use classification 0 because those queries never inspect that field.

## Keyboard capture query

`0x1412e3470` requests classification 2. For each selected entry it resolves the entry's opaque +0x138 token again against the full registry, starting from the first node. The first matching non-null entry supplies widget pointer +0x140. It computes the widget's cumulative parent opacity and returns true on the first strictly positive result; otherwise it returns false. It frees the candidate vector on both exits.

A duplicate token can therefore redirect a classification-2 candidate to an earlier entry with another classification and another opacity. This is preserved in `WindowState::visible_classification(2)`. Tokens equal to zero still participate in this query: it compares identities without the controller-map null rejection used by the menu-kind query. Do not merge these two query semantics.

The source collection guarantees membership in a stable registry. Concurrent removal, dangling pointers and invalid graphs are outside the model. Finite root-to-leaf alpha paths preserve the recovered f32 multiplication order; NaN payload parity is not claimed. Class 2's user-visible meaning is unknown; the name capture describes its position in keyboard routing, not an established modal-window class.

## Original-code evidence

`tools/gen_ui_capture_vectors.py` executes the complete collection producer, visible query, recursive opacity function and vector growth/copy code unchanged. Only allocator/free endpoints (`0x1422e7bf0`, `0x1422e7c2c`) are replaced. 1054 contexts compare exact selected entry indices and query booleans, including null payloads, empty lists, signed classification boundaries, independent token/class fields, duplicate-token rematching, zero tokens, negative/zero/positive opacity, parent chains and early-success order. Allocation failure and very large allocations are not covered.

```sh
python tools/gen_ui_capture_vectors.py /path/to/X-Plane.exe > /private/tmp/openxplane-ui-capture.txt
OPENXPLANE_UI_CAPTURE_VECTORS=/private/tmp/openxplane-ui-capture.txt cargo test --offline -p xp-app --test ui_capture_local -- --ignored
```

The keyboard dispatcher generator now installs actual registries and executes this query instead of its earlier result stub. Its 2152 combined cases pass with only four remaining service endpoints (interceptor, own key handler, Begin, End), plus allocator/free. The Rust replay also calls the recovered window-state query. Combined runs cover two explicit registry constructions; the 1054-context query corpus covers the broader duplicate/order/opacity boundaries. Both corpora remain local, and their tests are ignored in ordinary CI.

The existing 1033-context menu-window query comparison also passes after adding the classification field. This verifies no change to its previously covered behavior, not all runtime windows.

## Next and visual limits

Recover classification assignments and registry lifecycle, the interceptor body, actual action-bearing widget callbacks and the game show_menu handler. Wire the recovered services and stateful routing to the viewer after recovering their inputs/effects. The viewer still exits on Esc and renders no original menus. No screenshot/GIF/video parity or whole-game compatibility is established by these local queries.
