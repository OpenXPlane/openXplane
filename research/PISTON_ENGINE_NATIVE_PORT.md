# Piston engine: what a native port has to reproduce

Source: `crates/xp-airfoil/src/piston.rs` (`update_engine_piston`, matched against the original on random
objects, see the module header) and `crates/xp-airfoil/src/engine_*.rs`. The flight loop does not use it:
`FlightModel::forces` has an assumed thrust law (`flight.rs`, `PROP_EFFICIENCY`, `PROP_SPEED_OFFSET`, static
thrust cap). This note is the starting inventory for Phase 1 item 5, read from the existing code. Field names
below are offsets of the original structures; none is named yet.

## Why it cannot be dropped in
The routines run on the emulator (`Vm`, `Callees`): they read and write the original's objects at fixed
offsets (`F+0x20`, `B+0x5ff8 + n*0x68`, `M+0x24c`, ...) and call original functions for the frame time
(`0x140c448c0`), input bindings (`0x1407ace10`), temperature and density (`0x141ba6750`, `0x141ba64e0`)
and the noise source (`0x140984e50`). A native model needs those objects as typed Rust state.

## Stages of one update (`0x14119ac90`)
1. Engine kind `E = *(B+0x5ff8 + n*0x68)`: 0 sets the three terms to 1 and skips the power stage; other kinds
   run stage 2.
2. `piston_power`: two commanded values (`M+0x54/0x5c`) approached at 0.2 per second into `M+0x58/0x60`; a
   manifold/power term `M+0x24c` from the lever, the ratio `2*x0/max(F+0x68, 0.01) + 1`, a 15 % lever
   reduction, a 10 % reduction for kind 1 and a mixture-style lean factor; a pressure term
   `M+0x254 = M+0x24c * F+0x68 / 101325`; a density term `M+0x258 = density/1.225 * M+0x24c` from the
   temperature query with a heat offset; optional random scaling `0.75 + 0.1*noise` when a binding is set.
3. `piston_levers`: a lever level `M+0x22c` (1 for kinds 0 to 2, a ramp for kinds 3 and 4), the throttle
   ceiling, the commanded value `x6` blended between the lever `M+4` and a ceiling, a ramp limiter, kind 3
   and 4 limits against the bound elements, then the per-kind handlers `update_engine_kind0/12/4/3`.
4. `power_from_tables` (kinds 3 and 4): power from the altitude and temperature tables `B+0xb1c..0xb34`
   and the engine speed `M+0x90`, written to `M+0xcc`; the result is `M+0xcc / M+0x25c`.

## What the C172 SP needs (read from the installed `.acf`)

- `_engn/0/_type RCP_INJ`: kind 2 in the original's table (`ELE` 0, `RCP_CRB` 1, `RCP_INJ` 2, `TRB_FRE` 3, `TRB_FIX` 4;
  `research/FLIGHT_STEP.md`). So the path is stage 2 and 3 with `update_engine_kind12`; `power_from_tables`
  (kinds 3 and 4 only) and the kind 3/4 limits are not used.
- `_engn/0/_RSC_max_pwr_ENGN 2500`, `_RSC_redline_ENGN 2700` (rpm), `_power_max_limit 178`, `_thrust_max_limit 0`,
  engine health 1.0, no clutch, no freewheel.
- `_prop/0/_prop_type 0`, `_des_rpm_prp 2700`: a fixed-pitch propeller, no governor.
- Fixed units are not stated in the file; `178` is read as horsepower (the 172 SP is rated 180 hp) and the
  speeds as rpm, both unverified.

Consequence: a native port needs `update_engine_kind12` (1004 instructions in the original, ported at
`engine_*.rs`) plus the `piston_power`/`piston_levers` stages above and `prop::prop_force`, not the whole engine
table. Naming the offsets that this path reads is the next step; the kind 12 notes in `research/FLIGHT_STEP.md`
already describe its outputs (`M+0xcc` power, manifold terms `M+0x240/0x244/0x248`, speeds `M+0x90/0x98`).
A trajectory recording from the original (Phase 1 exit) is needed to check any result.

## Naming the offsets
The offset column of `crates/xp-acf/data/object_schema.tsv` is **hexadecimal** (an earlier attempt read it as
decimal and got unrelated names; that was a parsing mistake, not a schema problem). With the right reading the
`B` offsets of the kind 1/2 path resolve to ACF properties, and the engine record `E` (record `R5ff8`, stride
0x68) resolves too: `E+4 _power_max_limit`, `E+0x14 _RSC_max_pwr_ENGN`, `E+0x18 _RSC_redline_ENGN`,
`E+0x64 _engn_health_var` (so `E+0xc` and `E+0x1e4` are not engine-record ACF fields).

| Offset | ACF property (`acf/_...`) | Stock C172 SP value |
|---|---|---:|
| `B+0x934` | `prop_inlet_recovery_v12` | 0 |
| `B+0x938` | `crit_alt_mtr` | 0 |
| `B+0x93c` | `engine_pwr_rat_from_crit_alt` | 0 |
| `B+0x958` | `num_cylinders` | 4 |
| `B+0x9e0` | `throt_max_fwd_nrm` | 1 |
| `B+0x9f0` | `engine_friction_ratio` | 1 |
| `B+0x9f4` | `RSC_idlespeed_ENGN` | 500 |
| `B+0xab0` | `fadec_EQ_mixture` | 0 |
| `B+0xb00` / `0xb04` | `SFC_alt_hi_PRP` / `SFC_alt_lo_PRP` | 12000 / 2000 |
| `B+0xb08` / `0xb0c` | `SFC_full_hi_PRP` / `SFC_full_lo_PRP` | 0.425 / 0.445 |
| `B+0xb10` / `0xb14` | `SFC_half_hi_PRP` / `SFC_half_lo_PRP` | 0.36 / 0.38 |
| `B+0xb18` | `ff_rat_idle_PRP` | 0.15 |
| `B+0xb6c` | `SFC_recip_rich_ratio` | 1.1 |
| `B+0xa18` | `fuel_intro_time_prop` | 15 |
| `B+0x1a64` | `max_MP_limit` | 29.9 |

Not resolved by the schema: `B+0x920` (bound-element count), `B+0x9dc`, `B+0xa9c` (engine mode), and the `M`
(engine state) and `F` (flight) offsets, which are runtime state rather than ACF fields. The values are read
from the installed file; the interpretation of each term is the existing port's, not re-derived here.
Note that `B+0x938` and `B+0x93c` are 0 in the stock aircraft, so `piston_levers`' `x11 = (B+0x93c - 1)/1.7 + 1`
is 0.412 for it.

## Engine state `M` fields, as already documented
Meanings collected from `research/FLIGHT_STEP.md` and the port code; they are the earlier passes' readings, not
re-derived. Offsets not listed are not documented anywhere in the notes.

| Offset | Role |
|---|---|
| `M+4` | the throttle lever (input to `piston_levers`) |
| `M+0x40` | throttle from the starter (kind 1/2 handler) |
| `M+0x48` | selects which magneto inputs (`0x151`/`0x159`) count |
| `M+0x54/0x5c` -> `0x58/0x60` | commanded values and their lag (0.2 per second) |
| `M+0x78` | engine speed used for the torque terms (ratio to `E+0x18`) |
| `M+0x90/0x98` | engine speed in percent (integrated: 200 per second per unit of power over `B+0xa20`) |
| `M+0xa0` | lagged fuel-air state (factor 4 times the frame time) |
| `M+0xb8` | power from the propeller curve / summed drive less lubrication |
| `M+0xcc` | power (output; `M+0xcc / M+0x25c` is the return value, stored at `M+0x1d8`) |
| `M+0x20c`, `M+0x210/0x214` | rate and state of the lean/rich mixture response |
| `M+0x22c` | lever level (1 for kinds 0 to 2) |
| `M+0x240/0x244/0x248` | manifold terms, cut by the magneto switches |
| `M+0x24c` | power fraction (with manifold ratio) |
| `M+0x254` | pressure term, `M+0x24c * F+0x68 / 101325` |
| `M+0x258` | density term, density ratio times `M+0x24c` |
| `M+0x25c` | engine power the propeller is eased towards |
| `M+0x264` | heat offset added to the outside temperature query |
| `M+0x268` | friction term |
| `M+0x2b4`, `M+0x50` | kind 1 lean term inputs |
| `M+0x2bc` | lubrication blend |

Undocumented but read: `M+0x64`, `0x68`, `0x6c`, `0x70`, `0x74`, `0x84`. They need the setup code that writes them.

### Roles read from `update_engine_kind12` and `piston_levers`
Read from the code of the existing port (`piston.rs` lines 250-251, 855-890, 1065-1070); roles are inferred from
how each field is used, not from names in the original.

- `M+0x74`: running flag (integer); a non-running engine skips `kind12_running` and the table power is
  multiplied by it.
- `M+0x70`: engine-enabled flag; zero forces the lever level `M+0x22c` to 0 for kinds 3 and 4.
- `M+0x6c`: gate of the lean/rich response: only when nonzero are `M+0x210` and `M+0x214` updated (rate
  `M+0x20c * 0.3 * M+0x40 * (0.8 * lever + 0.2)` and a decay with time constant `2/ca`, `ca` being the speed over
  the idle speed `B+0x9f4`, clamped to `0.001..1000`).
- `M+0x64` and `M+0x68`: current and previous value of one quantity. Each frame `M+0x68` is set to `M+0x64`,
  and before that, for kind 1 only, a drop (`M+0x68 > M+0x64`) adds `(M+0x68 - M+0x64) * 0.3` to the lean
  state `M+0x214`, limited to `0..10`. This is a throttle-closing transient, not a steady term.
- `M+0x84`: a phase angle in the ignition pulsation: `sin(M+0x84 * 0.0174.. * cylinders)` times a ramp of
  `1 / cylinders` that rises with `|M+0x78|` up to half the idle speed `B+0x9f4`. It matters at low speed
  only; with 4 cylinders and idle 500 rpm it vanishes above 250 rpm.

Not yet known: what the setup code writes to `M+0x64` (the quantity whose drop is detected), and the units of
`M+0x84` (the factor `0x3c8efa36` is `pi/180`, so degrees).

## Progress: the power stage is native
`crates/xp-airfoil/src/piston_native.rs` now holds the power stage (`piston_power`: the lag of `M+0x58/0x60`,
and the terms `M+0x24c/0x254/0x258`) as plain Rust, `power_terms(&PowerInput, &mut dyn PowerEnv)`. It has no
emulated memory: the fields it reads are a typed struct and its surroundings (frame time, bindings, altitude,
temperature, density, noise) are a trait called in the original's order. The emulator adapter in `piston.rs`
calls it, so the recorded original vectors (`piston_engine_update_matches_the_original_machine_code`, 50+ cases
from `tools/gen_piston_vectors.py`) check it; a deliberate change of one coefficient makes that test fail.
Still on the emulator: `piston_levers` and the handlers `update_engine_kind12` (the C172's kind) and the
helpers. Next slices, in order of the C172's path: `piston_levers` (without kinds 3/4), `update_engine_kind12`,
then the propeller stage.

## Progress: the lever stage is native too
`piston_native::lever_terms` now holds the lever stage of `piston_levers` (the level `M+0x22c`, the commanded
value with the throttle ceiling, the ramp limit and the kind 3 and 4 limits against the bound elements) over
the typed `LeverInput`; its environment is two calls (frame time for kinds 3 and 4, then the ramp query).
The remaining parts of `piston_levers` (the per-kind handler dispatch and `power_from_tables`) and all four
handlers are still on the emulator.

Coverage of the recorded vectors, measured by changing one constant at a time in `piston_native.rs`: the
tests caught a changed `dt += dt` (the doubled frame time) and the `0.15` lever reduction of the power stage,
but not a changed `0.95` (kind 3 mode 2 limit), a changed `0.13` (kinds 1 and 2 scale) or a one-ulp change of the
kind 4 seed (the comparison tolerance is 1e-5 relative). So these three paths are carried over from the
emulator code unchanged but are not independently pinned by the vectors; they need more vectors with the
ceiling and the bound elements away from zero before they can be called verified.

## Progress: the head of the kind 1/2 handler is native
`piston_native::kind12_head` holds the part of `update_engine_kind12` before the running-engine branch: the
limited power input `a10`, the speed over the altitude-dependent rated speed `d15`, the lean/rich response
(`M+0x210`, `M+0x214`) and the kind 1 throttle-closing transient. Its environment is the flight altitude and the
frame time (twice). Unlike the lever stage, the recorded vectors pin it: changing any one of the constants
`0.3` (rate and transient), `0.8/0.2` (lever blend), `10000` (altitude span) and the decay divisor made the
test fail. Remaining on the emulator: `kind12_running` and `kind12_finish` (the manifold terms, the table
power, the fuel flow and the speeds), then the dispatch, `power_from_tables` and the kinds 0, 3 and 4 handlers.

## Progress: the running-engine part of the kind 1/2 handler is native
`piston_native::kind12_running_terms` holds `kind12_running`: the five temperature breakpoints `A1..A5`, the
throttle `M+0x40` ramp, the manifold terms `M+0x240/0x244/0x248` with the magneto cuts, the table power
`M+0xcc` and the fuel factor it returns. Its environment is the bindings, the flight altitude, the atmosphere
table, the clock and the wave noise.

Pinning by the recorded vectors, measured by changing one constant at a time: caught are the `-0.9848` and
`0.243` terms, the lever blend `0.45/0.55`, `-0.14`, the noise weights `0.05/0.95`, the magneto factors
`0.95` and `1.04`, the `a4 - a3` slope, the doubling `v0 + v0` and the throttle ramp's `0.25`... except
that these changes were NOT caught: the `0.633` slope (below `A2`), the `0.1235` and `1.3826` terms of `q4`,
both `0.95` thresholds (kinds 1 and 2), the `3.2` slope (below `A5`) and the `level - 0.25` ramp offset.
Those branches are carried over from the emulator code unchanged but the vectors do not pin them; they need
vectors where the state sits below the breakpoints and near the threshold. The remaining emulator parts are
`kind12_finish`, the dispatch, `power_from_tables` and the kinds 0, 3 and 4 handlers.

## Progress: the common end of the kind 1/2 handler is native
`piston_native::kind12_finish_terms` holds `kind12_finish` (power write with the lever term, the load and
friction terms, `M+0xb8`, the speed percent, `M+0x248` scaling and the lagged fuel-air state `M+0xa0`); its
environment is the engine lever, the thrust term and the frame time. The recorded vectors pass and the whole
workspace is green; I did not run the one-constant mutation check on this stage, so its pinning is unmeasured.
With it the whole kind 1/2 handler (head, running part, end) is native; the dispatch, `power_from_tables` and
the kinds 0, 3 and 4 handlers are still on the emulator, and the flight loop does not call any of this yet.

Mutation check of the end stage (one constant changed at a time, 10 changes): the vectors caught 8. Not caught: the `0.6` factor of a negative `x6f` and the `29.92` inHg divisor of the manifold limit (the branch `B+0x938 < 1` is rarely reached by the random vectors). Note that the stock C172 has `crit_alt_mtr = 0`, so that branch is the one it takes: add vectors for it before relying on this stage.

Biased vectors (`piston_stock.txt`, 80 attempts, kinds 1/2 with `B+0x938` below 1, `tools/gen_piston_vectors.py ... stock`, needs `llvm-objdump` or a copy of GNU `objdump.exe` named that on PATH) pass against the native code (test `piston_engine_update_matches_the_original_on_the_stock_path`). They did NOT catch any of the seven previously uncaught mutations (`0.6`, `29.92`, `0.633`, `0.1235`, `0.95`, `3.2`, `level - 0.25`), so those branches are still unpinned: the random state evidently does not reach them (or a later clamp hides them). Pinning them needs targeted presets of `M+0x214`, `M+0x40` and the lever, not more random cases.

Second pass: the stock vectors now preset `M+0x214`, `M+0x40`, the lever, `M+0x78` and `M+0x48` (150 cases). The `0.633` slope below `A2` is now caught; still not caught: `0.6`, `29.92`, `0.1235`, the `0.95` thresholds, `3.2` and `level - 0.25`. Those may be unreachable with these presets or may only affect words the recorded dump does not include; that is not yet established.

## Wiring plan for the flight loop (2026-10-30, after reading the code again)
Findings that decide the route:
- The propeller function `prop::prop_force` (1513 lines, checked at frame checkpoints against the original) works on the
  original's flat objects (`F`, `B`, engine record `E`, part record `P` at `B[0x6010] + n*0x3770`, output record `R`), not on
  typed state, and takes its airflow from `airflow()` on `F`. The piston stages in `piston_native.rs` take typed inputs.
  So there are two different seams: the engine side (typed, native) and the propeller side (object-based).
- `flight_init` (`flight_init.rs`) builds the `F`/`B` objects from an aircraft in the emulator's memory; it is the only
  producer of the `P`/`R` records the propeller function reads.

Route that keeps 1:1 checks (not started):
1. Run `flight_init` for the C172 in a `Vm` to obtain `F`, `B`, `E`, `P` (the loader exists; its inputs are not yet fed
   from the installed ACF end to end, check `flight_init` callers in `crates/xp-app`).
2. Per frame: set the controls (`F+0xb7a8 + 4n` power input, mixture, etc.), the airspeed and density in the `F`
   flow fields, call `update_engine_piston` (`piston.rs`), then `prop_force`; read thrust and torque from `R`
   (`R+0x14..0x88`, see `prop.rs` segment 10) and replace `T = min(cap, eta P/(V+38))` and the missing torque and
   P-factor in `FlightModel::forces`.
3. Compare against the four hands-off recordings: roll times (0.2 s reproducibility), liftoff, bank growth.
The first concrete task is step 1: a test that loads the C172 through `flight_init` and prints the `E` and `P` records,
so the emulator-memory objects are proven to exist for the stock aircraft before anything is wired.

## Step 1 done: the engine record of the C172 from the ACF loader (units confirmed)
`xp_acf::object::load_object` now stores the engine kind word `E+0` from `_engn/n/_type` (`ELE` 0, `RCP_CRB` 1,
`RCP_INJ` 2, `TRB_FRE` 3, `TRB_FIX` 4; it was left at 0 before, i.e. an electric engine). For the stock C172 SP the
record `E` (region `R5ff8`) reads, in the original's own units (test `engine_record_local`, needs the installed ACF):
`E+0` = 2, `E+4` = 132734.6 (**178 hp in watts**, so the unit guess above was right), `E+0x14` = 261.80 rad/s (2500 rpm),
`E+0x18` = 282.74 rad/s (2700 rpm), `E+0x64` = 1.0, and the engine part point `E+0x4c..0x54` = (0, -0.061, -2.018) m
relative to the CG (the thrust point the port now uses, in agreement with the earlier hand calculation). The
`F`/`B` objects and the part record `P` are the remaining pieces of step 1.

## Step 1b: the original's engine code runs on an object from the ACF (not yet producing power)
`crates/xp-app/tests/flight_local.rs` now has `vm_from_acf_object` (the words of `load_object` laid into a `Vm` with the
region pointers) and two tests on the stock C172:
- `original_engine_lever_runs_on_the_c172_object`: `engine_small::engine_lever` gives exactly `E+4 * B+0xb08`, i.e.
  132735 W times the throttle word, so the original's power command for kind 2 is the 178 hp rating times the lever.
- `original_piston_update_runs_on_the_c172_object`: `update_engine_piston` (power stage, lever stage, kind 1/2
  handler) runs 2000 steps of 5 ms against a sea-level environment (`LiveEnv`: frame time, bindings 0, 15 °C, 1.225
  kg/m³, noise 0). Result so far: `M+0x24c = M+0x258 = 0.75` (power/density term at full throttle), `M+0x22c = 1`,
  but `M+0xcc` (engine power) and `M+0x90` (speed) stay 0 and the return value is NaN: the engine is not running
  (no start state, starter and fuel flags not set) and one callee, `0x1408be0f0`, is not answered by `LiveEnv`.
Next: preset the running state the way `tools/gen_piston_vectors.py ... stock` does (`piston_stock.txt`), find what
`0x1408be0f0` returns, and then read `M+0xcc` at full throttle as the engine power the original feeds to the
propeller.

## Step 1c: with a running-engine preset the original update produces a power
Answering the remaining callees as the vector generator does (`0x1408be0f0` a float, here 0; `0x14067b2f0` 1.0;
`0x1410c9c60` a pointer; `0x14117c380` nothing) and presetting `M+0x70 = 1`, `M+0x74 = 1`, `M+0x78 = 2400` (rpm),
`M+0x214 = 1`, `M+0x40 = 1`, `M+0x48 = 3` (the state of `gen_piston_vectors.py stock`), the update on the C172 object
leaves `M+0xcc = 22518 W` and `M+0x90 = 916.7` (what unit this is is not known; the return value is `inf`) at full
throttle and zero lever response state. 22.5 kW is 17 % of the 132.7 kW rating, so the state is not the original's
steady full-power state: `M+0x210` (the lever response weight in `M+0xcc = engine_lever * M+0x210 + ...`), the
speed `M+0x78` (which the update does not advance by itself: the propeller side integrates it) and the manifold terms
(`M+0x24c = 0.75`) are still preset values. The values do not change over 2000 steps because nothing integrates the
engine speed. Getting the steady state requires the propeller force (`prop_force`) in the loop, which is the next piece.

## Step 1d: the propeller function runs to its end on the C172 records (nothing physical yet)
`original_prop_force_reaches_the_end_on_the_c172_records` (`flight_local.rs`) calls `prop::prop_force(.., Stop::End)` with
`B`, `E` (`R5ff8`) and `P` (`R6010`) from the ACF loader, an empty `F`, a still airstream (`LiveProp` environment: wind
0, no terrain, no events, element forces stubbed to zero) and returns `Ok`; the output record `R+0x14..0x88` is zeros
and NaN (as expected with a zero `F` and zero element forces: divisions by the empty frame values). So the function
accepts the real records. What a physical result still needs, in order:
1. the words of `F` it reads (matrices `F+0x430..0x454`, origin `F+0x378..0x388`, reference velocity `F+0x368..0x370`,
   rates `F+0x3cc..0x3d4`, density, `F+0xb7e8 + 4n` ice, ...): find them by running with `Stop::Segment1..10` against a
   NaN check, or from the recorded vectors' `F` regions (`prop_vectors`), and fill them from `FlightModel::state`;
2. the engine record array `M` (`F[0x68b0]`: the engine speed `M+0x78`, power `M+0xcc` from the piston update);
3. a real `element_force` for the blade elements: build `wing_element::ElementInputs` from the blade geometry in `P`
   (`_blad/*` rows of the schema live in `R6010`, like the wing code of `FlightModel` does for wings) with the prop
   airfoil (`_afl_file_*`), and answer `ElementCall` with its lift/drag/moment.

### Which words of `F` the recorded vectors set (46 offsets)
Collected from `prop_1..11.txt`: `28 6c 118 11c 120 170 1cc 1d0 2bc 2d0 2e4 2f8 310 328 350 358 368 36c 370 378 37c 380 384
388 38c 3cc 3d0 3d4 400 410 414 41c 430 434 440 444 450 454 651c 6520 653c 6540 b7e8 b7ec b7f0 42f54`. Filling the three
sine/cosine pairs (`430/434`, `440/444`, `450/454`: 0 and 1), `F+0x28 = 1` and a 30 m/s flow in `F+0x370` already changes the
output record (`R+0x50 = -2.0`, `R+0x58 = -55.87`) but the rest stays NaN until the engine record `M` and the blade
element forces exist; the meaning of `118/11c/120`, `170`, `1cc/1d0`, `350/358`, `400..41c`, `651c..6540` is not named yet.

## Step 1e: a real blade element function is wired into the live propeller call; the blade boundary arrays are missing
`LiveProp` in `flight_local.rs` now answers `PropEnv::element_force` with the verified `element_force::element_force` and
`profile::outer` (airfoils `NACA 16 (good propeller).afl`, `Clark-Y (good propeller).afl` from the install; their names
come from the ACF text because the loader models only strings of up to 15 characters inline, longer ones are on the
heap). The call then panics inside `wing_element::sweep_degrees`: the boundary arrays (x, y, z, chord per rib) the wing
code indexes are derived values; `FlightModel` computes them for wings ("original boundary arithmetic") but nothing builds
them for the blade records of `P`. The test catches the panic and prints it. Next piece: derive the blade boundary arrays
from `_blad/0/_Croot/_Ctip/_semilen_SEG/_els` and the rib tables the same way as for the wings, store them in `P`, then
rerun; after that the element forces feed `prop_force` and the `M`/`F` words are the remaining inputs.

## Step 1f: blade boundary arrays filled; the call now does not return
The test fills blade 0 of `P` like a straight wing: element count at `W+4`, ribs from `wing_boundary::boundary_point` into
`0x5bc/0x5e8/0x614` and the chord at `0x70` (10 elements, chord 0.102 m to 0.076 m, span 0.975 m). The index panic is gone, but
`prop_force` then runs for over 10 minutes without returning (killed), so some loop in the propeller or element code
spins on the still-incomplete `F`/`P`/`X` words (the rest of the wing fields the element function reads, such as the
control surface gates at `W+0x2fc..`, the flow words of `F`, or the state arrays `X`, are all zero). The call is now gated behind
`OPENXPLANE_PROP_LIVE` so the suite stays fast. Next: run with `Stop::Segment1..10` to find which segment spins, then fill
the words it waits on.

## Step 1g: the hang was a wrong base; the live call returns
In the original's part record the wing object `W` sits at `P+0x88` (`p.f32(0x88 + 0x5bc + 4i)` in `prop.rs`; the loader's `_els`
at `P+0x8c` is `W+4`, `_Croot` at `P+0x90` is `W+8`). The test had put the boundary arrays and chords at `W` offsets
inside `P`, so the element code read zeros and a wrap loop spun. With the arrays at `P+0x88+...` and the element function
given the record shifted by `0x88`, `prop_force` returns in under 2 s with the real element function and the propeller
airfoils (no element errors). The output record is still partly NaN/zero, so the `F` words (density `F+0x6c`, speed
reference `F+0x74`, temperature `F+0x5c`, the part-position words `F+0x350..0x38c`, rates `F+0x3cc..`) and the engine
record `M` are the next inputs; `OPENXPLANE_PROP_STOP=1..10` picks a checkpoint to look at.

## Step 1h: density, temperature, speed reference do not change the output
Setting `F+0x6c = 1.225`, `F+0x5c = 15`, `F+0x74 = 1`, `F+0x68 = 101325`, `F+0x408 = 1` leaves the output record as before
(`R+0x50 = -2`, `R+0x58 = -56.5`, thrust words zero). `prop.rs` reads from `M` only `M+0xb8` (the piston's torque-like output),
so the spin of the blades is not in `M`: the per-part runtime fields (the part record beyond its ACF words, e.g. `P+0x3730`,
and the element-state records `X`, which carry the previous pass's blade angles and speeds) are the missing inputs, and the loader
does not create them (they are runtime state of the flight object). Next: read what `flight_init` writes into `P`/`X` for engine
parts (the init code is ported, `flight_init.rs` PART/PART_AERO constants `0x388`/`0x3770`) and replicate it, then spin the part.

## Step 1i: the spin word was the missing input

The earlier note that the blades' spin lives in unmodelled runtime part state was wrong for the first-order
case: `R+0x1c` (the propeller's angular speed, rad/s) was simply zero in the live test. With 2400 rpm set there
(251.33 rad/s) the call still returns `ok`, but the output record has NaN in several words (`R+0x50`..) and the
thrust words are still zero. Checkpoints `OPENXPLANE_PROP_STOP=4..8` show the record unchanged, so the NaNs are
produced in the post-loop section (segments 9 and 10). Next: find which post-loop input is NaN
(candidates: `M+0xb8`, the zeroed `Y` record, the `0x3730` word of `P`).

## Step 1j: the NaN came from `P+0x4c`

`P+0x4c` is not an ACF word (not in `object_schema.tsv`). `prop_force` divides by it in the lag filter of
`R+0x84` (`engine::lag_filter`, `b950` argument) and in the induced-velocity reference (`2 rho P+0x4c`), so a zero
there gave 0/0 and the NaN words. The loader (or a later init step) must compute it. Setting it to a disk area
guess (pi * blade length^2) removes every NaN. The thrust and torque words are still zero: frame words
`0xe0..0xe8` (the element sums) stay 0 although `0x10c` and `0x118` are large, so the element outputs do not reach
the sums yet. Next: trace `out1..3` of `element_force` into `fr 0xe0..0xe8` and check the guess against the
original (find the writer of `P+0x4c`; the REA session stalled this turn).

## Step 1k: the element function saw a stale X record

`prop_force` writes the element state (`X+0x54` speed, `X+0x2c` angle, `X+0x194`) into its own `xs`, while the
live test's `element_force` held a clone taken before the call, so the element function saw zero speed (zero
force). `ElementCall` now carries `x: &Words`, the caller's current record, and the test uses it (the verified
vector tests ignore the field). With that, and `X+0x288 = 1`, the element lift is non-zero and `R+0x60` is
about 1.5 (not yet physical: a C172 prop should give ~1500 N at 2400 rpm and 30 m/s).
Still open: the element drag (`out2`) is NaN for every element, independent of the guessed aspect ratio at
`W+0x14` (like `P+0x4c`, a derived word the loader does not set), so some other derived input of the drag path
(`W+0x18`, `W+0x1c`, foil terms, `X+0xa4..`) is zero or NaN. The lift is also tiny for elements 0, 1 and 3..9
(1e-6) and 0.82 for element 2, which points to a per-element weight array at `W` that is not filled.

## Step 1l: finite thrust from the live call

The NaN drag came from `X+4+4e` (the per-element speed factor `v`, divisor in the induced-drag term) and
`X+0x28c` (its scale) both being zero: inf times 0. With both set to 1 (guesses; the port does not model their
writer yet) the elements return finite lift (200-400 N per outer element) and drag, and the output record gives
`R+0x60` (thrust) = 2927 N, `R+0x14` = 1.2e5, torque words small. A C172 at 2400 rpm and 30 m/s should give about
1500 N, so the guessed words or the stubbed noise table are still off by a factor of ~2. Elements 1-4 fail with
"noise coordinate outside supported domain" (the test's noise table and coordinates are stand-ins), so only
6 of 10 elements contribute. Open: the real writers of `P+0x4c`, `W+0x14`, `X+4+4e`, `X+0x28c`, `X+0x288`; the real noise table.

## Step 1m: the derived words, read from the live original (2026-10-31)

Attached Frida to the running reference (12.4.3, C172 loaded, cold and dark) and called the getter `0x141a64250`
(`rcx = 0x1461243b0`, index 0) through the rebased module address (the exe is ASLR-relocated; the static
addresses do not work as raw pointers). Values of the single part (`B+0x920 = 1`, `B+0x91c = 1`, kind 0):

| word | live value | note |
| --- | --- | --- |
| `P+0x4c` | 2.98868 | = pi * 0.97536^2: the disk-area guess was right |
| `P+0x3730` | 0.97536 | = blade length (`P+0x88+0x10`) |
| `W+0x14` | 0.875130 | not a plain aspect ratio (my guess 10.97 was wrong) |
| `W+0x18` | 21.942861 | also `X+4+4e` for all 10 elements and 4 stations |
| `W+0x1c` | 0.861344 | |
| `W+0x30`, `W+0x38` | 0 | |
| `X+0x288`, `X+0x28c` | 1, 1 | |
| `B+0x64f4/8/c` | 5.568264, 1.785518, 6.035962 | divisors of the element's noise coordinates |
| `B+0x1f78/7c/80` | 0.858891, 0.1, -0.265349 | already equal to the loader's values |

With these (and the guessed `X+4+4e = 1` replaced) all ten elements now return finite forces (the earlier
"noise coordinate outside supported domain" errors came from the zero `B+0x64f4..fc`), and the live call gives
`R+0x60` = 2784 N at 2400 rpm and 30 m/s, still above the ~1500 N expected. The stalled inner elements (negative
lift, alpha 19 deg) suggest the test airflow (`F+0x370 = -30`, identity attitude, zero ground effect) is not the
real inflow; the next check is reading the live `M`/`R` and the part record while the engine runs.
Writers of the derived words (the code that computes `W+0x14`, `W+0x18`, `W+0x1c`, `B+0x64f4`) are still unported.

## Step 1n: the live idle snapshot as a comparison case (2026-10-31)

Engine started in the live reference (idle). The propeller output record is the part record at `F+0x68c8`
(`R` = `Y`, 0x388 bytes; not the engine record at `F+0x68b0`): `R+0x1c` = 53.4539 rad/s (510 rpm), `R+0x54`
= -89.9988, `R+0x58` = 16.5699, `R+0x60` = 103.1 (thrust, N), `R+0x64` = 17.7278, `R+0x84` = 7.5647 (induced
velocity), `R+0x1e4..` = per-element induced state. The snapshot is in
`crates/xp-app/tests/data/prop_live_idle_record.txt` (runtime values, not an original resource).
Other derived words read live: `W+0x16c..` per-element pitch (58, 58, 47.35, 37.93, 31.54, 27.03, 23.69, 21.14,
19.13, 17.50, 0 deg) and the annulus areas at `P+0x58..0x7c`; the boundary arrays and chords the test builds
already equal the live ones.

`OPENXPLANE_PROP_LIVE=1 OPENXPLANE_PROP_OMEGA=53.45387649536133 OPENXPLANE_PROP_AIRSPEED=0` starts the port's
`prop_force` from that record. Result against live (one frame apart, so the accumulators differ a little):
`R+0x54` -90 (live -89.9988), `R+0x58` 16.5575 (16.5699), `R+0x84` 7.594 (7.565), but `R+0x60` thrust 36.5 N
(live 103.1) and `R+0x64` 12.7 (17.73). So the angles and the induced velocity now agree and the force sums are
low by a factor of about 3: the remaining difference is in the element forces (inputs still not read from the
live process: the other `F` words such as `F+0x120`, the element-state record `X` of the live frame, and the real
noise table).

## Step 1o: the noise table is MT19937; thrust within 5% of the live idle (2026-10-31)

The 262144-entry noise table (the one the buffet and the stalled elements read) sits at `0x14578f1f0` (from the
`lea r9, [rip+0x45ad9b0]` at `0x1411e1839`); dumped from the live process it equals, for all entries, the output
of MT19937 seeded with `init_genrand(1)` divided by 2^32 as float32. `NoiseTable::original()` now generates it
(unit test against the first four and the last entry). The stand-in table of the live test was the reason the
stalled inner elements had negative lift.

State loaded from the live idle snapshot (`prop_live_idle_f_x.txt`: the `F` words the call reads, among them the speed of
sound `F+0x74` = 340.26 and the non-zero yaw pair `F+0x440/444`; the element record `X` of station 0) with
`OPENXPLANE_PROP_LIVE=1 OPENXPLANE_PROP_OMEGA=53.45387649536133 OPENXPLANE_PROP_AIRSPEED=0`:

| word | port | live |
| --- | --- | --- |
| `R+0x60` thrust | 107.99 N | 103.10 N |
| `R+0x58` | 16.5567 | 16.5699 |
| `R+0x54` | -89.9993 | -89.9988 |
| `R+0x64` | 16.65 | 17.73 |
| element Cl (`X+0xf4`), outer | 1.294, 1.249 | 1.295, 1.249 |
| element Cl, inner | 1.01, 1.05, 1.05, 0.93, 0.87, 1.04, 1.27, 1.36 | 1.08, 1.10, 1.09, 0.96, 0.88, 1.03, 1.26, 1.35 |
| element alpha (`X+0x2c`) | within 0.2 deg except element 0 (41.6 vs 46.9) | |
| element speed (`X+0x54`) | within 0.2% | |

The test asserts thrust within 10% and `R+0x58` within 0.05 for this state. The live snapshot was taken a frame or
more before the port's call, so the remaining differences are of the order of the frame-to-frame change.
Still open: the engine/prop state of the port is not wired into `FlightModel::forces`; the record fields the
original's flight loop feeds (`R+0x1c` from the engine, `F+0x370` airflow, ...) have to come from the port's
state, and a loaded-propeller case (not idle) still has to be compared.
