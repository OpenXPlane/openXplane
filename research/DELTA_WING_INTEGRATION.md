# Positive-delta wing element integration

`wing_element::evaluate_with_delta` now composes the recovered delta target and
bounded blend with the existing whole-element evaluator. `DeltaContext` explicitly
supplies the noise table, double running time and three optional foil thicknesses.
The original order is preserved: profile accumulation, extra terms, lift scaling,
induced drag, delta construction/blend, then ice multipliers. Induced drag output
remains the straight-branch contribution, as in the original. Delta blending
replaces the persistent stall flag from the absolute blended ratio.

The existing `evaluate` API still rejects positive delta weight without context
unless the caller explicitly selects its legacy skip flag. Supplying a context
computes the delta branch regardless of that flag. Shared `profile_thickness`
keeps the original root/middle/tip interpolation and missing-profile behavior;
`element_force` reuses it instead of maintaining a duplicate implementation.

`tools/gen_delta_element_vectors.py` executes complete original function
`0x1411b6630` from reference SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Only the profile evaluator `0x141a44350` is a recording stub; this verifies its
interface composition, not native profile behavior. Geometry, CRT trigonometry,
thickness lookup, both noises, induced drag, ice and the final return all execute
original instructions. The synthetic table, time and thicknesses are explicit.

350 positive-weight cases cover 88 weights below 0.01, 124 cases with ice,
35 zero-time cases, 165 retain requests and different root/middle/tip thicknesses.
The port matches all 508 recorded profile calls exactly, all stall flags, return,
ratio and induced-drag words. On the macOS host 2099/2100 output words match exactly;
the single platform-trigonometry difference is 4.4703484e-8 absolute. Only Cl/Cd/Cm
allow 2e-6 absolute error. Debug/release checks also alternate the legacy skip flag
to verify the supplied context takes precedence.

This verifies the numerical element API. The approximate interactive flight model
still supplies assumed runtime fields and skips the delta branch; wiring it to
recovered aircraft/reference state remains a distinct task. Live noise initialization,
diagnostics output and pathological conversions are not established by these tests.
