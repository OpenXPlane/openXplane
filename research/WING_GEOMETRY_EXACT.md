# Exact quarter-chord sweep and delta-wing weight corpus

Date: 2026-10-12. Baseline executable SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The existing delta-wing helper used platform float32 tan, and its older test allowed up to 64 ULP.
A larger original-code corpus exposed a 3-ULP delta-weight difference (fixture line 2337).
The radians are still computed with the original float32 multiplication; tangent is now evaluated
in float64 and rounded once to float32. This corrects the exposed difference on the tested host.
The surrounding original operation order and the sweep computation are unchanged.

`crates/xp-airfoil/tests/wing_geometry_exact.rs` requires exact output bits, without a tolerance.
**4062 geometries, 8124 outputs** of original helpers `0x1411a0230` and `0x1411a00f0` match
in debug and optimized release on macOS ARM64. Other hosts are covered by CI; universal equality
of transcendental implementations outside this corpus is not established.

## Evidence and boundaries

- Generator executes both complete original functions, including original CRT atan2 and tanf.
- No stubs, replaced instructions, runtime imports or copied original code bytes in the fixture.
- It checks the executable digest and asserts the original CRT selector `0x1463a5090` is unchanged
  (0 in this baseline). Output is not obtained by recomputing the Rust formula in Python.
- Inputs contain both signs of sweep and adjacent float32 coordinates around angle thresholds
  15/40/50/75 degrees, zero transverse radius, signed zeros, unequal chord endpoints, translated
  and scaled coordinates, varied raw wing fields +0x18/+0x1c and 2500 deterministic random cases.
- Field names aspect/taper are inferred from the formula; their writers are not verified here.
- Source disassembly is the saved original helper listing; REA/Hopper launcher did not reach bridge
  readiness on this host. No successful REA decompilation is claimed.

## Reproduction

```sh
python tools/gen_exact_wing_geometry_vectors.py /path/to/X-Plane.exe > /tmp/wing_geometry.txt
cmp /tmp/wing_geometry.txt crates/xp-airfoil/tests/data/wing_geometry.txt
cargo test --offline -p xp-airfoil --test wing_geometry_exact
cargo test --release --offline -p xp-airfoil --test wing_geometry_exact
```

The generator requires Unicorn and the locally supplied baseline EXE; neither is a runtime
dependency of the engine. The original-code corpus was also reproduced using the current
upstream emulator and compared byte for byte. This is numerical helper coverage, not a
claim that complete aircraft trajectories or the full game are identical.
