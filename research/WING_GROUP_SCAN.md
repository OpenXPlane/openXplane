# Ordered original wing-group scan and live wiring

Reference SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
`group_wings_with_areas` ports inner scan `0x1411720dc..0x14117285b` inside
`0x141170d40`: 49 passes over slot IDs 0..47 in ascending order. Disabled,
root and visited candidates are skipped. Candidate tip is tested against current
group root, then candidate root against current group tip, using the two recovered
predicates. Both tests share an iteration without a visited recheck. A closed
candidate can therefore be appended twice. The last pass permits the recovered
large-dihedral connection exception. There is no early convergence termination.
Groups are independently rebuilt for each root and may overlap.

An accepted candidate updates the corresponding endpoint, chord and dihedral
reference. Its element areas are first summed from zero in float32; that subtotal
is added to group area once. Adding every element directly into the group total
would change rounding. Member order, duplicates and per-candidate subtotals are
preserved by the port. `group_wings` computes the element areas using the existing
recovered helper; the explicit-area API supports deterministic replay.

`tools/gen_wing_group_vectors.py` executes the complete inner scan and enforces
its exact endpoint. The input root vector, root area, endpoints and reference
registers are explicitly initialized. Both index/pointer vectors have capacity,
so vector-growth allocators are not reached. Four unavoidable outputstream
functions are return-pointer stubs: `0x1405e18d0`, `0x1405ea2c0`, `0x1408943e0`,
`0x140606f80`. They only return a mapped dummy stream; no predicate, interpolation,
CRT cosine, element-area arithmetic or state update is replaced. 5661 logging
calls occurred over the corpus. Stream effects and initial root-vector construction
are outside this comparison.

125 cases cover ordered chains, scrambled slot IDs, roots inside chains, disabled
slots, centerline crossing rejection, a late-pass ordering case and duplicate
candidate insertion, differing root/tip chords and spans, and propagation of the
chord-relative join tolerance. The resulting member IDs, pointer-vector identity, all six
end coordinates, area and four reference values match exactly. Rust tests compare
all 1375 float words and every member ID with original outputs; default platform
area calculation is checked separately at 2e-6 scaled absolute error.

The approximate flight model now performs recovered grouping and dimension writing
at aircraft load, using its currently constructed wing boundaries. The calculated
aspect/centroid parameters replace the zeros previously supplied as Wing+0x18/+0x1c.
All five local Cessna behavioural tests pass, including the high-sweep smoke case.
This is recovered arithmetic on approximate geometry. Exact boundary generation,
48-slot population (the typed reader currently handles 32 ACF wings), geometry-update
scheduling, reference aircraft bounds, noise initialization and the other flight
approximations remain distinct recovery tasks. No whole-model parity is implied.
