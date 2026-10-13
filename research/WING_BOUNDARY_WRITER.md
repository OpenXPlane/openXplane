# Original wing boundary coordinate writer

Date: 2026-10-13. Baseline executable SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

## Recovered chain

The aircraft geometry pass at `0x14124c880` handles both propeller/part and wing geometry. Its wing section reads the array at aircraft+0x6028, advances by 0x36c8 across 48 slots, and processes enabled wings. The coordinate loop `0x14124e542..0x14124e643` includes ribs 0 through the element count. It divides span by max(count, 1), multiplies by the rib index, applies design sweep/dihedral trigonometry, and adds `_c_off_for_RIB[rib] * chord[rib]` to the longitudinal coordinate.

Each point passes through the complete original helper `0x141294240..0x141294543`. It uses design angles, variable-sweep hinge offset (+0x3c/+0x40), runtime rotation (+0x38), dihedral delta (+0x28), retraction fraction (+0x54), side (+0), and origin (+0x6f0/+0x6f4/+0x6f8). The rotation/delta labels describe the recovered mathematical planes, not a fully recovered control-state mapping. The helper also applies the aircraft curvature fields +0x6070/+0x6074/+0x6078 through `0x1412941d0`, with a strict double 0.001 threshold and a double-precision amplitude*(1-cos) calculation.

The prior live geometry normalized a vector built with tan(sweep) and added an artificial quarter chord to z. For supported counts, it now runs the recovered writer with stored ACF rib chord offsets, chord values and CG-relative origins. The live dynamic angles, variable-sweep offset, retraction and curvature are explicitly fixed to zero until their callers are recovered. Larger rib counts retain the declared legacy approximation. This update feeds the recovered grouping and wing element kernels; the approximate rigid-body model still computes separate force arms, element centres and normals.

## Evidence

`tools/gen_wing_boundary_vectors.py` executes the complete original coordinate loop and point helper, including unmodified native CRT functions. It validates the executable digest, uses a bounded 500000-instruction budget per case, requires exact end-range completion, and asserts there are no stubs. Read-only code hooks record CRT return values; they do not replace instructions or results. The generator also checks that the outer design-angle calls equal their corresponding repeated helper calls. The corpus was reproduced byte for byte after adding these checks.

The corpus has 240 synthetic cases, 1431 points and 4293 coordinate words. It covers counts 0..10, all 48 slot indices, nonlinear chords and offsets, both sides, translated origins, nonzero design/dynamic angles, hinge/retraction state, and disabled/below-threshold/at-threshold/nonzero curvature. No reference aircraft or original instruction bytes are distributed in it. Static evidence comes from local LLVM disassembly; no successful REA decompilation is claimed.

Both debug and release on macOS ARM64 match all 4293 words bit for bit when supplied the recorded original CRT results. The public platform-trigonometry path matches 4292 words exactly; its worst scaled absolute error is 1.7028013e-7. The test enforces a 2e-6 limit per coordinate, scaled by max(abs(original), 1). Recorded-input parity and platform-tolerance parity are reported separately.

Workspace tests: 243 passed, 0 failed, 2 explicitly ignored. Five local Cessna flight tests executed against supplied aircraft. After the final CG-frame/offset integration, the affected xp-acf and xp-app suites were rerun. These are behavioural smoke tests, not original trajectory comparisons.

## Remaining scope

Full geometry equivalence is not established. Remaining work includes the caller's dynamic parameter update schedule, the curvature setup pass and runtime aircraft reference bounds, enabled/hidden runtime gates, element centre/normal/force-arm writers and other geometry passes. The zero dynamic-state live integration does not validate articulated aircraft. Platform transcendental equality outside the corpus, nonfinite/extreme angles, counts above ten and whole-simulator trajectories remain unproven.

Reproduction (Unicorn and the local baseline executable required):

```sh
python tools/gen_wing_boundary_vectors.py /path/to/X-Plane.exe > /tmp/wing_boundary.txt
cmp /tmp/wing_boundary.txt crates/xp-airfoil/tests/data/wing_boundary.txt
cargo test --offline -p xp-airfoil --test wing_boundary_exact -- --nocapture
cargo test --release --offline -p xp-airfoil --test wing_boundary_exact -- --nocapture
```
