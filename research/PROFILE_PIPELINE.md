# Unified profile evaluator and input replay

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). The sources are static analysis of
the original, not a comparison of running engines.

`src/profile.rs` joins the recovered stages of the function `0x141a412c0 .. 0x141a41a2a`: angle correction,
fixed AFL lookup, angle normalisation, the stall flag and the perturbations of the active branch. The numeric
tail of the function after the perturbations ends with diagnostic output and register restoration. The
original's logging is not reproduced.

Then the chosen tables are evaluated separately and Cl/Cd/Cm and the normalised angle are blended. This is the
recovered section of the calling function, not its whole output and not a wing force. The multiplier, divisor,
regime parameter and noise coordinates still come from outside.

## Shared stall memory

For two different tables the original, before the first call `0x141a45737`, puts the pointer from `RBP+0x3c8`
into the outgoing `RSP+0x50` (`0x141a456d7 .. 0x141a456de`). Before the second call `0x141a457a9` the same RDI
is saved to the same outgoing place (`0x141a4576e`). In the evaluator, five pushes and an allocation of 0xb0
give a stack offset of 0xd8; accounting for the return address, the outgoing `RSP+0x50` corresponds to its
`RSP+0x130` - the pointer to the stall flag.

So the first table updates the state, the second receives the updated value, and the flag after the second is
what stays outside. Independent flags for the two tables would be a different algorithm. The fourth
interpolation `0x141a45882` blends the normalised angle; the boolean flag is not interpolated. The
memory/diagnostic flags and the noise inputs for both tables are preserved.

`profile::evaluate` takes the previous state explicitly and returns the new one. An error does not change the
caller's state. Without a noise table a pass is possible only if the evaluator did not enter the active stall
branch; with an active branch an error is raised instead of silently skipping the corrections.

## Diagnostic replay

```sh
cargo run --offline -- airfoil-replay \
  "Xplane12/Airfoils/NACA 2412 (popular).afl" \
  examples/profile-replay.csv research/local/runtime-snapshot/noise.f32le 0
```

The last argument is the initial stall flag, 0 or 1. Instead of the table path, `-` can be given if all inputs
pass without an active stall. The input is a strict numeric CSV with a header:

```text
alpha,multiplier,divisor,regime,time,x,y,phase,retain
```

The angle is in degrees; the time is an explicit internal double in seconds; retain is 0 or 1. No other units
are assumed. Each row sets all inputs. There is no automatic time advance. The result flag is carried into the
next row. On an error the whole run is rejected before the result CSV is printed. The output contains the
indices of both tables, the effective angles, the normalised angle, the state and the coefficients.

The example with the stock NACA 2412 shows entering stall at 16°, holding at 12° and reset at 10°. The initial
multiplier/divisor and regime in the example are set for diagnostics, not obtained from the Cessna flight model.

## Getting a working table on Windows

`tools/capture_xplane_runtime.py` only reads process memory. It needs 64-bit Python and the provided
X-Plane.exe running. The PID can be seen in Task Manager, on the Details tab. After a flight loads, run from the
directory with the script:

```powershell
py -3 capture_xplane_runtime.py --process-id 1234 --output runtime-snapshot
```

1234 is replaced by the PID. The output directory must be new. The script checks the SHA256 of the EXE, gets
the actual module base accounting for ASLR, reads the table at RVA `0x578f1f0` twice, and the time double at
`0x2f01918` before and after the reads. It saves `noise.f32le` and `snapshot.json`. Non-finite data and changes
to the table between reads are rejected. A constant table is flagged separately: stability alone does not prove
that initialisation has finished. The process is not suspended; the snapshot of the time and the table is not
atomic.

The table file can be checked on any OS:

```sh
python3 tools/capture_xplane_runtime.py --inspect-table runtime-snapshot/noise.f32le
```

The Windows branch of the tool has not been verified on a running original yet. It does not solve the task of an
independent table generator, but it lets real working values be passed to the diagnostic evaluator. Snapshots of
the original process are kept in `research/local/`, outside Git.

## What this completes and what remains

The numeric stages of the named profile function are now connected with explicit inputs. Synthetic tests check
the shared memory of two tables, the difference between regimes with and without memory, carrying and resetting
between rows, a single table, the requirement of noise and the precision of time.

A standalone aircraft simulation is not ready yet: the sources of the corrections and the regime, the generator
of the shared table, the clock advance, the flow by sections, the integration of forces and moments, the engine,
the landing gear and the controls are not recovered. Agreement with the running original evaluator has also not
been measured.
