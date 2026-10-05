# Selecting and blending AFL tables

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the EXE only;
the original was not run for comparison.

## The calling function

By relative CALL instructions three candidate calls of `0x141a412c0` were found, confirmed by the disassembler:
`0x141a45499`, `0x141a45737`, `0x141a457a9`. All are inside the function `0x141a44350 .. 0x141a46152` according
to the `.pdata` table.

The multiplier and divisor of the angle correction are not computed here: on entry XMM2 is saved in XMM13 and
XMM3 in XMM12. The angle also arrives from above. Before the evaluator is called they are passed through
registers and the stack. Their origin in the higher calling code is not established yet.

The input parameter for table selection is read into XMM10 from `RBP+0x378` at `0x141a443f9`. In the prologue
RBP equals the input RSP minus `0x338`, so the source is a stack argument at the input `RSP+0x40`. The name and
physical units of the argument are not recovered yet.

## Selecting the pair

The range `0x141a44420 .. 0x141a4449f` scans structures with a stride of `0x2230`, comparing the input with the
float32 at offset 0 (`parameters[0]` of the AFL). Two passes find the nearest value from below and from above.
The distances are initialised to `FLT_MAX`: the bits `0x7f7fffff` at `0x142546c1c`.

A strict comparison of distances keeps the first table on a tie. If one side is missing, the candidate from the
other side is used. On an exact match both indices are equal; out of range, one extreme table is chosen. Sorting
of the input tables is not required. openXplane rejects an empty airfoil and non-finite inputs; the original's
error and recovery branches are not reproduced yet.

At `0x141a45415` the chosen indices are compared. If they are equal, the evaluator is called once. If they
differ, it is called twice, after which the output values are blended separately.

## The helper interpolation

The calls `0x141a457da`, `0x141a45812`, `0x141a4584f` go to `0x1406ea0b0`. The interface of this function is
`x0, y0, x1, y1, x`. The last input is passed on the stack. The confirmed order of operations:

```text
if x0 == x1:
    return (y0 + y1) * 0.5
dy = y1 - y0
dx = x1 - x0
offset = x - x0
slope = dy / dx
delta = slope * offset
value = delta + y0
return clamp(value, min(y0,y1), max(y0,y1))
```

The coefficient 0.5 is the bits `0x3f000000` at `0x14250e248`. The computations are scalar float32. Equal x
values are averaged, not resolved to an arbitrary side. For a pair obtained by table selection this case is
usually removed by the single-index branch.

## Implementation and limits

`src/regimes.rs`: `select`, `blend`, `evaluate_table_stage`. CLI `airfoil-mix <afl> <alpha> <multiplier>
<divisor> <parameter>`. First both chosen tables are evaluated separately through the implemented `aero`
section, then the results are blended. The table parameters are not averaged before the angle is computed: that
would be a different algorithm.

The original calling function blends the outputs of the full evaluator, including the late corrections. They are
now connected in the separate `profile` module, see [PROFILE_PIPELINE.md](PROFILE_PIPELINE.md). So the
openXplane command still shows a diagnostic composition of two recovered sections, not the full result of the
original function or a wing force.

## Checks

18 unit tests pass, Clippy without warnings, offline build. Checked: unsorted tables, duplicates, exact values,
bounds, a single table, an empty set, both interpolation directions, saturation, equal abscissas and non-finite
inputs.

All 34 provided airfoils pass three values of the input parameter (`0.01`, `0.75`, `100`) - 102 runs without
errors. NACA 2412, angle 5°, multiplier=divisor=1, parameter=0.75: tables 1/2 are chosen, Cl `0.759999990`, Cd
`0.008600000`, Cm `-0.035580002`. These are results of our diagnostic section, without comparison against the
running EXE.

Local evidence in `research/local/`, excluded from Git: `airfoil-evaluator-call-candidates.json`,
`airfoil-evaluator-caller.txt`, `airfoil-regime-blend-helper.txt`, `airfoil-regime-audit.txt`.
