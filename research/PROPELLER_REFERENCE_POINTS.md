# Propeller reference point producer

2026-10-13. Baseline SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.

The original block `0x141a8879b..0x141a88dc6` traverses aircraft+0x920 part/propeller records at +0x6010, stride 0x3770. Type +0 equal to 9 skips a record; type zero or negative does not. For included records it reads element count +0x8c, chooses lower index min(count-1,10), and calls the unmodified value-clamped interpolation helper on the radius array at +0x644 with fixed query 10.0 (`0x14250e578`). The radius is not the maximum of the array and, for nonmonotonic samples, should not be replaced by a guessed geometric extent.

It adds eight corners around stored center +0x778/+0x77c/+0x780. Negative coordinates use center-radius; positive coordinates use radius+center, in float32. Z sign changes fastest, then Y, then X. No radius absolute value or clamp is introduced.

`propeller_bounds_points` reproduces this pass with explicit stored inputs. Its supported finite diagnostic domain is up to eight records and element counts 1..100, with twelve radius samples to include the upper index 11. Type 9 skips validation and processing, just as the original branch skips those reads. Zero included element counts and nonfinite active fields remain outside this API's supported domain.

## Evidence

`tools/gen_propeller_bounds_vectors.py` runs the complete original block with its original interpolation helper, validates reference identity and exact end-range completion, and uses no stubs. Preallocated vector capacity excludes allocator growth. The local corpus has 400 cases with 0..8 records, type 0/1/9/-1, counts 1/2/4/10/11/12/100, nonmonotonic signed radius arrays, translated centers and signed zeros. All **28632 coordinate words** match bit for bit in debug and release on macOS ARM64.

The corpus is local and is not published. The comparison test is explicitly ignored in ordinary CI; a synthetic CI regression checks corner order, type 9 and the distinction between the interpolation result and radius maximum. No reference aircraft, executable bytes or new original-output fixture are distributed.

## Integration limits

Individual gear, wing, body and propeller point producers are now recovered. Complete point-vector assembly, caller inclusion state and original runtime body-grid/propeller radius/center preparation remain unverified; they are not replaced with rendered mesh bounds. Live reference size fields and curvature integration therefore remain incomplete. Other hosts' original-code comparisons, nonfinite/extreme domains, allocator behaviour and complete trajectories are not established. Compatibility estimates remain unchanged.

Reproduce with the supplied baseline EXE and Unicorn:

```sh
python tools/gen_propeller_bounds_vectors.py /path/to/X-Plane.exe > /tmp/propeller_bounds.txt
OPENXPLANE_PROPELLER_BOUNDS_VECTORS=/tmp/propeller_bounds.txt cargo test --offline -p xp-airfoil --test propeller_bounds_local -- --include-ignored --nocapture
OPENXPLANE_PROPELLER_BOUNDS_VECTORS=/tmp/propeller_bounds.txt cargo test --release --offline -p xp-airfoil --test propeller_bounds_local -- --include-ignored --nocapture
```
