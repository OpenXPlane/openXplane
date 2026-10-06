# Engine record layout and the engine code of the reference build

## Established from the code

- The engines are an array of records of `0x2cc` bytes, its pointer at `+0x68b0` of the aircraft object
  (the one at `+0x20` of the flight object, "B" in [WING_ELEMENT.md](WING_ELEMENT.md)). At most 16 engines
  are exposed through the array datarefs.
- Byte offsets inside a record, read from the array getters of the datarefs (`tools/extract_engine_layout.py`):

| Offset | Type | Dataref |
|---|---|---|
| 0x000 | float[] | `sim/flightmodel/engine/ENGN_thro` |
| 0x004 | float[] | `sim/flightmodel/engine/ENGN_thro_use` |
| 0x040 | float[] | `sim/flightmodel/engine/ENGN_mixt` |
| 0x044 | float[] | `sim/flightmodel/engine/ENGN_cowl` |
| 0x048 | int[] | `sim/cockpit/engine/ignition_on` |
| 0x050 | float[] | `sim/flightmodel/engine/ENGN_heat` |
| 0x074 | int[] | `sim/flightmodel/engine/ENGN_running` |
| 0x078 | float[] | `sim/flightmodel/engine/ENGN_tacrad` |
| 0x090 | float[] | `sim/flightmodel/engine/ENGN_N1_` |
| 0x098 | float[] | `sim/flightmodel/engine/ENGN_N2_` |
| 0x0a0 | float[] | `sim/flightmodel/engine/ENGN_MPR` |
| 0x0b0 | float[] | `sim/flightmodel/engine/ENGN_EPR` |
| 0x0b8 | float[] | `sim/flightmodel/engine/ENGN_TRQ` |
| 0x0b8 | float[] | `sim/flightmodel/engine/ENGN_driv_TRQ` |
| 0x0cc | float[] | `sim/flightmodel/engine/ENGN_FF_` |
| 0x1bc | float[] | `sim/flightmodel/engine/ENGN_fuel_press_psi` |
| 0x1c4 | float[] | `sim/flightmodel/engine/ENGN_oil_press_psi` |
| 0x1cc | float[] | `sim/flightmodel/engine/ENGN_oil_temp_c` |
| 0x1e4 | int[] | `sim/cockpit/engine/fadec_on` |
| 0x1e8 | int[] | `sim/flightmodel/engine/ENGN_fadec_pow_req` |
| 0x1f4 | int[] | `sim/cockpit/engine/igniters_on` |
| 0x208 | int[] | `sim/cockpit/engine/fuel_pump_on` |
| 0x230 | int[] | `sim/flightmodel/engine/ENGN_burning` |
| 0x234 | float[] | `sim/flightmodel/engine/ENGN_burnrat` |
| 0x238 | int[] | `sim/flightmodel/engine/burner_enabled_per_engine` |
| 0x258 | float[] | `sim/flightmodel/engine/ENGN_sigma` |
| 0x25c | float[] | `sim/flightmodel/engine/ENGN_power` |
| 0x260 | float[] | `sim/flightmodel/engine/ENGN_assumed_temp` |
| 0x298 | int[] | `sim/flightmodel/engine/ENGN_propmode` |
| 0x2b4 | float[] | `sim/flightmodel/engine/ENGN_crbice` |
| 0x2b8 | float[] | `sim/flightmodel/engine/ENGN_oil_quan` |
| 0x2bc | float[] | `sim/flightmodel/engine/ENGN_oil_lube_rat` |

- `0x141197b00` (10.8 KB, `0x2a70` bytes) is the engine update: it stores `ENGN_power` at `+0x25c`, uses `pow`,
  the interpolation helper `0x1406ea0b0`, the engine singleton getters `0x141a64250`/`0x141a64260` and many
  helpers (`0x1411e39a0`, `0x1411a2bf0`, `0x1411a2d90`, `0x1411dd610`, `0x141170810`, `0x1408625a0`).
- `0x141260090` (10 KB) reads 49 input-binding queries (`0x1407ace10`, throttle, mixture, magneto, starter
  commands): the pilot input side of the engines, not the physics.
- `0x1411a0380` is a switch of engine flag setters (ignition, fuel pump, FADEC...), 26 cases.

## Not established

What the update computes step by step (power, propeller torque and thrust, manifold pressure, fuel flow,
temperatures), and the meaning of the unnamed fields. Nothing of the engine is ported yet; the viewer's engine
is the approximate model of `crates/xp-sim/src/flight.rs`.

## Attempt to run the engine update in the emulator

Running `0x141197b00` on random objects fails early: the engine update asks the atmosphere (`0x141ba64e0`,
`0x141ba6750`, `0x141baf810`, object at `F+0xbfa8`), which interpolates a runtime-filled table of 0x801 entries
at `0x14612bd90` (the image holds zeros; index `(altitude_m + 5000) / 100`, pairs of floats per entry, pressure
scaled by 101325). Where that table is built has not been found yet. Plan: build the harness the way the wing
element one was built, with the unported callees replaced by stubs that record their arguments, and port the
atmosphere table first because every flight function depends on it.

Update: `tools/engine_harness.py` runs the update to its end in the emulator (random floats in the objects, the
counts `B+0x91c`/`+0x920` set to 1, diagnostics off, four stubs). The function is about 1400 instructions
after cleaning with `tools/clean_asm.py` and calls about 25 helpers, among them the input-binding query
`0x1407ace10` (ids `0x1d1`, `0x2fb`, `0x239`...), the interpolation `0x1406ea0b0`, a signed power
`0x1408625a0`, and several small engine helpers (`0x1411a2bf0`, `0x1411a2d90`, `0x1411e39a0`, `0x1411dd610`,
`0x141170810`). Nothing is ported yet.

## Ported engine functions

`crates/xp-airfoil/src/engine.rs`, verified by `tools/gen_engine_vectors.py` (1300 emulator cases in
`crates/xp-app/tests/data/engine.txt`):

- `signed_pow` (`0x1408625a0`): `x^p` keeping the sign of `x`, 0 for zero. 300 cases identical.
- `curve` (`0x14082b800` and `0x1411a2bf0`, two copies of one function): `v0 + (v1 - v0) * t^p` for `t` the
  position of `x` between `a0` and `a1` limited to 0..1. 389 of 400 identical, the rest within 35 ulp
  (cancellation after a libm `pow` differing in the last bit).
- `ram_power_factor` (`0x1411e39a0`): gas dynamics of the intake. The ram rise `(1 + 0.2 M^2)^3.5` scaled by
  the efficiency `B+0x980`, with the normal shock total pressure recovery
  `(2.4 r^2 / (0.4 r^2 + 2))^3.5 * (2.4 / (2.8 r^2 - 0.4))^2.5` blended in above the critical Mach number
  `B+0x984`; the slipstream speed by a five step square root iteration (`g = (g + 2 D / (rho g)) / 2` from
  170.145, with `D` from the engine descriptor, `rho = 1.225 B+0x950`); the result times the density ratio.
  598 of 600 identical, the rest within 2 ulp.

The small engine helpers `0x1411dd610` (throttle gain between `B+0x9d8` and `B+0x9dc`), `0x1411a2d90` (engine
starter conditions from two input-binding queries) and `0x141170810` (a response curve with the exponent
`B+0x9ac`) are read but not yet ported.

## Engine update, first part ported

`engine::engine_update` (`crates/xp-airfoil/src/engine.rs`) ports the first part of `0x141197b00`: the intake
power factor stored at `+0x258`, the spool-up timer `+0x22c` (advanced by the frame time, doubled when `+0x228`
is set, reset when the aircraft limit `B+0x9bc` exceeds `+0x98`), the throttle response (a power law with the
exponent from `B+0x9b0/0x9b4`, optionally limited by two starter conditions and the speed limit), and the
results at `+0x240`, `+0x244` and `+0x248`. 120 emulator cases (`tools/gen_engine_update_vectors.py`; the
atmosphere accessors, the engine flag, the frame time and the input-binding query are replayed from the
recording) are identical for these five fields, NaN results included. The other written fields (`+0x78`, `+0x90`,
`+0x98`, `+0xb0`, `+0xc4`, `+0xcc`, `+0x1d8`, `+0x1dc`, `+0x21c`, `+0x25c`, `+0x270..+0x27c`, `+0x28c`, `+0x2c4`,
`+0x2c8`) belong to the rest of the function, which is not ported.
