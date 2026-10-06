# The wing element aerodynamics function (partial)

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the function at
`0x1411b6630 .. 0x1411b8df7` (10183 bytes), the caller of the profile function
[`0x141a44350`](AFL_REGIMES.md). It is **not ported yet**; this note records what is established so the port
can be done in verified pieces with the emulator ([VERIFICATION.md](VERIFICATION.md)).

About 1400 of its 2284 instructions are diagnostic logging (stream calls), guarded by two flags at
`0xbcd0(%rdi)` and `0xbcc8(%rdi)`; the numeric code is about 900 instructions with 24 calls. The diagnostic
strings name the quantities: `chord`, `dynamic_viscosity`, `Re_meg`, `root foil`, `mid foil`, `tip foil`,
`weighted`, `takes cl/cd/cm to`, `aero aspect ratio`, `gives Cdi`, `takes Cdi to`, `takes total Cd to`,
`aero_sweep_00_MAC`, `alpha_induced`, `delta-wing Cl/Cd/Cm/alpha ratio`, `final Cl/Cd/Cm/alpha ratio`, and
`ICE IS CHANGING THE COEFFICIENTS! ice_cl_mult=... ice_cd_mult=...`.

## Interface (observed)

Windows x64 calling convention. `RCX` (`rdi`) is an object whose fields include `+0x20` (a pointer to a second
object holding per-aircraft reference values at `+0x64f4`, `+0x64f8`, `+0x64fc`, and values at `+0x1f00`,
`+0x1f3c`), `+0x408`, `+0x5c`, `+0x6c`, `+0x1a0`, `+0x1a4`, `+0xdac`. `RDX` (`rsi`) is the wing object: it has the
layout of the ACF wing loaded by `0x141aad640` ([ACF_SCHEMA.md](ACF_SCHEMA.md): `+0x00` `_is_right_mult`, `+0x04`
`_els`, `+0x5c..+0x68` the four `_foil_rat_*` ratios) plus runtime arrays: per-element flap and slat flags at
`+0x56c` and `+0x594`, corner point coordinates at `+0x5bc`, `+0x5e8`, `+0x614` (one array per axis, 11 entries),
element vector components at `+0x70`/`+0x74`, and three airfoil object pointers at `+0x3678`, `+0x3680`, `+0x3688`
with their names at `+0x3618`, `+0x3638`, `+0x3658` (std::string objects). `R8` is a float array indexed by the
element (read at `+0x04`, `+0x2c`, `+0x54`, `+0x28c`; written at `+0x1bc` and `+0x1e4`), `R9` an int flag, and the
fifth to fifteenth arguments on the stack: the element index (int), three floats, three extra floats that are
added into the accumulators (`+0x48`, `+0x50`, `+0x58`), and four output pointers (accumulated Cl, Cd, Cm and
the induced-drag term). It returns a float in `XMM0`: the weighted normalised angle.

## Steps

1. **Element geometry (ported, verified).** `0x1411a0230` gives the sweep in degrees of an element's
   quarter-chord line: `atan2(zq[i+1] - zq[i], sqrt(dx^2 + dy^2)) * 57.29578` with `zq = z - chord / 4` and `dx`,
   `dy` the differences of the first two coordinate arrays (`+0x5bc`, `+0x5e8`) between the element's two
   boundary points, the third array (`+0x614`) being `z` and the chord array `+0x70`. `0x1411a00f0` turns it
   into the delta-wing weight `ramp * ratio * tail`: `ramp = clamp((sweep - 40) * 0.1, 0, 1)`; `ratio` from
   `c = 4 / tan(clamp(|sweep|, 15, 75) deg)`, `clamp((field18 - 2c) / (c - 2c), 0, 1)` or 0.5 when `c = 0`;
   `tail = clamp(2 (1 - field1c), 0, 1)`. Both are in `src/wing_element.rs` and match the original bit for bit
   in 1500 cases each ([VERIFICATION.md](VERIFICATION.md)); `field18` and `field1c` are the object fields at
   `+0x18` and `+0x1c` whose meaning is not established.
2. **Flow factor and Reynolds number.** A lift-slope style factor `1 / (1 + 5.7296 / (pi * x))`
   (5.7296 = 0.1 per degree in radians) followed by `k = (sqrt(8 * y + 1) + 1) / 4` is computed and later
   multiplies the accumulated Cl. The dynamic viscosity of air is interpolated linearly through
   `1.4638e-05` (-50), `1.7231e-05` (0) and `1.9608e-05` (+50) against a temperature-like field
   (`+0x5c(%rdi)`), using the interpolation helper `0x1406ea0b0`; `Re_meg` is the chord Reynolds number divided by
   `1e6`. This `Re_meg` is the regime parameter passed to the profile function.
3. **Position inputs.** The corner coordinates interpolated at the element centre and divided by the three
   aircraft reference values (`+0x64f4`, `+0x64f8`, `+0x64fc`) give normalised x, y and z of the element. They are
   passed on as the `x`, `y`, `phase` noise coordinates of the stall perturbation
   ([BUFFET.md](BUFFET.md)). Confirmed in `0x141a44350`: its second and third float arguments are forwarded as the
   noise `x` and `y`, and the stack float as `phase`. The earlier note that those two arguments were an angle
   multiplier and divisor was wrong; the multiplier and divisor come from step 4.
4. **Angle and the angle correction factors.** An element angle is wrapped into -180..180 degrees (`xmm9`).
   Two factors are faded towards 1 as the absolute angle grows from 20 to 70 degrees
   (`x' = clamp(x + (1 - x) / 70 * (|angle| - 20))`, kept between `x` and 1): the flow factor `y` of step 2 and a
   flap/slat factor built from the per-element flags `+0x56c`, `+0x594` and the fields `+0x1a0`, `+0x1a4`. The
   faded values (`xmm14`, `xmm15`) are passed to the profile function together with the angle, so they are, as far
   as the data flow shows, the angle multiplier and divisor that `0x141a412c0` takes
   ([AERO_LOOKUP.md](AERO_LOOKUP.md)). This reading is from the register flow and is still to be confirmed in
   the emulator.
5. **Three airfoils.** The element's span position `t = (index + 0.5) / elements` gives weights by the four
   `_foil_rat_*` ratios: the root weight falls from 1 at `_foil_rat_rot` to 0 at `_foil_rat_mid_inner`, the tip
   weight rises from 0 at `_foil_rat_mid_outer` to 1 at `_foil_rat_tip`, the middle weight is the rest. A weight is
   forced to 0 when the neighbouring airfoil has the same name. When equal ratios make the division undefined the
   weight is 0.5.
6. **Blend.** For each airfoil with a nonzero weight the profile function is called with its table object, the
   normalised position, the flags, `Re_meg`, the angles and four output pointers; its Cl, Cd, Cm and the
   normalised angle are multiplied by the weight and summed into the accumulators.
7. **Extra terms and compressibility.** The three extra stack floats are added to the accumulators and Cl is
   multiplied by the factor `k` from step 2.
8. **Induced drag.** `Cdi = Cl^2 / (pi * [rsi+0x14] * e)` with `e` the value saved from `R8` at entry; scaled by
   `R8[+0x28c]` and added to Cd.
9. **Delta-wing block.** When the sweep term from step 1 is positive, a blend of the straight-wing result with a
   delta-wing estimate is made using sine/tangent terms of the angle, a shaping function `0x14082b800` called with
   the constants 45, 75, 0.75, 0.035, 0.07, 40, 25, 1.5, 0.9, a noise-driven term (`0x1411e1940`, `0x1408bd9d0`) and
   a clamp of the result between the straight-wing and delta-wing values.
10. **Ice.** When an ice factor in the stack arguments is positive, Cl and Cd are multiplied by two per-element
    factors saved at entry (`ice_cl_mult`, `ice_cd_mult`).

## Not established

The meaning of the objects' fields beyond what the diagnostics name; the sources of `R8`'s arrays (they come from
the caller's per-element flow computation); the behaviour of helpers `0x14082b800`, `0x14121bfd0`, `0x1411e1940`,
`0x1408bd9d0`, `0x1411a0230`, `0x1411a00f0` and of the C runtime functions `0x14230b380`, `0x14230be80`,
`0x14230c1d0`; the exact formulas of steps 2, 4 and 9; and whether the temperature field is in degrees Celsius.
Steps 1, 5, 6 and 7 are read in full; the rest is read at the level of the constants and data flow above.

## Plan

Port from the bottom up and verify each helper against the original in the emulator before composing: the
interpolation helper is already done; next the two geometry helpers, the shaping function and the noise terms,
then this function with synthetic objects whose fields are filled at exactly the offsets read (found by tracing
memory reads in the emulator), so the verification does not depend on knowing every field's meaning.
