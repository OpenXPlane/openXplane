# Aircraft curvature setup kernel

2026-10-13. Reference SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The recovered block `0x14124df67..0x14124e036` clears aircraft+0x6070/+0x6074. When abs(aircraft+0x28fc), the loader field for `acf/_ball_ang_def_deg`, exceeds double 0.001, it converts degrees using float32 0x3c8efa36. It combines original float32 sin/cos of that angle with double sin/cos of twice the widened angle, then computes atan2 of the summed components. The result is rounded to float32 before doubling through double precision. It stores that angle, copies the complete bounds writer's +0x64f4 halfspan to +0x6078, and stores halfspan/angle at +0x6074. Inactive inputs preserve the old +0x6078 value. These fields feed the already recovered point-curvature transform. The curvature name describes the formula, not an independently established semantic interpretation of the ACF property name.

`wing_boundary::curvature_parameters` preserves the precision boundaries and branch order. It uses platform trigonometry and atan2; no claim of universal bit equality is made. Counts and geometry dimensions are not inferred from this helper.

## Local evidence

`tools/gen_wing_curvature_vectors.py` executes the unmodified original block, including original CRT functions, with no stubs. It checks reference identity, exact end-range completion and a bounded 500000-instruction budget per case. Inputs and outputs are synthetic numeric state; the corpus stays local and is not checked into this repository.

The local corpus contains 521 cases: signed zeros, adjacent float32 values around both signs of the double 0.001 degree threshold, varied halfspan including zero, previous-scale preservation, representative positive/negative and wrapping angles, and 500 deterministic random angles in [-720,720]. On macOS ARM64, 1562 of 1563 output words match exactly; one differs through platform mathematics. Worst scaled error is 8.907767e-8; the local test requires <=2e-6 scaled by max(abs(original),1). Debug and release checks are run separately. An ordinary CI run explicitly ignores this original-code comparison because the local corpus is unavailable; a synthetic regression checks disabled-branch preservation and the float32-versus-double threshold distinction.

Reproduce locally with the baseline EXE and Unicorn:

```sh
python tools/gen_wing_curvature_vectors.py /path/to/X-Plane.exe > /tmp/wing_curvature.txt
OPENXPLANE_CURVATURE_VECTORS=/tmp/wing_curvature.txt cargo test --offline -p xp-airfoil --test wing_curvature_local -- --ignored --nocapture
OPENXPLANE_CURVATURE_VECTORS=/tmp/wing_curvature.txt cargo test --release --offline -p xp-airfoil --test wing_curvature_local -- --ignored --nocapture
```

## Integration limits

The viewer still supplies zero curvature. Connecting this setup requires the original complete reference halfspan, which includes more geometry than currently recovered wing points. This change does not substitute an approximate halfspan or establish complete aircraft bounds, dynamic geometry scheduling, nonfinite CRT behaviour, other hosts' original-code comparisons or trajectory parity. Compatibility estimates remain unchanged until broader integration evidence exists.
