# Combined main reference-geometry pipeline

2026-10-13. Baseline SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

`reference_geometry_bounds` assembles the main point vector in original source order: ten gear slots, up to 48 wings, up to 39 bodies, then the counted propeller records. Disabled slots produce no points. Each producer retains its original point order and duplicates. The combined vector is passed to the recovered origin-inclusive bounds reducer. Inputs must be stored runtime geometry; this function does not prepare raw ACF grids or update articulated surfaces.

## One original execution, rather than concatenated expectations

`tools/gen_reference_geometry_vectors.py` executes the entire original range `0x141a87e30..0x141a8923e`, including all four point passes, intervening auxiliary numerical calculations and final main bounds. It requires the exact end-range PC and a bounded 20-million-instruction budget. The initial 2-million budget proved insufficient in an internal helper; incomplete results were rejected. All 80 final replays completed after increasing the budget.

The internal auxiliary-vector helper allocates temporary storage. Only original operator new (`0x1422e7bf0`) and delete (`0x1422e7c2c`) are replaced with scratch allocation and no-op release. All geometry/interpolation/auxiliary numerical helpers and bounds arithmetic execute their original instructions. The final corpus records 158 allocation and 158 release calls. This is explicitly an allocator-controlled replay, not a no-stub native run. Main vector capacity is preallocated; allocator failure/growth behaviour is not tested.

The 80 synthetic cases vary gear types and fields, enabled wing slots and 1/4/10 element boundaries, body slots and station/ring counts, propeller counts/types/radii and translated nonmonotonic coordinates. They include an entirely empty vector and high-index slots. Every original main point word, including ordering, is compared: **22350 exact point words**. All **800 main bounds words** match exactly, including min/max, planar halfspan and three reference size fields. Debug and release comparisons pass on macOS ARM64 without tolerances.

The local corpus is not committed. The comparison is explicitly ignored in ordinary CI; the CI regression verifies empty-pipeline origin inclusion. Missing corpus environment/input fails an explicitly requested comparison. No aircraft files or original executable bytes are published.

## Scope limits

The auxiliary aircraft object vector at +0x60e0/+0x60e8 is empty in these replays. Original auxiliary numerical helper output fields are not asserted by the Rust comparison; only the main point vector and ten final main bounds fields are verified. Real-aircraft runtime grid/radius/center preparation, enabled/hidden state scheduling, curvature feedback, auxiliary fields and complete geometry updates remain unfinished. No real-aircraft reference snapshot, live replacement of constant reference fields or full-trajectory parity is established. Compatibility shares remain unchanged until those broader inputs/integration are verified.

Reproduce with Unicorn and the supplied baseline:

```sh
python tools/gen_reference_geometry_vectors.py /path/to/X-Plane.exe > /tmp/reference_geometry.txt
OPENXPLANE_REFERENCE_GEOMETRY_VECTORS=/tmp/reference_geometry.txt cargo test --offline -p xp-airfoil --test reference_geometry_local -- --include-ignored --nocapture
OPENXPLANE_REFERENCE_GEOMETRY_VECTORS=/tmp/reference_geometry.txt cargo test --release --offline -p xp-airfoil --test reference_geometry_local -- --include-ignored --nocapture
```
