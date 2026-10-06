# The approximate flight model

Date: 2026-10-06. **This is not the reference build's flight model.** It is openXplane's own rigid-body model
that runs on the Cessna's real ACF and airfoil data so the aircraft can be flown while the original's model is
still being recovered ([WING_ELEMENT.md](WING_ELEMENT.md)). Do not read its results as agreement with
X-Plane. The code is `src/wing.rs` and `src/flight.rs`.

## What comes from the original

| Piece | Source |
|---|---|
| ACF mass, CG, wing geometry, gear springs and dampers, engine power, control deflection limits and chord ratios, flap detents | ACF properties ([ACF_SCHEMA.md](ACF_SCHEMA.md); lengths with the build's float32 feet constant) |
| Airfoil tables, finite-grid lookup, stall hysteresis, buffet perturbation, three-table blending | `profile::evaluate`, verified bit for bit against the original machine code ([VERIFICATION.md](VERIFICATION.md)) |
| Span weights of the root, middle and tip airfoils of an element | the wing element function, read in full ([WING_ELEMENT.md](WING_ELEMENT.md) step 5) |
| The whole per-element coefficient computation: flow factor, compressibility factor, Reynolds number from viscosity, the three airfoil weights, the profile function with its Mach factor, accumulation, induced drag | `wing_element::evaluate` and `profile::outer`, both verified bit for bit against the original machine code ([VERIFICATION.md](VERIFICATION.md)); what the model feeds them (see below) is its own |

## What is openXplane's own simplification

- A flat ground at height 0, no wind, no turbulence, ISA atmosphere.
- Element forces only: lift and drag along the element's flow from the airfoil tables, the pitching moment about
  the quarter chord. No element-to-element interference, no fuselage lift, no sweep or span-flow effects beyond
  the span-direction vector.
- Control surfaces shift the effective angle of attack by `0.65 * tau * delta` (thin-airfoil `tau`, an assumed
  effectiveness factor); flaps add `0.012 (delta/10)^2` to Cd on flapped elements.
- Inputs fed to the element function: the temperature and density of the ISA atmosphere, the element's own speed
  in the plane of the airfoil as the Reynolds-number speed, the Mach number from that speed, the local angle of
  attack (with the control-surface shift and the tail downwash added), the aspect ratio of the whole surface as
  the function's aspect-ratio field with a unit speed factor, no flap or slat factor, no ice, and a position
  normalisation of 10 m. In the original these come from the flight loop, which is not recovered. A wing whose
  design sweep exceeds 35 degrees runs the straight-wing path without the delta-wing block.
- The tail sits in the wing's downwash estimated as `1.2 * 2 CL / (pi AR)` from the wing lift one step ago.
- A fuselage pitching-moment slope of `+0.5` per radian (referenced to wing area and mean chord) and a parasite
  drag area of `0.30 m^2`; without them the lifting surfaces alone are about twice as stable in pitch as a real
  Cessna 172 and the stick cannot reach the stall.
- Propeller: thrust is `min(2400 N * (P/Pmax)^(2/3), 0.78 P / (V + 22 m/s))`, with power lapse `(rho/rho0 - 0.12)/0.88`.
  No torque, gyroscopic or p-factor effects, no slipstream on the tail.
- Inertias: published Cessna 172 values (1825, 1285, 2667 kg m^2 at 1043 kg) scaled by mass; the ACF's own inertia
  fields are zero.
- Landing gear: a spring and damper per strut from the ACF (`_strut_max_wgt_frc` at `_strut_max_wgt_def`, `_damp`),
  static loads from the three-strut balance, tire forces from a friction circle with assumed coefficients
  (rolling 0.02, brakes 0.7, lateral 0.9), nose-wheel steering from the rudder.
- The stall-buffet noise table is synthetic (the original's table content is not recovered).
- Fixed time step, semi-implicit Euler at 200 Hz.

## Behaviour on the provided Cessna 172 SP

Mass 951 kg (empty plus 80 kg fuel and 90 kg payload). Checked by `crates/xp-app/tests/flight_local.rs` and `fly-test`
(these need the reference installation and are skipped without it):

- Rests on its gear without drifting (height, speed and attitude constant over 10 s).
- Accelerates straight down the runway (heading holds), lifts off with full power under a simple pitch
  controller and climbs about 680 feet per minute at 94 knots, wings level. The real aircraft lifts off near
  55 knots and climbs about 700 feet per minute at 73 knots: the model's liftoff is later than the real one
  (it has about 15 percent less maximum lift than the real aircraft, whose fuselage also lifts).
- Control signs: stick back pitches up, stick right rolls right, right pedal yaws right.
- Held full back at idle power it stalls the whole wing and oscillates through repeated stall and recovery.

The G1000 and seaplane variants load and fly as well (the seaplane's floats are treated as wheeled gear and its
preview uses `fuselage_floats.obj`); only the standard variant is covered by the tests.

```sh
cargo run --release --offline -- fly-test Xplane12 "Xplane12/Cessna 172 SP/Cessna_172SP.acf" 60
FLY_DEBUG=1 cargo run --release --offline -- fly-test Xplane12 "Xplane12/Cessna 172 SP/Cessna_172SP.acf" 40
```

## Flying it

```sh
cargo run --release --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

The window follows the aircraft with a chase camera and shows telemetry in the title. The aircraft meshes are
posed each frame from the model's state (`scene::AircraftBody`) and re-uploaded to the GPU; the ground is
static geometry from apt.dat ([WORLD_GEOMETRY.md](WORLD_GEOMETRY.md)). Keys are listed in the README. The
simulation advances in fixed 1/200 s steps from real elapsed time (at most 0.1 s per frame), so a slow frame
does not change the physics.

## Not done

Trim, flap and gear controls beyond the flap handle, engine start and fuel flow, the propeller as a blade
element model, ground effect, wind and turbulence, retractable gear, a proper autopilot, and comparison with
trajectories of the original. The tuning constants above are placeholders to be replaced by recovered
original behaviour.
