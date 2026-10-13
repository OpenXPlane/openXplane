# Original grouped-wing dimension writer

Reference SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Kernel `0x14117285b..0x141172bc6` inside `0x141170d40` writes Wing+0x18 at
`0x141172bc1`, Wing+0x1c at `0x141172b71`, and endpoint length at +0x36a8.
It accepts an explicit pointer list of member wings, outer endpoint coordinates
and the previously accumulated group area. `wing_dimensions` exposes the same
numerical inputs without claiming the upstream grouping pass has been recovered.

For every member element the area is float32 of the double product of mean chord,
span and sweep cosine, divided by element count. Its x/y center comes from the
original clamped interpolation helper. The radial distance from the root endpoint
is weighted by element area and accumulated in float32. The mean divides by the
area sum floored at 0.01; dividing by the root-to-tip projected distance (also
floored at 0.01) gives a centroid fraction. Wing+0x1c is the clamped ramp of this
fraction from float32 0.33 with float32 gain 5.88235330581665.

Wing+0x18 uses the full 3D endpoint length. The original computes asin of the z
fraction, converts to degrees then back to radians, takes cosine and multiplies
by length; doubling and squaring gives projected full span squared. It divides
by twice the explicit group area and floors the result at 0.5. Algebraically
replacing the asin/cos sequence would change rounding, so the port preserves it.

A span-10 half-taper wing with chords 6 to 3 gives shape 1.0 for one element,
0.69362735748 for four, and 0.67647051811 for ten. A rectangle gives 1.0 for all
three counts; a triangular wing gives 1.0, 0.08088228106 and 0.02941156365.
Therefore assigning raw chord taper to +0x1c would not reproduce the writer.

`tools/gen_wing_dimensions_vectors.py` runs original code without stubs, with
exact intermediate/end guards. 635 cases include one/two/three-member groups,
rectangle/trapezoid/triangle shapes, counts 1/4/10 and randomized deformed geometry.
Passive hooks record original member cosines and final projection cosine; they
replace no instructions. `wing_dimensions_with_trig` matches all 4445 output
words exactly, including area, weighted radius sum, mean, fraction and length.
The platform path matches 4419/4445 words on macOS; worst scaled absolute error
is 1.3563161e-6, below its 2e-6 threshold. Arithmetic replay has zero tolerance.

Supported API domain: nonempty member list, matching finite coordinate/chord arrays,
1..10 elements per member, positive finite span/group area and nonzero finite
endpoint length. Original malformed/degenerate inputs are not established. Group
selection via `0x141a6beb0`, endpoint propagation and the upstream group-area pass
remain separate recovery tasks. Live fields are kept explicit until that evidence
exists; no adjacency or symmetry grouping is silently assumed.

Update: grouping is now covered by [WING_GROUP_SCAN.md](WING_GROUP_SCAN.md).
The live model supplies recovered aspect/shape values computed on its approximate boundaries.
