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

`instruments` (`crates/xp-airfoil/src/flight_state.rs`, `0x14127307c..0x141273779`, 100 cases; the engine flag, frame time,
the magnetic variation `0x14076b5d0`, `0x141244b60`, `0x1407d7bc0` and `0x1407cc570` are replayed): the cockpit quantities.
`F+0x6638` and `F+0x663c` accumulate the horizontal and the total distance (a double increment added to the float),
`F+0x41c` is the equivalent airspeed in knots (`sqrt(F+0x70) * speed * 1.9438445`), `F+0x420` the Mach number,
`F+0x424` the dynamic pressure (`speed^2 * F+0x6c / 2`), `F+0x428` and `F+0x42c` the replayed variation values; the
instruments `F+0x500/0x504` (the angles `F+0x404/0x408` scaled by a ramp of the speed between 5 and 10),
`F+0x508` (the heading difference wrapped to -180..180), `F+0x50c`, `F+0x510` (the normal acceleration held to
-0.1..1), `F+0x514` are blended toward their targets by `lerp` with rates from the frame time (divided by
`B+0x24a4` or `B+0x2498` for some); then the rates of change `F+0x51c/0x52c` (factor `10 dt`) and `F+0x534/0x53c`
(factor `dt`) are smoothed derivatives of the speed-scaled flight path angle, the scaled angles `F+0x404/0x408`
(stored at `F+0x518`, `F+0x520`, `F+0x528`), the replayed load term and the ground speed in knots. The register state
at the block start is `xmm8 = 1.0` (double) and `xmm11 = 0.0`.

`force_coefficients` (`crates/xp-airfoil/src/flight_state.rs`, `0x141273779..0x141273dfb`, 140 cases; the per-record query
`0x141a6ba30` and the frame time are replayed): `F+0x258` is set when the type word `F+0x42f84` is 2 to 6
(`0x141964140`: `idx - 2 <= 4` unsigned) and `F+0x25c` when `F+0x24c` is set and it is not. The aircraft-axes
forces `F+0x2c0/0x2d4/0x2e8` are rotated by the pairs `F+0x460/0x464` and `F+0x468/0x46c` into `F+0x298`
(`2d4 c464 - 2c0 c460`), `F+0x2b0` (`2e8 c46c - 2c0 c468`) and `F+0x2a4` (`2c0 c464 + 2d4 c460 + 2e8 c468`); over
`F+0x424 * B+0x2880` they give the coefficients `F+0x65a4/0x65a8` and `F+0x65ac = F+0x298 / F+0x2a4`. For an
aircraft heavier than 0.99 of `B+0x288c` with the positive parameters `B+0x28e8/0x28ec/0x28f0`, the estimate
`B+0x28f4` (an angle in degrees, clamped to -100..100) is computed from half the loaded mass and a wind term
(`B+0x64f4` times 0.5, or 0.33 and a weighted mix of nine fractions at `F+0xbbb4`, each queried through
`0x141a6ba30`, when `B+0` is at least 0x4b0), then integrated twice (`B+0x28f8`, `B+0x28fc`, both clamped, double
precision with the frame time). The register state at the block start is `rdi = 0`, `rsi = 1`, `xmm8 = 1.0` and
`xmm10 = 0.5` (doubles).

`late_state` (`crates/xp-airfoil/src/flight_state.rs`, `0x141273dfb..0x14127402d`, 120 cases; the fuel and load update
`0x141245750` (550 instructions), the engine update `0x14125e4f0` (a large function) and the input-binding query are
replayed): with `B+0xc54` the former runs, otherwise `F+0x6490`/`F+0x64a8` are cleared and `F+0x6494` takes `F+0x5c`.
With the aircraft feature `B+0x8c4` and the global `0x142f01978` set, the control assist `F+0x6594` is computed:
with `B+0x8c8`, `ramp(F+0x404 / B+0x8ec between 0.75 and 1) * (signed_sqrt(F+0x41c / B+0x7b0) - 0.25) * 1.3333334`
unless the binding query with mode 2 is active (then 0); without it, 1 when the mode 1 binding is inactive and either
`F+0x6c98` exceeds a quarter of `B+0x7b0` with `F+0x404 > B+0x8ec` or the time `F+0x6f4c` is later than the sim time
`0x142f01918` (otherwise 0). With `F+0x28 == 0` the engine update runs. With `F+0xdbc` the angles `F+0x3d8` (to +-45)
and `F+0x3dc` (to +-20) are limited by replacing an out-of-range value with the limit (NaN passes) and the quaternion
`F+0x3e4` is rebuilt from `F+0x3e0`, `F+0x3dc`, `F+0x3d8`. The register state at the block start is `rdi = 0`, `rsi = 1`.
After this block the function only looks up a name for the aircraft (a loop over 19 entries and the string copy
`0x1407debf0` into `F+0x2c`), which is not ported.

`arm_probe` (`crates/xp-airfoil/src/flight_state.rs`, `0x14126a791..0x14126aad6`, 120 cases; the terrain probe `0x14195ffc0`
and the engine flag are replayed): the angle `F+0x6548` of the arm described by `B+0x4440..0x4454` takes the demand
`F+0x6528` through `interpolate_clamped` between `B+0x444c` and `B+0x4450`; with `F+0x28 == 0` and `B+0x4454 > 0`
five iterations place the arm tip in the world, ask the terrain probe and, on a hit, lower the angle by
`(probe height + 0.1 - tip height) / B+0x4454` in degrees, held to the limits. It returns `esi` (1 once any tip was
below the surface), which the next block reads. The block starts with `rbx = B`.

`gear_aero` (`crates/xp-airfoil/src/flight_state.rs`, `0x14126883f..0x141269920`, 50 cases): the aerodynamic drag of the ten
gear records (`B+0x6080`, stride `0x88`). A record with a kind word is skipped when its animation state
(`0x1407d6c50`: entry `i` of the vector at `F+0x6958`, `0x90` bytes each) is retracted (`+0x10 < 0.01`) and
`B+0x2830` is not positive. `found` (the original's `esi`) becomes set when a live body record names the gear
(`body+0x5f4 == i`); it stays set across skipped gears and is cleared after a force is applied. The drag areas come from
the record (`+0x58 * 0.2` (1 for kind 1), capped by `+0x5c`, times `+0x18`, doubled; and `4 * +0x58 * +0x5c` scaled by the
kind: 3 x2, 4 x1.2, 5 x2.4, 6 x2.8, 7 x4, 1 x0, and x0.25 when no flag and `found`), are multiplied by the dynamic
pressure `F+0x424` and `|e|^0.1` of the extension (the first two terms use at least `B+0x2830`, the third does not) and
summed with a term `B+0x2834 * q * e / rbp[0x1758]` for records with the second flag. The sum is applied by `0x140f26ef0`
along the air velocity at a point blended from the record and the state's own offsets. The block between the gears'
force and the debug flag is a log. Inputs: the frame slot `rbp+0x1758` (computed before the loop from the wheel groups),
`xmm7 = 0`, `esi = 0`.

`wheel_groups` (`0x1412686a9..0x14126883f`, 80 cases): the count of gear records with both leading words nonzero, stored as a
float at `rbp+0x1758` for `gear_aero`.

`hook_state` (`crates/xp-airfoil/src/flight_state.rs`, `0x14126aad6..0x14126b3ee`, 100 cases; the hook object `0x1411b63a0`, the height `0x1407cd810`, `0x1408be280`, `0x1408ce690`, `0x1408625a0`, the wire probe `0x1411e14d0`, the pull `0x1408e3230` and the engine flag are replayed):
the arresting hook. The geometry table at `0x14578b040` and the state globals `0x14589a000..0x14589a020` (wire index,
second index, progress, engagement point) are read and written as memory. With `F+0x28 == 0`, the object and
`B+0x4454 > 0`, an engaged hook (`[0x14589a000] >= 0`) eases the arm angle `F+0x6548` toward the wire direction (a clamped
interpolation weighted by the height) and applies the pull; the tips are placed in the world
(`F+0x654c/0x6554/0x655c` from the arm base, `F+0x6550/0x6558/0x6560` from the tip, each through the aircraft-frame
rotation plus the position with three engine-flag queries); when `esi` (from `arm_probe`) is set, `F+0x6528 > 0.9`
and the height exceeds 10, the three wires are probed and the last engaged one is recorded (`[0x145899ffc] = h^2 / 200`);
a release test (height below 1 with the demand below 0.5) and the hold timer (`+0.1 dt` per step, ending the
engagement at 1) ease the engagement point. The register state at the block start is `xmm8 = 0.5` (double), `xmm10 = 0.5`,
`xmm12 = 1.0` (double), `r13 = -1`, `r14 = F+0x6548`.

The replay test helper now also compares the float and stack arguments of replayed calls (floats within a relative 1e-5).

`tow_and_records` (`0x14126b3e8..0x14126b548`, 120 cases; the singleton calls `0x140f42620`, `0x140f39ae0` and the record lookup `0x140f5c540` are replayed): with `B+0x2894 > 0` and `F+0xdac == 0` a pull
`B+0x2894 * 9.798 * F+0x6590 * ramp(F+0x70)` (the ramp is 1 at `0.01` and 0 at 0) along the second axis is moved into
the aircraft frame and added as an axial force with arm `B+0x289c`, a side force and a normal force; then every live
record of the list at `F+0x69b8` is handed to `0x140f39ae0` through the singleton.

`float_drag` (`0x14126b548..0x14126b757`, 100 cases; the airflow `0x14121b580`, called with the wash switched on, is replayed): while
`F+0x148 > 0.01` each of the four float-section records at `B+0x6098` (stride `0x1c8`) with a nonzero first word gets a drag
`|v|^2 * 1.23 * sin(clamp(|+0x1bc - +0x1b0| - 1, 0, 180) deg) * (+8) * F+0x6c / 2` applied at its point
(`+0xc/0x10/0x14`) along the air velocity there by `0x140f26ef0`. Register state: `xmm12 = 1.0` (double), `r13 = -1`,
`r14 = 1`, `esi = 0`.

`float_waves` (`0x14126b757..0x14126bf30`, 80 cases; the airflow, the wave function `0x1408bd9d0` and the frame time are replayed): three float
sections (point `B+0x3f40/0x3f4c/0x3f58`, area `B+0x3f64`, strength `B+0x3f70`) keep a smoothed air velocity at
`F+0x6564/0x6570/0x657c`. With the water switch `F+0x650c` clear the stored values follow the points of the sections whose
area is positive and `F+0x652c` is cleared. With it set, `F+0x652c` rises by `0.5 dt` to 1; every section with a positive
area (the loop head tests the area, not the strength) takes the air velocity at its point (wash off), is pushed by the other
sections (`wave` noise `w * 0.5 + 1` times the stored offsets, a separation measure from the radii `sqrt(area / pi)`,
`0.1 * speed * (offset / distance) * clamp(1 - distance / (2 * radii))`), blends the stored velocity toward the result
(`F+0x652c * strength * v / max(|v|, 1)`) with factor `dt` and applies the drag `1.2 * area * F+0x652c * |v|^2 * F+0x6c / 2`
by `0x140f26ef0`. The frame slot `rbp+0x1750` holds `F` in the original.

`world_pull` (`0x14126bf30..0x14126c034`, 100 cases; `0x1411e4bd0`, `0x1409057f0` and `0x1411d9d80` are replayed): with `F+0x28 == 0`
two interface updates run; with `F+0x28 == 1` and one of the globals `0x145899fd0/fd4` set the force
`-(0x145899fc4, fc8, fcc)` is added as a world-axes force at `(0, B+0x2638, B+0x263c)`. The larger in magnitude of
`F+0xf0` and `F+0xfc` plus `F+0x108` becomes the float at `rbp+0x1750` (`F+0x108 + F+0xec` when `F+0x3c == 1` and
`0x1411d9d80(F)` is zero).

`gear_targets` (`0x14126c034..0x14126c4e4`, 60 cases; `0x1411daa80` is replayed; the key table at `0x1460e9708` is read as memory): unless
`F+0x28 == 0` and `F+0x689c` is set, every gear record (`B+0x6080`, stride `0x88`, ten) with a kind other than 0 and 1 and a
live target (`+0x50` or `+0x54` above 0.01, or `+0xc` set) writes the target `+0x24` of its animation entry
(`F+0x6958`, `0x90` bytes each): `F+0x218` when `F+0x214` is set; when a key of the table (code `0x25` or `0x49`, enabled
by the parallel table at `+0xba44`, 500 slots) is held, `(F+0x20c + F+0x208) * +0x50 + pedal * +0x54` held to
`+-(+0x50)`; otherwise `pedal * clamp(+0x50 - (+0x54 - +0x50) * (sqrt(clamp(1 - (1 / B+0x1e94) * (max(first.+0x58, 0.01) * entry0.+0x40 * 1.9438445))) - 1))`
between the two (0.5 under the root when `B+0x1e94` is zero). With `B+0xe78` the value is multiplied by `0x1411daa80(F+0xbdd8)`;
it is then multiplied by the entry's extension `+0x10` and by -1 when the point `p0.+0x7c - sin(entry.+0x18) *
(p0.+0x18 - (1 - ext) * p0.+0x20)` is above zero. `pedal` is the float at `rbp+0x1750` (also in `xmm6` at the block start; it
is reloaded from the slot after every gear). The register state at the block start is `xmm7 = 0.01` (double), `xmm10 = 0.5`, `rsi = rdi = 0`.

`steering_state` (`0x14126c4e4..0x14126c7dc`, 100 cases; `0x1417dacf0` (two queries), `0x14123d110`, `0x1411e3ee0`, `0x1408625a0` are replayed): only while `F+0x3c == 1`.
Unless blocked (`F+0x28 == 0` with `F+0x6824`; the globals `0x142fe2a48`, `0x1460e0a1c`, `0x1460e0a6c`; a held key of code 6
or 7 in the key table; the queries with `0x1d9` and `0x1da`; `B+0x2840 < 0.01`; `F+0x23c != 0`), the demands `F+0x240`
(left) and `F+0x244` (right) follow the pedal `rbp+0x1750`: the signed ramp `interpolate(|p|; 0.5 -> 0, 1 -> 1) * sign(p)` is
stored at `rbp+0x1758`, the speed ramp `interpolate(F+0x41c; 0 -> 1, r -> 0)` (`r` from `0x14123d110`) gives a root, and
`0x1408625a0(signed, B+0x2840)` scaled by it either replaces the left demand alone (toggle off) or splits the average of
both (toggle on, `0x1411e3ee0(0x1460e0a10, 0x4e)`); both held to `0..1`. With `B+0xe94`, `F+0x238 == 0` and both demands
above 0.9 the latches `F+0x228`, `F+0x22c` (only with `B+0xe90`) and `F+0x220` are cleared; with `B+0xe90 == 1` and
`F+0x22c == 1`, `F+0x220` and `F+0x224` take the maximum of the demands and `F+0x224`. Register state: `xmm9 = 0x7fffffff`
(the abs mask), `xmm7 = 0.01` (double), `xmm10 = 0.5`, `r12` = the key table, `rdx` = its end.

`gear_state_update` (`0x14126c7dc..0x14126d1f2`, 40 cases; the bindings `0x72`/`0x73`, `0x1411daa80`, `0x140816eb0`, the planet object and `0x1406eaf20` are replayed):
the brake state of the ten gear animation entries (`F+0x6958`, `0x90` bytes): each entry's `+0x50` is cleared, then for
gears whose record has `+8` set it takes `F+0x224`, adds the left demand `F+0x240` when the wheel's lateral offset
`sin(entry+0x14) * (p0+0x18 - (1 - ext) * p0+0x20) * cos(entry+0x18) + p0+0x64` is below -0.01 (cleared again when binding `0x72` is
set) and the right demand `F+0x244` when it is above 0.01 (binding `0x73`); with `B+0xe7c` the history
`F+0xbdd8` limits and smooths it (`0x1411daa80`, decay `3.858e-6` with the global `0x142f01968`, `0.25 *` the change of the
history slot `F+0xbe00+4i`), and the final value is held to `0..1`. Unless the sim speed `0x142f01920 >= 1` with `F+0x24c` set and
`F+0x64e4` set, the point `(B+0x280c, B+0x2814, B+0x2810)` is converted to the world and then to the geographic doubles
`F+0x64e8/0x64f0/0x64f8`. Then `F+0x24c/0x250` are cleared, the floats `+0x2d0/0x2d4` of each enabled wing's element record are
cleared, and each gear entry with a kind takes `+0x20 = record+0x24` and clears `+0x2c..0x40`, `+0x64`, `+0x5c`.
Register state: `xmm7 = 0.01` (double), `xmm9 = 0x7fffffff`, `xmm12 = 1.0` (double), `r14 = 0`.

`wheel_contact` (`0x14126d1f2..0x14126d5d3`, 60 cases; `0x1411878f0`, the tire function `0x1411c8690` and the strut function `0x1411c7a50` are replayed, their terrain logic is not
verified): the latch `F+0x224` moves toward `F+0x220` at 1 per second (10 when it is above the target and the latches
`B+0xe90`/`F+0x22c` do not both hold; not at all while both hold and it is above or `B+0xe90 >= 2`), `F+0x64c8` eases toward `F+0x64c4`
by `2 dt`. Each of the ten gears with a kind goes to the tire function `0x1411c8690(entry, F+0x28, i)` when it is not of kind 1 and
`|B+0x65a8|`, `|F+0x348|`, `|F+0x350|` are all below 45 degrees; otherwise to `0x1411c7a50(F, i, ...)` with the foot point computed
from the entry's pose. The returned contact counts add up; `F+0x24c = count > 0`, `F+0x250 = count >= 3`, `F+0x64cc = pedal * B+0x2808`.
Register state: `xmm9 = 0x7fffffff`, `xmm11 = rad`, `xmm12 = 1.0` (double), `rcx = B`.

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

## Wing strip contact probe (`0x14126d5d3..0x14126dd98`)

Ported as `flight_state::wing_ground_probe`. Early out (to `0x14126ef6b`) unless `F+0x42ec8 > altitude - |wind|`; otherwise loops the 48 wings, skipping those bound by `0x1407ace10(F,1,0x251,i)` or with byte `+0x678 == 0`, and issues three `0x1411c7a50` calls per wing from the normalised strip direction. Verified against the emulator (30 cases, 26 entering the loop). Not established: the stack slots beyond the fourth (`0x40`, `0x48` and the two pointer arguments) of the `0x1411c7a50` calls are not compared.

## Body surface contact probe (`0x14126de36..0x14126e4c8`)

Ported as `flight_state::body_surface_probe`. For a body with `+0x5f4 == -1` whose last row rises above `0.05 * B+0x64fc`, every point of every row (`+0x660 + 0xd8 a`, `+0x658` points) is rotated by the body's Euler angles (`+0x9c/0xa0/0xa4`, `rotate_pairs`) and offset (`+0x90..0x98`), and `0x1411c7a50(F, -1, ...)` is called with the point and the direction from the row centroid, scaled to half the body height (`+0xc`; a point within 0.01 of the centroid uses `z - (+0x24 + +0x28)/2`). A nonzero reply sets `F+0x24c`. Verified on 80 emulator cases (206 replayed calls); the sine/cosine come from the platform libm, compared at 1e-5 relative. Mutation checks catch the centroid, the middle height sign and the flag value; the 0.01 and 0.05 thresholds are not probed near their edges. Not established: the callee's remaining stack arguments (tenth of drag, null pointers).

## Body contact response (`0x14126e4c8..0x14126e974`)

Ported as `flight_state::body_contact_blend`. For body `j` the callee `0x1411cb1e0(F, body, body+0x588, body+0x630, j, edi, r12, &o1, &o2, &o3, &acc_a, &acc_b)` is replayed for every row pair `edi < +0x654 - 1` and `r12` from `trunc(n/4)` while `r12 <= 0.75 n` (`n = +0x658`); a nonzero answer sets `F+0x24c`, `F+0x250`, `rbp-0x78` and `body+0x2c`, and keeps the contact with the lowest and the highest `o3`. Without a contact the body's `+0x30/+0x34/+0x38` are cleared. Otherwise `+0x30..+0x50` blend toward `acc_a`, `acc_b`, a speed-scaled `acc_a / max(acc_b, 0.01) * min(1, (|v_xy| - 1) / 9)^2`, and the lowest/highest contact values, each with a fresh `clamp(2 * frame time, 0, 1)`. Verified on 60 emulator cases (235 callee and 360 time replays); mutations of the trackers, the clamp of `acc_a`, the speed term, the flags and the loop start are caught. Not established: what the callee computes, and the pointer outputs beyond the first (compared only up to four stack slots).

## Body contact pass (`0x14126dd98..0x14126e9a0`)

Ported as `flight_state::body_contact_loop`, the driver over the 39 bodies (`[B+0x6040]`, stride `0x34c8`) that runs the surface probe and the contact response above. A body is skipped when its model index (`+0x5f0`, up to `0x26`) is bound by `0x1407ace10(F, 1, 0x179, index)`, when `+0x54` is nonzero, or when the byte `+0x588` is clear. The counter lives in `rbp+0x1750`. Verified on 7 whole-pass emulator runs (the two ported blocks run in the emulator too, with the callees replayed); a mutation of the `+0x54` test with the values 0/1 is not distinguishable. The test data file is large because every body is preset.

## Ground response after a body contact (`0x14126e9a0..0x14126ef74`)

Ported as `flight_state::ground_response`; it runs only when the contact flag `rbp-0x78` is set. With `B+0x2804` and `B+0x2808` positive, the ground velocity (the point `(-B+0x2800 * F+0x3d4, -B+0x2800 * F+0x3d0, 0)` moved out of the aircraft frame by `0x1407ac020` without origin shift, added to `F+0x368/0x370`) gives a bearing through `atan2f`, wrapped to ±180 against `F+0x3e0`; its sine against the heading `F+0x64cc` drives a moment `f(knots) * speed * sin * (|sin| * speed * B+0x2804 * 1000) * F+0x64c8`, with `f` the `0x1406ea0b0` line from `(0, 1)` to `(B+0x7b0, 0)`, clamped to `±B+0x288c` and applied with `0x1408e3230`. With `F+0x64e4` set, the stored geographic point is converted with `0x140913e60`, `0x140816eb0` and `0x141296750`, and the vector to it, scaled by the `0x1406ea0b0` line `(0, 0)..(25, 0.1 * 9.798 * B+0x2898)`, is applied as a force (`0x1408e3230`) and a squared reach (`0.1 * B+0x64ac`)² is passed to `0x14119e5b0`. Then, with `B+0x28dc` set and `F+0x64dc > 0.01`, a brake force `1.25 * speed * (B+0x2890 / 10 * F+0x64dc)` is applied and `F+0x28c` grows by `frame time * scale` within `0..B+0x2890`. Verified on 100 emulator cases (51, 34 and 18 of the three force calls); mutations of the constants are caught except the knots factor (one ulp), the ±180 wrap edge and the last bits of rounding. Not established: what the force callee `0x1408e3230` does with its arguments (replayed).
