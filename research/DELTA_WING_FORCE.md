# Delta branch through get_el_force

`element_force_with_delta` passes an explicit noise table and double time to
`evaluate_with_delta`, using the existing Call's foil thickness values. The
legacy API continues to require context for positive delta weight. No synthetic
runtime defaults are introduced into this verified numerical API.

`tools/gen_delta_force_vectors.py` executes full original `0x1411b9840` from
reference SHA256 `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
200 positive-delta cases include 81 with ice and 20 zero-time cases, 301 recorded
profile calls, varying controls, mask queries, thicknesses and geometry. The
profile evaluator is a recording hook; the input-binding query is the existing
mask replacement instruction sequence. These two substitutions are disclosed in
fixture metadata. Trigonometry, noise, geometry and all other numerical callees
execute naturally; the completion guard verifies return. Structural-load work
is explicitly skipped with F+0x28=1 and diagnostics flags are zero.

Although F+0x420 is bounded to subsonic values, flow separation can select the
alternative panel branch. The expanded corpus exposed cancellation in case 94:
the delta element's Cl/Cd/Cm words matched exactly, but platform float32 atan in
the alternative branch changed its lift by three ULP and moment by six ULP.
The final almost-zero moment differed by 1024 ULP after blending and an offset.
Tracing the original established atan input `0x3e140f55` and output `0x3e130a75`.
Double atan rounded to float32 reproduces that result without widening the test
tolerance. This correction remains local to the alternative panel model.

After correction, debug and release match 194/200 entire output sets exactly;
worst difference is 35 ULP under the existing 64 ULP force tolerance. All persistent
stall flags and profile call structure agree. Angle/multiplier/divisor inputs
retain the existing 8 ULP or 2e-5 absolute platform-trigonometry allowance; all
other recorded call fields are exact. The old 250-case force corpus also passes:
248 output sets exact, worst difference two ULP on the macOS host.

Live runtime state recovery and interactive flight-model wiring remain separate.
The fixtures use an explicit reproducible table, not a claim about reference
initialization. No diagnostic streams or structural loads are verified here.
