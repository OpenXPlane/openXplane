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
- `engine_held_back` (`0x1411d9f60`): the engine hold rule of the input handlers: kind 6 with a zero lever
  (`F+0x64b4`) is never held; the binding pair `0x179`/`0x1f9` blocks modes 2/1; the bindings `0x2f6..0x2f9`
  block by the sign combination of the record's `+0x790`/`+0x798` floats; otherwise `0x140822620` decides (not
  ported). 600 vectors with stubbed binding answers (`tools/gen_engine_held_vectors.py`).
- `record_flag_6040` (`0x1411d9ec0`): the enabled byte of a record of the `B+0x6040` table (stride `0x34c8`),
  blocked by binding `0x179` (queried with the record's mode word) and by the word at `+0x54`. 400 vectors
  (`tools/gen_record_flag_vectors.py`). What these records are is not established.
- `cosine_blend` (`0x14121b4d0`, double result, `0x14230b380` is `cosf`), `root_ratio` (`0x1411b5ee0`: `2*sqrt(v/pi)`
  over `max(record - f664, 0.01)`, held to 0..1; negative `v` reaches the CRT's sqrt error handler `0x142300430`,
  whose result is not established) and `record_flag_6028` (`0x1411da150`, binding `0x251`): 900 vectors
  (`tools/gen_flight_helper_vectors.py`). Meaning of the quantities is not established.
- `airflow` (`0x14121b580`, `crates/xp-airfoil/src/airflow.rs`): the airflow at a point of the aircraft. The point is
  rotated into the world frame in double precision (matrices `F+0x430..0x454`, origin `F+0x378..0x388` unless the
  engine flag is set), the wind sampler `0x141ba80a0` (passed in; not ported) is asked for the wind there, the
  wind is held to +-200 per component (otherwise the original aborts), the reference `F+0x368..0x370` is
  subtracted, the result is moved back with `to_aircraft_frame` and the rotation term of the rates
  `F+0x3cc/0x3d0/0x3d4` is added. Each stored result goes through `0x141176330` (non-finite becomes zero). 500
  vectors with a stubbed sampler, identical bits (`tools/gen_airflow_vectors.py`). Not ported: the call of
  `0x14117d970` selected by the last argument (callers seen so far pass zero).
  **Correction**: the eighth argument of the frame transform is *not* the caller's `r12` (as first assumed): the
  function's inline finite check executes `xorl %r12d, %r12d` at `0x14121b735` on every path, so the argument is 0
  and the origin is never subtracted from the wind.
- `scalar` (`crates/xp-airfoil/src/scalar.rs`): `clamp` (`0x14081dfa0`), `sign` (`0x140910be0`), `max3`
  (`0x1411b4a20`), `snap` (`0x1411b03f0`: a value inside the bounds goes to the nearer one), `lerp`
  (`0x140819240`, `t` held to 0..1) and `kind_is_3_or_7` (`0x1411e26e0`). 1500 vectors including NaN and ties
  (`tools/gen_scalar_vectors.py`).
- Reading of `0x1411bd470` (24 KB, called once per engine index): it is the **per-engine propeller force** function,
  not a wing function as first assumed: it indexes the engine record `B[0x5ff8] + i*0x68` and the part record
  `B[0x6010] + i*0x3770` with the same index, flies the airflow at the part's point `+0x790/+0x794/+0x798`, and
  loops over blade stations (`boundary_at(r12+0x88, ...)`) with `get_el_force`. Not ported yet.

## The propeller force function `0x1411bd470` (in progress)

Ported in segments to `crates/xp-airfoil/src/prop.rs`; each segment is checked at a frame checkpoint against the
original running on lazily random-filled objects (`tools/xp_fuzz.py`: the first read of a word fills it with a
random value of the kind the instruction implies, so the initial state is exactly the words that were read;
`tools/gen_prop_vectors.py` records them with the stubbed callees' answers). Float words are compared up to a few
ulps of libm.

- Segment 1 (entry to `0x1411bda66`, 120 trials, 65 reaching the checkpoint): the speed factor from the two
  ramps on `F+0x170` (`B+0x2140/0x2144/0xa78`), the early return below 0.01, the airflow at the part's point
  `P+0x790..0x798`, its rotation by `P+0x79c..0x7a4`, the three speed terms (`hypot2`, `hypot3`, `atan2`), the
  `F+0x1cc/0x1d0/0x120` terms, and the lever block of part kinds 3 and 7 (`R+0x30..0x3c`). The debug dump
  guarded by `F+0xbcc8` is not ported.
- `rdi` in the original is `P+0x88`, so the `+0x708/+0x710/+0x718` reads of that block are `P+0x790/0x798/0x7a0`.
- Segment 2 (`0x1411bda66` .. `0x1411be62a`, 150 trials): the lever response curve on `R+0x84` / `F+0x8e8` (a stack of
  `interpolate_clamped` segments around the speed ratio), the smoothing of `R+0xa0` toward it with `2 * dt`, the
  `P+0x10 == 2` branch, the rotation of the point (0, 0, -1) by the part's angles, and the zeroing of the
  per-pass accumulators (frame slots `0xcc..0xec`, `0x114`, `0x10`).
- Segment 3 (first pass of the station loop up to the second airflow call at `0x1411bf1c8`, 100 trials): the
  per-pass constants, the station position on the blade (`boundary_at` x/y/z, dihedral, azimuth angle `inner *
  pi/2`), its rotation by the part angles and into the world frame, the terrain probe `0x14195f4b0`
  (replayed), the ground-effect ratio, the clamped `R+0xa4` and `R+0x144` factors and the six-octave
  turbulence (`NoiseTable::basis2` = `0x140984e50`, a 2-D lookup in the same 0x40000-float runtime table as the
  3-D noise; the vectors fill the table with a fixed pseudo-random sequence because the real contents are
  generated at run time and not recovered).
- `tools/clean_asm.py` now also prints the repairs of the inline finite checks of the form `movaps %xmm14, %xmmN`,
  `movss %xmm14, off(%rbp)` and `movl %esi/%edi, off(%r14)`; the function has twelve of them (after
  `get_el_force` and at the force accumulation), each replacing a non-finite value by zero.
- The second airflow call of the loop passes 1 as its last argument, which selects the call of `0x14117d970`
  (18 KB, not ported yet).
- Segments 4-8 (the rest of the station loop and the loop control; every segment is checked at its first-pass
  checkpoint, segment 8 after the whole loop with 2-4 elements x 4 azimuths): the second airflow call with the wash
  adjustment `0x14117d970` (not ported, replayed), the rotation of the wind into the part frame, the blade-station
  angle bookkeeping (`X+0x194/0x54/0x2c`, wrapped to -180..180), the call of `get_el_force` (replayed through
  `PropEnv::element_force`, which is verified separately as `element_force`), the force terms of the pass with their
  finite checks, three `rotate_pairs` into the aircraft frame and the accumulation into the propeller force
  totals `F+0x2e4`, `F+0x2d0`, `F+0x2bc`, the pass record (frame words `0x1b0..0x200`) appended by `0x141219d90`
  when `F+0x28` equals the global at `0x142f2e3dc`, the moment sums (frame slots `0xcc`, `0xd0`, `0xdc`, `0xd4`,
  `0xd8`, `0x70`, `0x110`, `0xc8`, `0xe0`; `0x1408fd570` is the angle interpolation, ported as
  `scalar::angle_lerp`), the ground-strike event (`0x1407cdce0`, replayed) and the air-flow lag step
  (`engine::lag_filter` on `R+0x284+4*(inner+4k)`).
- Loop structure of the original: an outer loop over the `P+0x8c` span elements `k` (frame slot `0x8d8`) and an inner
  loop of 4 azimuth positions (`inner * pi/2`, frame slots `8` and `0x30`).
- Not ported yet: the code after the loop (`0x1411c22c9..0x1411c3531`) and the wash function `0x14117d970`.

### The propeller force function: complete (`prop::prop_force`)

All of `0x1411bd470` is now ported (`crates/xp-airfoil/src/prop.rs`) and compared with the original running to the end
on random objects (`prop_force_matches_the_original_machine_code_to_the_end`, 120 trials; the segment tests compare
each stage at its checkpoint). Every written word of `R` (the output record), `F` (the propeller force and moment
totals `F+0x2bc/0x2d0/0x2e4` and `F+0x2f8/0x310/0x328`), the element-state records `X`, the strike record `N` and every
frame slot the port models match the original (float words up to a few ulps of libm).

Stages added after the loop:

- Segment 9 (the post-loop section, up to `0x1411c2d35`): the lag of `R+0x84`, the thrust-vector clamp of
  `R+0x48/0x4c` and the limit logic against `B+0x2094/0x2098` (the event calls `0x1f9`/`0x209`), the moment terms
  added to `F+0x2f8/0x310/0x328` from the sums of the loop, with the two branches selected by the lever flag.
- Segment 10 (`0x1411c32b6`): the output record `R+0x14..0x88` (efficiency-like ratios, the angle of the wind in the
  disc `atan2`, the six factors of `R+0x5c`..`0x80`).
- Segment 11 (the end): the pitch-limit force term of `R+0x64` from the engine record list at `F[0x68b0]`.

Callees that are replayed from the original and not yet ported: the airflow wash `0x14117d970` (18 KB), the wind
sampler `0x141ba80a0`, the terrain probe `0x14195f4b0`, the element force's profile callbacks (the function itself
is `element_force`), `0x1407cdce0` (an event queue), `0x1408154c0`, `0x1411a0900`, `0x141218060`, `0x1411daa80`,
`0x141219d90` (the pass record log) and the input-binding queries behind `0x1411d9f60`. The debug stream dump that
the original performs when `F+0xbcc8/0xbcd0` are nonzero is not ported (the vectors keep both zero). The 2-D noise
table `0x14578f1f0` is generated at run time; the vectors fill it with a fixed pseudo-random sequence.
