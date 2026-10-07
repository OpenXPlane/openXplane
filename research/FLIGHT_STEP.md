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

### Pointer-following callees and force sinks (`crates/xp-airfoil/src/callees.rs`, `vm.rs`)

Ported on a sparse view of the original's memory (`Vm`: objects at the emulator's absolute addresses, so stored pointers
are followed as the original does) and checked word by word against the original running on lazily random-filled
objects (`tools/gen_callee_vectors.py`, `pointer_following_callees_match_the_original_machine_code`):
`engine_ratio` (`0x1408154c0`), `blend` (`0x1411daa80`), the lever limits `limit_a` (`0x1411a0900` with `0x1411a0970`,
`0x1411a0a10`) and `limit_b` (`0x141218060` with `0x1412180d0`, `0x141218180`), the force sinks `add_axial_force`
(`0x1411767f0`), `add_normal_force` (`0x141176be0`), `add_side_force` (`0x141176e30`) and `add_world_force`
(`0x141176a30`, which also stores the magnitude at `F+0x294`), `add_aero_force` (`0x140f26ef0`, "addFaero": builds a
frame from the direction, accumulates `F+0x2e8/0x2d4/0x2c0` and the moments `F+0x2fc/0x314/0x32c`, and pushes the
0x50-byte record to the log vector at `0x146125768` when `F+0x28` names the recorded object), and three small
curves (`0x141294180`, `0x141a6b610`, `0x140f29bb0`).

Read but not ported: the airflow wash `0x14117d970` (1400 instructions: a jet-wash loop over the engines and a wing-wake
loop over the 48 wings with downwash, swirl and radius ratios; it also calls `0x141186930`), the wind sampler
`0x141ba80a0` and the 11 KB tire-contact function `0x1411c8690`. `update_flight` (`0x1412656b0`, 8400 instructions
after removing diagnostics) assembles these: the wing and element loops, the body forces (`0x6040` table), landing
gear, radiators, arresting cable, speed brakes, chute, water rudder, anchor and water pick-up, then the force
totals already ported.

### Engine control update and engine family (`crates/xp-airfoil/src/controls.rs`)

`engine_controls` (`0x141260090`, 1836 instructions without diagnostics) is the per-frame engine orchestrator and is
ported completely, checked at four checkpoints and to the end (`tools/gen_controls_vectors.py`; the replay of the
recorded callees is generic: `tools/xp_vmcase.py`, `vm::Callees`). It: copies the override area to the propeller
totals when `F+0x6760` is set; otherwise computes each throttle group's lever as the weighted mean of its engines'
levers (`B+0xc90..`, with the `B+0xd88` curve), dispatches each engine to its kind's update (`0x14119ac90` kinds 0-4
when `F+0x6764` is clear, `0x141197b00` the engine update for kinds 5 and 6, `0x14119a570` kind 7), calls the
propeller force `0x1411bd470` for each part that is not held back, sets the lever of the propeller parts of kind 6
from the jet engines (`F+0x64b4/0x64b8`), eases the engines' power (`M+0x25c`) toward the propeller's, and then runs
the lever groups (`B+0xb78` groups: the lever position moves with the group's thrust, limited by `limit_a`/`limit_b`
and the rate follower `0x1410c9620`), the rates of change `M+0x7c`, `N+0x18`, `N+0x10` and the moments of the
propeller parts into `F+0x2f8/0x310/0x328`.

Also ported: `apply_engine_thrust` (`0x1411975a0`) and `update_engine_kind7` (`0x14119a570`, which reads the runtime
atmosphere table at `0x14612bd90`). The hold rule `engine_held_back` is used wherever the original inlines it (the
inline copies call the bindings in the same order). Not ported: `0x1411854a0` (a per-group query), `0x1411da6c0`, `0x1411c5a90` and the init `0x141190ed0` (replayed).

### Engine kinds 0-4 (`crates/xp-airfoil/src/piston.rs`)

`update_engine_piston` (`0x14119ac90(M, F, n, inputs)`, 888 instructions without diagnostics) is ported completely
and checked on 150 random objects (`tools/gen_piston_vectors.py`, `piston.txt`). Kind 0 only sets `M+0x24c/0x254/
0x258` to 1. For the other kinds: `M+0x58/0x60` follow `M+0x54/0x5c` at 0.2 times the frame time; the held lever
(`M+0x58` unless input `0xa8` is bound, against `M+0x60`) gives the power fraction `M+0x24c` (with the manifold ratio
from `B+0x934 * F+0x424 / F+0x68`, the 15 % lever term, the kind 1 lean term from `M+0x50`/`M+0x2b4` and the
squared half-power term), `M+0x254` (over 101325) and `M+0x258` (the density ratio over 1.225; both scaled by the
noise `0x140984e50` when input `0x1a1` is bound). Then the throttle gain `0x1411dd610` is blended with the lever
`M+4` (kinds 3/4: the larger; kind 4 with `B+0xa9c == 3` and `M+0x298 == 1` a second blend), scaled by `M+0x22c`
(the ramp: kinds 3/4 and others move it by the frame time over `B+0xa18`; kinds 0-2 set it to 1), optionally replaced
by an interpolation on `E+0x18` (`0x1411a2e40`), and limited for kind 3 (`B+0xa9c` 2) and kind 4 (3) by the extremes
of the bound elements' values against the engine ratio (factors 1.05, 0.95, 1.06). The kind handlers
(`0x14119bc00` kind 0 with the lever input, `0x14119d380` kinds 1 and 2, `0x14119c610` kind 4, `0x14119cb70` kind 3)
are called with the resulting lever; the kind 4 handler is ported (below), the others are replayed. For kinds 3 and 4, when `F+0x28 != 0` or
`F+0x6880 == 0`, the power `M+0xcc` is two curves (`0x14082b800`) of the engine speed `M+0x90`, from a bilinear
lookup of the temperature and altitude in the runtime atmosphere table.

`engine_ramp_limited` (`0x1411a2e40`) is ported (`B+0xaac`, the state's `+0x1e4` and the two bindings `0x2fb`/`0x239`).

The kind 4 handler `update_engine_kind4` (`0x14119c610(M, B, F, n, level)`, 311 instructions) is ported: the root
(exponent 0.5) and square of the speed ratio `M+0x78 / E+0x18` give the torque terms, the thrust term of the fuel
supply (`0x141238c20`) and the starter delay (`0x1411924e0`) come from the already ported helpers of `engine.rs`
through `engine_env` (the environment queries are the original's callees, replayed in the tests), the noise
(`0x1408bd9d0`, argument twice the global clock `0x142f01910`) scales the power by 0.95 + 0.05 noise, and for a
running engine `M+0x240/0x244/0x248` come from the lever with exponents 0.1, 5 and 3. The query mode argument of
the binding call (2 for the first fuel cut-off test of `0x141238c20`, otherwise 1) is not modelled by `EngineEnv`.

The kind 3 handler `update_engine_kind3` (`0x14119cb70(M, B, F, n, level)`, 469 instructions) is ported too: for a
running engine the lever is limited by the starter (`0x1411a2d90`, `starter_ready_of`) and the throttle curve
(`0x1411a2bf0`, the same code as `0x14082b800`), giving the manifold terms `M+0x240/0x244/0x248`; the lever goes
through the feedback blend on `B+0x9d0`, the 2/7 power and the 3.5 power of the speed give the load terms (with the
`F+0x400` term, the lubrication `M+0x2bc` blend and the bindings `0x1d1`, `0x181` (noise), `0x171` (cut) and `0x1b1`
(the starter delay)); the thrust term of the fuel supply, the friction (`M+0x268` over the 4.5 power of the speed)
and the frame time integrate the speed `M+0x90` (200 per second per unit of power over `B+0xa20`), `M+0x98` is the
speed ratio in percent, and `M+0xb8` is the power from the propeller curve `0x141a6a650`.

`propeller_curve` (`0x141a6a650(B, a, r, d)`, 311 instructions): the propeller power coefficient for the speed `a`
(percent), the speed ratio `r` and the density factor `d`. `B+0x990 == 0`: the 3rd and 1.4th powers of `a/100`
blended between 25 and 50 percent (a clamped line), the 4th power times 1.202 and the square of `r` times -0.2 (each
scaled by the blend and `d`), less 0.002 of `r` in percent, plus the difference of the squares of `a/100` and `r`
over `(1 + 0.05 a) (1 + 5 r)`. `B+0x990 == 1`: the density ratio from the runtime atmosphere table at the position
135.344 (entries 135 and 136 of `0x14612bd90`, i.e. altitude 8534 m, over 1.225), the exponents `B+0x998..0x9a4` extrapolated
along the line to `d`, and the power laws of `a` and `r` over `0.95 sqrt|r| + 0.05`. Any other value gives 0.

The kind 0 handler `update_engine_kind0` (`0x14119bc00(M, B, F, n, level)`, 548 instructions) is ported with its heat
rate `heat_rate` (`0x14119f090`): two banks (inputs `0x141` and `0x149`, both cut by `0x171`) get a drive from the
lever (or, below 1 percent lever on a cold engine, the shutdown drive) and their temperatures `M+0x1ac/0x1b0` follow
the net heat (the drive's square against a spread of `1.25` power of the speed ratio and the part's load fraction,
scaled by the cooling term `(T - ambient)/(B+0x1b18 - 15)` to the power 2.05); the engine temperature `M+0x1a4`
follows `heat_rate`, `M+0xb8` is the summed drive less the lubrication term `c58` times `E+0x10`, and `M+0x90/0x98`
are the speed in percent. Note the first-argument objects: the cooling reference `B+0x1afc`, limit `B+0x1b18` and the
size terms `B+0x7ac..0x7bc` are read from the aircraft object, not from the part.

The kind 1 and 2 handler `update_engine_kind12` (`0x14119d380(M, B, F, n, level)`, 1004 instructions; the two
piston types of the engine table, `RCP_CRB` and `RCP_INJ`: the Cessna 172 SP is kind 2) is ported too. The engine type
strings at `0x1427bd2f8` are, in order, `ELE` (kind 0, the heat model above), `RCP_CRB` (1), `RCP_INJ` (2), `TRB_FRE`
(3), `TRB_FIX` (4), then the jets. The handler: the lean/rich mixture response (`M+0x210/0x214`, a first-order
system with rate `M+0x20c` and decay time `2/ca`), the five altitude/temperature breakpoints `A1..A5 = K/(125 B+0xb6c)
+ 0.8 a + 0.2 d - 1` for `K` = 125, 105, 110, 85, 95, the throttle `M+0x40` from the starter, the manifold terms
`M+0x240/0x244/0x248` (each cut by the magneto switches: `M+0x48` selects which of the inputs `0x151`/`0x159` count),
the mixture-density lookups in the atmosphere table at the two sets of `B+0xb00..0xb14`, the power `M+0xcc`, the
manifold pressure terms (the 29.92 inHg of `0x14254d9c8`), the thrust term, `M+0xb8`, `M+0x90/0x98` and the lagged
fuel-air state `M+0xa0` (factor 4 times the frame time). 200 emulator cases (about 60 of kinds 1 and 2, 40 of
them running) are identical, NaN-free.

Not ported: the callees already listed as environment (frame time, bindings, atmosphere queries, noise,
the global clock).

### Engine control with the engine updates native

`engine_controls` now calls the ported engine functions itself instead of replaying them: `update_engine_piston`
(kinds 0-4), `update_engine_kind7`, `apply_engine_thrust`, and the small helpers `engine_has_mode` (`0x140822620`: a
switch over the modes 0 to 6, 1 or 0), `group_query` (`0x1411854a0`, with the integer power `0x141192820`),
`replay_active` (`0x1411c5a90`) and `engine_start_state` (`0x1411da6c0`: the start-up switch and the options
`B+0xc24..0xc34`; the simulation time is the global double at `0x142f01918`). Each helper has its own vectors
(`tools/gen_engine_small_vectors.py`, `engine_small_*.txt`); the four controls stages (`controls_1..4.txt`) are
regenerated so that the real engine code runs in the emulator under the orchestrator and only the leaf environment is
replayed: the input bindings, the frame time, the atmosphere queries, the noise, the random generator, the fuel
draw, and the unported callees (`0x141197b00` the kinds 5 and 6 update, `0x1411bd470` the propeller force on the
`Objects` layout, `0x141190ed0` the init).

### Blocks of `update_flight` and the body functions

Blocks are verified by running them from their start address to their end address in the emulator on random
objects (`tools/gen_flight_block_vectors.py`; the registers a block reads at its start are set by the case, and the
leaf callees are replayed): `flight_step::wing_aspect_pass` (`0x141265f7d..0x14126644a`: for each of the 48 wings
that is enabled and has `|W+0xf4| < 45`, the blending factors `X+0x288/0x28c/0x290` from the aspect ratio
`boundary_ratio` over the area factor), `thrust_effects_pass` (`..0x141266b52`: the engine controls call, the six
rocket forces `F+0x6518` 0x264..0x269, the pitch-tilt thrust `F+0x6514`, the moments `F+0x57c`, the blown-flap
factors `F+0x64bc/0x64c0`) and `element_pass` (`..0x141267978`: for each element of each enabled wing the air
velocity at the element (`0x14121b580`, replayed), resolved with the dihedral cosine and sine into the
cross-flow terms; the relative speed `X+0x50` (blended by `W+0x54`), the angle `X+0x28` wrapped to -180..180, the
sweep-corrected factor `X+0`, the counters `F+0x544..0x558` of elements with control surfaces and their mean
speed in knots (1.9438), the element force `0x1411b9840` (replayed) added to `X+0x294/0x298` and the moments
`F+0x314/0x32c`, and the aerodynamic force `0x140f26ef0` at the element point). The locals of the original's frame
are read and written at `rbp + offset`, as the environment calls fill them.

`part_force_pass` (`0x141269920..0x14126a791`, 110 cases; the repaired-value log calls `0x1405dcad0` and `0x141a67610`
are stubbed): the drag of the propeller-like parts. It runs when `B+0xc0c > 0.5` and `B+0xc10`, `B+0xc14` are
positive; every part record (`B+0x6010`, stride `0x3770`, count `B+0x91c`) gets the aircraft velocity
`F+0x29c/0x2a8/0x2b4` (a non-finite component counts as zero), plus, when `B+0xc08` is set, the rotation of
`(0, 0, N+0x84)` by the part's angles (`0x14120cf60`, stack argument 8 = 0 so the offsets are not added); with
`ratio = |v| / B+0xc0c` the force magnitude is `B+0xc14 * B+0xc10 * ratio^2 * F+0x6c`, times 0.5 and `M+0x44` in
double precision, applied along `v` at the part's offset `P+0x790..0x798` by `0x140f26ef0`. The register state at the
block start is `rsi = 0` and `xmm8 = 0.5` (double).

`rigid_body_step` (`0x1412709ff..0x141271fa2`, 120 cases; the engine flag, the frame time `0x140c448c0` (six separate
queries) and the flag inside `0x1407ce850` are replayed; `0x141964300` is a plain table read and `0x1408e25a0` and
`0x141176330` are the same non-finite-to-zero repair): the equations of motion. The totals in the aircraft axes
(`F+0x2f4` side, `+0x2e0` normal, `+0x2cc` axial) over the mass `F+0x288 * 9.798279762268066` give the accelerations
`+0x354/0x344/0x34c`; times the same constant they are rotated into the world axes by `0x1407ac020` (no origin shift)
into `+0x35c/0x360/0x364`, and gravity toward the planet's centre is subtracted (`F+0x78 * position / radius`, the
radius from the position with `6378145` added to the altitude; positions come from the doubles `+0x378/0x380/0x388`,
zero when the engine flag is set). Euler's equations with the inertias in the frame locals `rbp+0x1750`, `rbp+0x1758`
and `rbp-0x78` (`I1`, `I2`, `I0`) give the angular accelerations `+0x3c0 = (L - (I1-I2) w2 w3) / I0`,
`+0x3c4 = (M - (I0-I1) w1 w3) / I2`, `+0x3c8 = (N - (I2-I0) w1 w2) / I1` with `w = F+0x3cc/0x3d0/0x3d4` and
`L/M/N = F+0x30c/0x324/0x33c`. The velocities `+0x368/0x36c/0x370` and rates advance by `dt * acceleration` in double
precision. With `F+0x28 == 0`, `F+0x538 < 10` and one of the globals `0x145899fd0..fdc` set, the roll rate `+0x3cc`
is zeroed. With `F+0x24c` and both globals `0x145899fe0/4` equal to -1, an acceleration above 15, or an altitude
(`0x1407ce850`) below the limit `F+0x42f5c` (with, when the table flag `0x141964300(0x14611ac80, F+0x42f84)` is set, a
speed above 100), divides the velocities and rates by the acceleration and sets `F+0xda8`. The register state at the
block start is `r14d = 1`. The comparison of the test helper accepts a difference only between two normal floats,
because integers and flags that differ must not pass as denormals.

`set_position` (`0x141a6ace0(F, x, y, z)`, 150 cases; the terrain functions `0x141963d30`, `0x141962b00`,
`0x14195f4b0` and `0x141962750` are replayed with random results, so the glue around them is verified but not the
terrain logic): stores the position doubles `F+0x378/0x380/0x388` (a component whose float32 value is not finite
becomes zero), derives the ground-probe lengths from the vector `B+0x64f4..0x64fc` (`|b|`) and the distance
travelled in a step: `F+0x42f50 = 0.2 |b| + min(|b|, speed * dt)`, `F+0x42f54 = 2 |b|`,
`F+0x42f58 = 1.01 (|b| + F+0x42f50)`, then asks the terrain object at `F+0x42e40` for the ground under a
vertical segment through the position. With the probe state valid (`F+0x42f88`) the ground height `F+0x42f5c` is
taken from the probe, or, when the probe reports a hit, extrapolated from the cached plane
(`F+0x42f60..0x42f74`, divided by the slope term snapped away from zero by 0.01); otherwise the state is initialised
(`-500.0`, the cell `F+0x42f84 = -1`, a unit normal) and the height comes from `0x141962750`.

The attitude (`crates/xp-airfoil/src/attitude.rs`, 200 cases each): `euler_to_quaternion` (`0x1408816e0`, three half
angles from degrees through `sin` and `cos`), `quaternion_to_euler` (`0x140889cb0`: normalises the quaternion in place
by `1 / sqrt(max(|q|^2, 0.1))` and returns `a = atan2(2 (q2 q1 + q3 q0), q1^2 + q0^2 - q2^2 - q3^2)` wrapped to
`0..360`, `b = -asin(clamp(2 (q3 q1 - q2 q0)))` and `c = atan2(2 (q3 q2 + q1 q0), q0^2 - q1^2 - q2^2 + q3^2)`
wrapped to `-180..180`, all in degrees) and `integrate_attitude` (`0x140f5fb10`: the three rate increments, radians
times 57.29578, become a quaternion (arguments in the order of the third, second and first increment) that multiplies
the stored quaternion on the right, after which the Euler angles are stored). The trigonometry is the platform's
libm, so values agree within about a unit in the last place.

`integrate_motion` (`0x141271fa2..0x14127223b`, 100 cases; the engine flag, frame time, the terrain functions and the
record list singleton are replayed): the position is advanced by `dt * velocity * scale` (the double at
`0x142f01898`, three frame-time queries, each preceded by the position accessor) through `set_position`; the rates
times the frame time (three more queries) rotate the quaternion at `F+0x3e4` giving the Euler angles
`F+0x3e0/0x3dc/0x3d8`, whose sines and cosines are stored as the frame pairs `F+0x430/0x434`, `0x440/0x444` and
`0x450/0x454`; then every live record of the list at `F+0x69b8` (16 bytes each; live when it differs from the empty
key in `0x142f03778/0x142f03780` and `0x140f5c540` accepts it) is advanced by the frame time through the singleton at
`0x14578b780`. The register state at the block start is `xmm12 = 0.0174533` (the constant is loaded early in the
function).

`body_velocities` (`0x14127223b..0x141272395`, 120 cases; the airflow `0x14121b580` is replayed): the world velocity
`F+0x368/0x36c/0x370` moved into the aircraft axes by `0x141296750` (no origin shift) is stored at
`F+0x2a0/0x2ac/0x2b8`; the air velocity at the centre of gravity (the airflow at the origin, the wash switched off by
the last argument `r12d = 0`; its other three trailing arguments are the wash's excluded indices, -1) lands in
`F+0x29c/0x2a8/0x2b4`; `0x141183bf0` gives its direction angles (stored at `F+0x404/0x408/0x40c`, radians) and its
speed (`F+0x400`), and the three angles are then converted to degrees scaled by the speed held to `0..1`.

`geodetic_state` (`0x141272395..0x1412728f7`, 80 cases; the planet object `0x14193ae40`, the geographic conversion
`0x1406eaf20`, the local matrix `0x1419f7ee0` and the matrix-to-Euler `0x1419f6fd0` are replayed): the position becomes
the doubles `F+0x390/0x398/0x3a0`; the matrix (three rows of three floats 16 bytes apart at `rbp+0x15a0`) turns the world
velocity into the local velocity `F+0x3f4/0x3f8/0x3fc`; the Euler angles `F+0x358/0x350/0x348` (seeded from
`F+0x3e0/0x3dc/0x3d8`) change over the frame time into the rates `F+0x438/0x448/0x458` (radians per second), which
are smoothed into the accelerations `F+0x43c/0x44c/0x45c` by `lerp` with the factor `20 dt` (nine frame-time
queries); finally the position goes through the planet object's 4 x 4 double matrix (`+0x200..0x270`) into
`F+0x3a8/0x3b0/0x3b8`.

`flight_angles` (`0x1412728f7..0x141272a96`, 120 cases): from the local velocity `F+0x3f4/0x3f8/0x3fc` the flight path
angle `F+0x414 = atan(F+0x3f8 / max(sqrt(F+0x3f4^2 + F+0x3fc^2), 0.01))` and the track
`F+0x410 = atan2(F+0x3f4, -F+0x3fc)` (degrees, wrapped to `0..360`), the track of the world velocity
`F+0x418 = atan2(F+0x368, -F+0x370)` likewise, and the sine and cosine pairs of the angles `F+0x404` and `F+0x408`
at `F+0x460/0x464` and `F+0x468/0x46c`. 

`path_samples` (`crates/xp-airfoil/src/flight_state.rs`, `0x141272a96..0x14127307c`, 120 cases; the engine flag,
the height above the ground `0x1407d76f0`, the planet object and the geographic distance `0x1406e2be0` are replayed):
the takeoff and landing record, kept while the global `0x142f01920` (the sim speed) exceeds 1. While the speed is below
one knot (`m/s * 1.9438445`) the longitude/latitude pairs of the start points (`F+0x65b8/0x65c8`, `0x65d8/0x65e8`,
`0x65f8/0x6608`) follow the position; below 50 feet (`m * 3.28084`) the end point is refreshed and, when all six
doubles are set, `F+0x65b0` is raised; the second set (`F+0x65c0..0x6610`) works the same way with the flag
`F+0x65b4` and the speed above one knot. The four distances between pairs of these points (always computed) go to
`F+0x6618/0x661c/0x6620/0x6624` in feet. The register state at the block start is `xmm7 = F+0x368`,
`xmm9 = F+0x370` (floats) and `xmm11 = 0.0`.

The body functions (`crates/xp-airfoil/src/body.rs`), verified as functions: `body_aero` (`0x141a51600`: the
cross-flow forces of a body record from its lengths `+0x10/0x14/0x18`, end points, `|sin|` and `cos^4` of the angle
and the dynamic pressure; 300 cases) and `body_wave_drag` (`0x141a522d0`: the supersonic wave term of a gridded
surface, area-weighted mean normals of the triangle pairs of the grid cells with the Ackeret factor
`2/sqrt(M^2-1)`; 70 cases). A negative argument of the original's square root goes through a domain handler the
emulator cannot run, so the vectors keep those arguments non-negative.

`body_pass` (`0x141267978..0x1412686a9`, 39 bodies of `B+0x6040`): the record's reference point is rotated with
its angles (`+0x588` object: angles `+0x9c..0xa4`, offsets `+0x90..0x98`; third argument the lever curve), the air
velocity there (`0x14121b580`, replayed) is rotated into the body axes and converted to angles and speed
(`0x141183bf0`); `body_aero` gives three results, the wave drag replaces the third (blend factor from
`body_blend` of the Mach number, `root_ratio` and the cross-flow magnitude) unless the record's engine index is a
jet; `R+4` is set to the third result over the dynamic pressure times `R+0x10`; with `B+0x2894 > 0.01` and
`F+0xdac` the three results are scaled by 0.05; the force is applied along the air direction at the rotated point
by `0x140f26ef0`. 25 emulator cases identical. The argument order of `0x14120cf60`'s three inputs is
`(xmm1, xmm3, stack float)`, and its outputs go to `r8`, the fifth and the seventh argument.

`atmosphere_step` (`0x1412763c0(F)`, the first call of the step after the force clearing): from the altitude `F+0x3a0`
(zero under the engine flag) it sets the gravity `F+0x78 = 3.986012e14 / (6378145 + h)^2`, the temperature `F+0x5c`
(`0x141ba6750`), `F+0x58` (that temperature less `0x141ba6290`), `F+0x60` (the temperature less the table value of
the runtime atmosphere table at the altitude, second floats of the entries), the density ratio `F+0x6c`
(`0x141ba63a0` of the altitude `0x141ba6df0 * 0.3048 + h`), `F+0x70 = ratio / 1.225`, the pressure `F+0x68 = ratio *
287.053 * T` (Kelvin), the speed of sound `F+0x74 = sqrt(401.874 T)` and the total temperature `F+0x64 = (1 + 0.2 M^2)
T - 273.15` with the Mach number `F+0x420`. The three weather-object accessors are replayed. 150 cases identical.

`wing_chain_factor` (`0x14121a9b0(W, X, &list_a, &list_b, log)`, called from the first loop of the step with the lists of
chained wings, which it releases): see the function's doc comment; 120 cases identical (the vector release call
`0x1422e7c2c` is stubbed). The wash `0x14117d970` is ported in sections (`crates/xp-airfoil/src/wash.rs`: the jet
exhaust of the engines of kinds 5 and 6, and the propeller slipstream of the parts, each compared from the function
entry to a checkpoint, 80 cases): for an engine part the point is rotated into the propeller axes
(`0x141296900`, whose six stack floats are `sin, cos` of the angles `+0x79c`, `+0x7a0`, `+0x7a4`), the normalized
radius and axial position give the swirl and thrust velocity profiles, and the result is rotated back and added to the
three outputs after finite-value guards. The wing wake (`0x14117f79f..0x1411819c2`: per wing and element, a downwash
from two tangents blended by the body blend factor, a decay factor from the swirl term, with the excluded wing and
body area ratios) and the tail (the body shadow scales the outputs by `sqrt(1/2)` inside a body or by
`sqrt(1 - min(1, value))`) are ported too; 80 more cases, with the shadow function replayed (it writes a flag at
`rbp+0x670` and a value at `rbp+0x674`).

The body shadow `0x141186930(F, result, x, &out1, z, &out2, y, &out3, excluded)` and its ray-box test `0x141296c40`
are ported in `crates/xp-airfoil/src/shadow.rs` (300 and 28 cases, the latter with the input-binding query
`0x1407ace10` replayed). The ray starts at the point and runs along `-out`; for each of the 39 body records
(`B+0x6040`, stride `0x34c8`) that is not bound away (`0x1407ace10(F, 1, 0x179, body+0x5f0)` for indices up to `0x26`),
has `+0x54 == 0` and the enabled byte `+0x588`, is not the excluded body and has `+0x10` above the excluded body's, the
point is moved into the body frame (`+0x618..0x620`) and tested against the body box (`+0x58..0x6c`): inside, or a
slab hit of the ray. The 32 mesh boxes (`+0xa0`, stride `0x28`, a triangle count at `+0xc` and a first triangle at
`+8`, nine floats per triangle at `+0x70`) are slab-tested, and a point inside the body box also tries the reversed
ray; the triangles are Moeller-Trumbore tests in float32, giving bit 0 for a hit with `t >= 0` and bit 1 for `t < 0`.
Both bits (3) mean the point is inside the mesh (`result[0] = 1`); only bit 0 adds the body's `+4` to the value.
The slab test follows the original's `cmov` selection (the fourth vector component never contributes). The two debug
logging branches (`F+0xbcc8`, `F+0xbcd0`) are not ported. So the whole wash is ported.

Not ported: the radiator/gear loop of the step (it uses the terrain probe and
standard containers) and the gear contact function `0x1411c8690` (2300 instructions, terrain-driven; no DSF scenery
is available to verify it).
