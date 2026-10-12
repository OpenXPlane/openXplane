# Reynolds, flap/slat, induced drag and ice: verified instruction ports

Date: 2026-10-07. Reference SHA256:
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
Implementation: `src/wing_aero.rs`; reference fixture: `tests/data/wing_corrections.txt`.
These are five stages of `0x1411b6630`, not the whole wing function or flight model.

## Viscosity and Reynolds — 0x1411b6979 .. 0x1411b6a44

Raw input offsets: object temperature-like float +0x5c, field +0x6c, wing element floats
+0x70/+0x74, per-element flow +0x54. Their production and exact units are not yet established.
The viscosity table contains exact float32 bits:

| Temperature-like input | Viscosity bits | Address |
|---|---|---|
| -50 | `377595bb` | `0x142672628` |
| 0 | `37908b4a` | `0x14267262c` |
| 50 | `37a47bdc` | `0x142672630` |

The original clamped interpolation helper `0x1406ea0b0` executes unchanged. The negative branch
uses -50..0; otherwise 0..50. Port delegates to the already recovered `regimes::blend`.

With `a=wing[+0x70]`, `b=wing[+0x74]`, operation order is:

```text
ratio = f32(b / a)
sum = f64(ratio) + 1
square = f32(ratio * ratio)
normalized = (f64(square) + sum) / sum
extent = f64(a) * f64_constant(0x3fe5555555555555)
chord = f32(normalized * extent)
flow = f32(flow[+0x54] * object[+0x6c])
product = f32(chord * flow)
Re_meg = f32(f64(product) / (f64(viscosity) * 1000000))
```

`chord` is named by original diagnostics, not assumed equal to the prototype Element.chord.
1060 vectors compare viscosity, chord and Re_meg, including clamp boundaries, signed zero,
zero flow, varied geometry operands and deterministic random inputs.

## Flap/slat factor — 0x1411b6817 .. 0x1411b689b

Flap and slat are active when the element integers at wing +0x56c / +0x594 are nonzero.
For each active component, float32 instructions compute:

```text
state = object_state - 0.0
sum = foil_reference + aircraft_extra
ratio = sum / foil_reference
delta = ratio - 1.0
factor = state * delta + 1.0
```

Flap state is object +0x1a0, extra aircraft +0x1f00. Slat uses +0x1a4 and +0x1f3c.
The reference is foil object's pointer +0xd0, then float +0x10. Initial factor is 1.
The flap factor is replaced only when active; an active slat multiplies it.
Do not simplify to `extra/reference`: the original addition and subtraction affect rounding.
1024 vectors cover all four flag combinations, negative extras, fractional and out-of-range states.

## Induced drag — two ranges

`0x1411b7c14..0x1411b7c42` computes and stores `Cl² / ((aspect * PI_f32) * efficiency)`.
`0x1411b7d87..0x1411b7da3` multiplies the stored term by flow +0x28c, stores the scaled
term and adds it to Cd. All arithmetic is float32. 1024 vectors compare scaled drag and total Cd.

The approximate flight model now calls this recovered function with its existing whole-surface
aspect ratio and explicit efficiency=1 / scale=1 assumptions. Arithmetic is verified; its inputs
still differ from the original per-element flow caller. This does not establish trajectory equality.

## Ice factors and application

`0x1411b67b4..0x1411b6817` reads a float32 amount from frame +0x150 and promotes it to float64:

```text
alpha_scale = f32(f64(amount) + 1)
lift_multiplier = f32(1 / (f64(amount) * 1.1_double + 1))
drag_multiplier = f32(f64(amount) * 4_double + 1)
```

1.1 is exact stored float64 bits `0x3ff199999999999a` at `0x14253f7f0`;
4.0 is at `0x14250e390`. 1024 vectors compare all three resulting factors.

`0x1411b8d15..0x1411b8da0` multiplies Cl/Cd by the saved multipliers only when amount > 0.
Logging flags at object +0xbcd0/+0xbcc8 are zero, so the original logging branches are skipped.
1024 vectors cover negative amount, signed zero, positive amount, NaN and infinities. The
condition and IEEE comparison are preserved; nonpositive/NaN amounts leave coefficients unchanged.

## Reproduction and limitations

```sh
python tools/gen_wing_correction_vectors.py Xplane12/X-Plane.exe > tests/data/wing_corrections.txt
cargo test --offline --test wing_corrections
cargo test --release --offline --test wing_corrections
```

Generator requires Unicorn; no numpy, stubs, patched instructions, external math substitutes or original
code bytes in the fixture. It verifies the EXE hash, supplies registers/memory explicitly and requires
reaching each exact end boundary. Intermediates are collected at original instruction boundaries rather
than recomputed by the generator. All **5156 vectors match bit for bit in debug and release**.

Finite-input/result checks in the Reynolds/flap/drag/factor APIs are openXplane diagnostic contracts,
not recovered original error handling. Supported finite operand domains are not exhaustive. Ice application
preserves the raw comparison including NaN/infinities. The entire caller is not executed in these tests.

Reynolds, flap/slat and ice are exposed as explicit-input numerical stages and are not silently wired to
invented aircraft fields. The prototype still uses its own atmospheric/Reynolds and flap approximations
and has no ice state. Next: recover original geometry/input construction and span mixing; compose the
whole wing element, then compare its force caller and flight trajectories.
