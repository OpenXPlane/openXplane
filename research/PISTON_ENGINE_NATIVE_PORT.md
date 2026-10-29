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
