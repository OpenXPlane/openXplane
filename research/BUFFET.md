# Coefficient perturbations during active stall

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). The implementation is based on
static analysis, without running the original.

## The basis function

The range `0x1411e17a0 .. 0x1411e1935` was disassembled. Inputs: three float32 values and an integer argument.
The function takes the absolute values of the coordinates, separates integer parts and fractional remainders and
computes an index:

```text
D = (((integer_z << 6) + integer_y) << 6) + 100 * seed + integer_x
table_index = (D + corner_offset) & 0x3ffff
```

The index operations wrap as uint32. Eight offsets are used: `0, 1, 64, 65, 4096, 4097, 4160, 4161`. The
fractional remainders give the interpolation weights; there is no extra smoothing of the weights in this
section. The order of operations follows the instructions: first the upper z layer, then the lower, then their
sum. After the float32 interpolation the result is converted to double, doubled, 1 is subtracted, and it is
converted back to float32.

The reference to the shared table at `0x14578f1f0` is at `0x1411e1839`. The mask shows 262144 addressable
elements. This address lies in the part of `.data` not backed by file bytes: the table is filled while the
program runs. The zero content of the initial memory is not taken for its working values. The fill code and the
initial conditions are not recovered.

## Sum of scales

The function `0x141a42a10 .. 0x141a42c0e` calls the basis function six times. The coordinates are computed
through double with conversion to float32; the sum of the results is accumulated in double and the total is
returned as float32.

| Order | Coordinate scale | Weight |
|---|---:|---:|
| 1 | 2 | 0.5 |
| 2 | 1 | 1 |
| 3 | 4 | 0.25 |
| 4 | 8 | 0.125 |
| 5 | 16 | 0.0625 |
| 6 | 32 | 0.03125 |

The constants were read from the EXE at `0x14250e330`, `0x14250e390`, `0x14253f790`, `0x14253f860`,
`0x14250e400`, `0x142543b08`, `0x14250e440`, `0x14253f760`, `0x14250e308`. The sum of the weights is 1.96875;
the function does not normalise the sum to one.

## Active-stall corrections

The branch `0x141a4190f .. 0x141a419db` modifies Cl and Cd. Three float32 inputs are read from the stack at
`RSP+0xe8`, `+0xf0`, `+0xf8`. Their origin is not established yet; in the API they are x, y and phase, with no
claim about their physical meaning. The global double at `0x142f01918` is passed in explicitly as time in
seconds. It is tied to the dataref `sim/time/total_running_time_sec`; the evidence and the precision of access
are described in [RUNTIME_TIME.md](RUNTIME_TIME.md). The order in which time advances is unknown for now.

```text
lift_phase = float32(running_time_seconds * 4 + double(phase))
lift_noise = fractal(x, y, lift_phase, seed=0)
Cl_out = float32((double(lift_noise) * 0.25 + 1) * double(Cl_in))

drag_phase = float32(running_time_seconds * 5 + double(phase))
drag_noise = fractal(x, y, drag_phase, seed=1)
Cd_out = float32((double(drag_noise) * 0.25 + 1.5) * double(Cd_in))
Cm_out = Cm_in
```

The coefficients 4/5/0.25/1/1.5 are confirmed by reading the constants and the instructions. Cm is not
overwritten in this branch. Whether the branch applies at all is decided by the flag recovered in
[STALL_STATE.md](STALL_STATE.md).

## Implementation and limits

`src/buffet.rs` contains `NoiseTable::basis`, `fractal`, `perturb`. The table is supplied explicitly. The engine
has no invented table fill or hidden choice of initial state. The module is not wired into the ordinary lookup
automatically until the source of the shared table and of the other inputs is recovered.

The CLI `buffet-eval <table.f32le> <x> <y> <phase> <running-time-seconds> <cl> <cd> <cm>` reads exactly 1048576
bytes - 262144 little-endian float32 values - and computes only the active-stall branch. The length and the
finiteness of the numbers are checked. The supported range of the basis function's coordinate magnitudes is
below 2^32; edge cases of the original's conversion of large coordinates are not reproduced. This is
openXplane's diagnostic contract.

## Checks

22 unit tests pass, Clippy without warnings, offline build. The new tests use our own synthetic tables:
individual nodes, centre interpolation, seed offset, reflection of negative coordinates, the index mask, the sum
of scales, the Cl/Cd multipliers and the preservation of Cm.

The CLI was checked on three constant synthetic tables. With all elements 1, zero coordinates and runtime=0 the
noise is 1.96875; Cl=2 becomes 2.984375, Cd=0.25 becomes 0.498046875. This is a check of the formulas, not a
comparison with the working table of the original X-Plane.

Local artifacts are excluded from Git: `research/local/noise-basis-candidate.txt`, `airfoil-angle-lookup.txt`
(the historical name of the disassembly file of the sum of scales), `airfoil-3d-corrections.txt`,
`buffet-diagnostic-check.txt`.
