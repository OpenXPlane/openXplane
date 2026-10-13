# Original wing enabled and endpoint-join predicates

Reference SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
The enabled-byte writer `0x141ab17d0..0x141ab1816` selects a wing when its stored
span is greater than double 0.001 and either stored root or tip chord is greater
than double 0.001. All three operands are float32 metres widened to double. A
float32 threshold would differ at the nominal 1 mm value. `xp_acf::wing::enabled_meters`
reproduces the gate; `Wing::all_from_acf` now applies it after the original float32
feet-to-metres conversion. This admits zero-root, positive-tip wings and excludes
submillimetre surfaces. Missing/malformed ACF policy remains the parser's own.

`tools/gen_wing_enabled_vectors.py` executes the original writer without stubs.
2072 exact boolean cases cover adjacent values around +/-1 mm, signed zero,
343 scaled-feet combinations and random finite inputs. A parser regression
checks a tip-only wing, a submillimetre span and a zero-chord surface.

`wing_dimensions::can_join_wing` ports candidate-root helper `0x141a6beb0` and
candidate-tip helper `0x141a6bbf0`. Both extract endpoints through the original
clamped interpolation. A candidate crossing x=-0.01 to x=+0.01 is rejected.
Reference-point differences are float32 before widening; X/Y must be strictly
below double reference_chord * 0.01, Z strictly below reference_chord * 0.2.
The candidate/reference dihedral difference wraps in float32 to [-180,180].
An absolute difference up to 45 degrees is accepted; larger differences require
the final-pass flag and reference abs(x) >= double 0.01.

`tools/gen_wing_join_vectors.py` executes both complete original helpers with no
stubs. 2936 one-element candidate cases produce 5872 exact boolean results,
covering coordinate tolerance boundaries, centerline crossings, angle wrapping,
45-degree neighbours, final-pass flags and randomized endpoint geometry. Dyadic
random endpoints avoid incidental endpoint cancellation; original interpolators
still execute. The API validates finite matching coordinate arrays and 1..10
segments; angle differences beyond one million degrees are rejected rather than
attempting an unbounded wrap. Damaged/nonfinite original states are not verified.

Static reading of the caller `0x141170d40` shows 49 ordered passes over candidate
IDs 0..47 with visited-state checks, root and then tip predicates, late-pass flag
only on pass 48, endpoint/chord/angle propagation and area accumulation. The full
caller loop has not yet been compared dynamically, including its allocation and
logging paths. Live grouping is not inferred from simple adjacency or symmetry.
These predicates are necessary building blocks for that remaining port.

Update: the complete inner ordered scan has now been replayed with outputstream-only
stubs and explicit root/vector initialization; see [WING_GROUP_SCAN.md](WING_GROUP_SCAN.md).
