# Angle correction and AFL lookup from the original EXE

Date: 2026-10-06. Reference: [BASELINE.md](BASELINE.md), SHA256
`2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`. This is the result of static analysis of the
function `0x141a412c0 .. 0x141a41a2a`. It has not yet been compared with the original running.

The implementation in `src/aero.rs` covers the section up to the first coefficient sums. This is part of the
profile algorithm, not finished aircraft aerodynamics. CLI: `airfoil-eval <afl> <induced-alpha> <multiplier>
<divisor>`.

## Inputs

After the prologue RCX points to one table structure. The offsets `+4` and `+8` hold parameters 1 and 2 of the
AFL block; this is confirmed by the loader that writes them. Their role follows from the expression
`-parameter[2] / parameter[1]`: the zero-lift angle for a positive slope.

Stack inputs after the prologue:

| Offset from RSP | Role in the studied section |
|---|---|
| `0x110` | The angle; the diagnostic string calls it AOAinduced |
| `0x118` | Angle correction multiplier; diagnostic alpha-mult |
| `0x120` | Angle correction divisor; diagnostic alpha-div; then the Cl scale |

How the earlier stages obtain these inputs is not recovered yet. The values for the CLI are given by the user;
`1/2` in the examples are check conditions, not a Cessna wing parameter found in the ACF.

## Correction

The order below matches the float32 scalar operations in the range `0x141a412df .. 0x141a41419`. `wrap` adds 360
when the angle is below -180 and -360 when it is above +180, until the angle is in range; the two bounds are
kept separate.

```text
zero = slope > 0 ? -cl_zero / slope : 0
offset = alpha_induced - zero
scaled = offset * multiplier
base = scaled / divisor + zero
fade = clamp((abs(alpha_induced) - 30) * float32(0x3d088889) + 0, 0, 1)
delta = wrap(alpha_induced - base)
effective = wrap(fade * delta + base)
```

The factor `0x3d088889` equals `0.03333333507180214`; it was read at `0x1425841bc`. At |alpha| <= 30 the
correction applies in full; between 30 and 60 degrees there is a transition; at |alpha| >= 60 the angle returns
to the input, taking the circular wrap into account. The instructions and diagnostic strings confirm exactly the
numeric rule; the physical interpretation of the earlier multipliers still needs research into the calling code.

In openXplane NaN/Infinity and a zero divisor are rejected. The wrap loop is limited to 4096 iterations for
pathological inputs. This is a limit of the diagnostic tool, not established X-Plane behaviour.

## Index and weights

In the range `0x141a414ae .. 0x141a4184e` the branches turn the effective angle into a fixed-grid coordinate.
The file's alpha column does not take part:

```text
a = clamp(effective, -180, 179)
if a < -20: x = clamp((a - (-180)) + 0, 0, 160)
else if a < 20: x = clamp((a - (-20)) * 10 + 160, 160, 560)
else: x = clamp((a - 20) + 560, 560, 719)
lower = trunc(x)
upper = lower + 1
weight_lower = float32(upper) - x
weight_upper = 1 - weight_lower
```

The limit of 179° is confirmed by the constant `0x43330000` at `0x14254d9d4`. After wrap the angle +180° selects
the 179° row, and -180° selects the -180° row. We do not replace this with the more familiar symmetric
interpolation.

The grid constants read from the EXE:

| VA | float32 value |
|---|---:|
| `0x14250e83c` | -180 |
| `0x14250e820` | -20 |
| `0x14250e5f4` | 20 |
| `0x14253fa64` | 160 |
| `0x1427b5788` | 560 |
| `0x1427b578c` | 719 |
| `0x14250e578` | 10 |

## Coefficients

The instructions `0x141a4175d .. 0x141a417ba` access arrays at offsets `0x14` (Cl), `0xb58` (Cd), `0x169c` (Cm).
For each:

```text
left = weight_lower * table[lower]
right = weight_upper * table[upper]
value = left + right
Cl = value_Cl * divisor
Cd = value_Cd
Cm = value_Cm
```

The order of float32 operations is kept. No rearrangement to `left + (right-left)*t` and no fused multiply-add
is used. The later branches of this function keep correcting the results: these coefficients must not be passed
off as final forces or the whole output of the original. There is no Reynolds table blending, no area or flow
speed computation, no forces, moments, control-surface effects or motion integration.

## Verification

16 unit tests pass, Clippy without warnings, offline build. The new tests use synthetic arrays: the bounds ±20°,
±180°, half-degree and fractional angles, zero slope, the transition angles 30/45/60°, a Cl-only scale, wrap, a
damaged length and invalid numbers. The alpha column in the test table deliberately differs from the fixed grid,
to check its independence from the index computation.

All 34 provided airfoils passed 8 argument sets - 272 diagnostic runs without errors. This is a check of the
implemented section on content, not proof of agreement with the original's flight.

NACA 2412, induced-alpha=10, multiplier=1, divisor=2: effective-alpha `3.774509668`, indices `397/398`, weights
`0.254913330 / 0.745086670`. In the four tables Cl is `1.269999743`, Cm `-0.036755491`; Cd depends on the table.

Local evidence in `research/local/` (excluded from Git): `airfoil-3d-corrections.txt`,
`airfoil-index-constants.json`, `airfoil-reference-evaluation.txt`.
