# Delta context in the interactive flight model

The approximate `xp-sim` flight model now calls `evaluate_with_delta` for every
wing element. It supplies the same synthetic noise table already used by profile
buffet, the model's double running time and thicknesses from the loaded AFL files.
The previous swept-wing skip flag is removed. This connects the recovered kernel
to the running flight model; it does not establish whole-model equivalence.

Static reference evidence for AFL 1110 thickness: loader `0x141a42f06` reads the
first header scalar and `0x141a42f0b` stores it at foil+0x58. The next scalar is
stored at +0x5c. The recovered `0x14121bfd0` kernel reads foil+0x58. Thus
`Airfoil::thickness()` returns header_scalars[0] without a guessed conversion.
Reference SHA256 is `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
This field correspondence is disassembly evidence, not an emulated whole-loader test.

`OPENXPLANE_REFERENCE_ROOT` lets local flight tests use a reference installation
outside a managed worktree. Five behavioural tests ran against the supplied local
Cessna resources: resting on gear, takeoff/climb, control signs, slow stall and a
new high-sweep smoke case. The latter creates a temporary ACF copy with the first
four wings at 60 degrees, loads its real airfoils, runs 50 physics steps and checks
finite state plus nonzero finite force/moment reports from all four surfaces.
The temporary ACF is removed; no original resources are committed. Without local
resources these behavioural tests explicitly skip and supply no flight evidence.

Remaining approximation includes the synthetic noise initialization, assumed
Aircraft+0x64f4/+0x64f8/+0x64fc reference values, Wing+0x18/+0x1c, state divisor,
geometry construction, downwash, controls, propulsion and inertias. These values
need independent recovery. The numerical delta API remains separately verified
with explicit controlled state in [DELTA_WING_INTEGRATION.md](DELTA_WING_INTEGRATION.md).
