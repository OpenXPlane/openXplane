# Normalised angle and airfoil stall state

Date: 2026-10-06. The reference EXE and SHA256 are in [BASELINE.md](BASELINE.md). Source: static analysis of
`0x141a417c5 .. 0x141a41907` in function `0x141a412c0`. It has not yet been checked against the original
running.

## Confirmed behaviour

After the Cl/Cd/Cm lookup the original computes a separate value from the angle that has already gone through
wrap and the limit to the range -180..179. The positive bound is taken at offset `+0x10` (AFL parameter 4), the
negative one at `+0x0c` (AFL parameter 3). With different bounds the positive and negative sides are normalised
independently.

Order of float32 scalar operations:

```text
when alpha > 0:
    limit = parameter[4]
    normalized = limit == 0 ? 0.5 : (1 / (limit - 0)) * (alpha - 0) + 0
when alpha <= 0:
    limit = parameter[3]
    normalized = limit == 0 ? -0.5 : 0 - (1 / (limit - 0)) * (alpha - 0)
```

At a zero angle both branches of the original may write the value; the negative one runs last. So with zero
bounds the result is -0.5. Zero bounds have confirmed special behaviour here and are not replaced with an
arbitrary typical angle.

The flag at stack `RSP+0x100` turns state retention on. The pointer at `RSP+0x130` points to the output/previous
flag:

```text
if state memory is off: stalled = abs(normalized) > 1
else:
    if abs(normalized) > 1: stalled = true
    if abs(normalized) < 0.75: stalled = false
    between the thresholds the previous state is kept
```

The comparisons are strict: exactly 1 does not switch the state on, exactly 0.75 does not reset it. The bits of
the constants were read from the EXE:

| VA | Bits | Float32 |
|---|---|---:|
| `0x14250e248` | `0x3f000000` | 0.5 |
| `0x14250e6dc` | `0xbf000000` | -0.5 |
| `0x14250e274` | `0x3f400000` | 0.75 |

## Implementation

`src/stall.rs` returns the normalised angle and the state flag. The previous state is passed in explicitly, which
lets it be stored for a specific evaluated profile. When two tables are blended the original uses one shared
flag - see [PROFILE_PIPELINE.md](PROFILE_PIPELINE.md). The flight loop and the placement of these states along
the wing are not implemented yet. The `airfoil-eval` CLI shows the variant without memory; the mode with memory
is checked by sequences in unit tests.

This is the numeric part of the stall mechanism. By itself it adds no forces, does not reduce Cl, does not
increase Cd and does not model aircraft buffeting.

## The next branch of the original

When the flag is active, the instructions `0x141a4190f .. 0x141a419db` call `0x141a42a10` twice with different
inputs, then modify Cl and Cd. One of the sources is a global double at `0x142f01918`; it is tied to
`sim/time/total_running_time_sec`. The update order still needs analysis of the writing code. A value in the
file's memory image must not be taken for a constant of the runtime. The helper function itself calls
`0x1411e17a0` several times. This branch is implemented with explicitly passed inputs and table.

Update from the next stage: the generator and corrections were moved into a separate module with explicitly
passed inputs. Initialisation of the shared table remains unknown; the time source is established in
[RUNTIME_TIME.md](RUNTIME_TIME.md). Details are in [BUFFET.md](BUFFET.md).

## Checks

20 unit tests pass; Clippy without warnings; offline build. The new tests check asymmetric bounds, the negative
side, the sequence of entering and leaving, the exact thresholds 1 and 0.75, disabled state memory, zero bounds
and NaN.

All 34 provided airfoils passed the angles -30, 0, 10 and 30 degrees with multiplier=divisor=1 - 136 diagnostic
runs without errors. For NACA 2412 at +16° the normalised angle is `1.066666722` and the flag is active. The
printed coefficients still belong to the table section, without the original's later perturbations.

Local artifacts in `research/local/` are excluded from Git: `airfoil-3d-corrections.txt`,
`airfoil-stall-constants.json`, `airfoil-stall-audit.txt`.
