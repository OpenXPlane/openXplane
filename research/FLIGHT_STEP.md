# The flight model step (`0x1412656b0`)

A 60 KB function that runs the aircraft's physics for one step. Parts of it are ported by running a block of it in
the emulator on random data (the block must not call state-dependent functions; the debug range checks are
replaced by a stub that returns).

## Force and moment totals (block `0x14126f37f..0x14126f7f3`) - ported

`forces::force_totals`. In aircraft axes the flight object keeps, for each of the three forces (axial, normal,
side) and three moments (L, M, N), the contribution of the propeller, the aerodynamics, the landing gear, the
aircraft mass (moments only) and a plug-in, plus the total: `+0x2bc..+0x2cc` axial, `+0x2d0..+0x2e0` normal,
`+0x2e4..+0x2f4` side, `+0x2f8..+0x30c` L, `+0x310..+0x324` M, `+0x328..+0x33c` N (the names are the
`sim/flightmodel/forces` datarefs). When the flag `+0x676c` (propeller), `+0x6768` (aerodynamics) or `+0x6770`
(gear) is set, that group's six values are replaced from the override area `+0x6780..+0x67c0`; the totals are the
aerodynamic term plus the propeller, gear, mass and plug-in terms in that order. When `+0x675c` is set the
block does nothing (the totals come from outside). 300 emulator cases are identical.

## Small functions of the step, ported

Verified by emulator vectors (`tools/gen_frame_vectors.py`, `tools/gen_wing_misc_vectors.py`):

- `rotate_pairs` (`0x1407ac180`): rotation of three values by three sine/cosine pairs; `from_aircraft_frame`
  (`0x1407ac020`): that rotation with the frame's pairs, then the origin offsets added in double precision
  (counterpart of `to_aircraft_frame`); `rotate_euler_offset` (`0x14120cf60`): the same from three angles in
  degrees, plus the offsets `+0x90..+0x98` of the object. Identical (the angle version within libm error).
- `boundary_at` (`0x1411c5950`, `0x1411c59b0`, `0x1411c5a10`): a wing boundary coordinate array at a span position.
- `signed_sqrt` (`0x14122d4b0`), `element_dihedral` (`0x1411a02f0`).
- `boundary_ratio` (`0x14121b290`): from the three boundary arrays of a wing (`W+0x5bc`, `+0x5e8`, `+0x614`) at its
  mid element (`count * 0.5` as float32, clamped interpolation), combined with the frame terms
  `F+0x430/0x434/0x450/0x454`, the optional double `F+0x380` and the reference `F+0x42f5c`, divided by a
  denominator whose magnitude is held to at least 0.01. 300 vectors, identical bits
  (`tools/gen_boundary_ratio_vectors.py`). What the ratio means for the flight step is not established.
