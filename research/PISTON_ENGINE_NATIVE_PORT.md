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
