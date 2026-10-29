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
