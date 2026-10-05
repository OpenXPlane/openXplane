# AFL: table structure and the start of the aerodynamics research

Date: 2026-10-06. The reference EXE and its SHA256 are in [BASELINE.md](BASELINE.md). The original was not run:
the results of this stage rest on content and static analysis. The numeric resource version of the EXE is
`12.4.3.11`.

## Method

We follow the approach of an independent implementation that reads the original content, as described by
[openOMSI](https://github.com/openOMSI-Project/openOMSI). Its
[architecture document](https://github.com/openOMSI-Project/openOMSI/blob/main/docs/ARCHITECTURE.md) lists the
addresses of the functions studied and behaviour fixes made from analysing the original. In openXplane we also
record the build, the addresses, the observations, the implemented rule and the limits of verification.

## The reader that was found

From the string `This does not appear to be a valid airfoil for Airfoil Maker!` a candidate function
`0x141a42c10 .. 0x141a44160` was found (`.pdata` bounds). The instructions are confirmed with llvm-objdump. The
debug names of the nearest exports are not taken as names of these functions.

| Address / VA range | Observation |
|---|---|
| `0x141a42d70`, `0x141a42d7c` | Reading and comparing the device code with 1234 (`0x4d2`) |
| `0x141a42ef4` | A separate version 1110 branch (`0x456`) |
| `0x141a42f06`, `0x141a42f22` | Reading two header scalars |
| `0x141a42f39 .. 0x141a42feb` | 14 iterations reading two coordinates |
| `0x141a42ff6 .. 0x141a4301a` | Reading the table count and preparing storage |
| `0x141a43043 .. 0x141a433ab` | Reading 25 table parameters; structure stride `0x2230` |
| `0x141a434ce .. 0x141a437ed` | Row loop: index from 0 to 720 inclusive (`0x2d0`) |
| `0x141a437f3 .. 0x141a437f9` | Move to the next table |

In the row loop the first token (alpha) is skipped; three coefficient arrays are stored. In the content format
the rows have four columns: angle of attack in degrees, Cl, Cd, Cm. The official
[Airfoil Maker manual](https://developer.x-plane.com/manuals/airfoil_maker/) explains the purpose of these
coefficients. The official
[X-Plane 12 flight model report](https://developer.x-plane.com/article/x-plane-12-flight-model-report/) confirms
that several Reynolds regimes are stored in one airfoil.

The numbers in the first position of the NACA 2412 block: `0.1, 0.5, 1.0, 3.1`. In the code they are called
`parameters[0]` for now: the units, the regime choice and the blending of tables in the EXE are not yet
confirmed. The other parameters are also stored without guesses about their purpose.

## Implementation and differences

`src/airfoil.rs` reads AFL 1110, two scalars, 14 coordinate pairs, the table count, 25 parameters of each table
and 721 coefficient rows. It also stores the alpha column, although the studied loader of the original skips it.
Versions 700/900 are rejected for now.

openXplane's strict validation requires finite numbers, increasing angles, the exact number of columns and a
complete file. This is our error policy; equivalence with the original's handling of damaged files is not
established. The module does not substitute missing coefficients and does not generate an airfoil.

`airfoil-info <file.afl> [alpha-degrees]` shows all tables and a diagnostic linear lookup. The grid of the
provided files is non-uniform: a 0.1° step from -20° to +20°, a 1° step outside, 721 rows in total from -180° to
+180°. The lookup accounts for the spacing between neighbouring angles, keeps exact nodes and rejects angles
outside the table. There is no extrapolation, Reynolds blending, Mach corrections, finite wing, flaps or force
calculation.

The function `0x141a412c0 .. 0x141a41a2a` was found: its diagnostic strings contain `AOAinduced` and `3-D alpha`.
In the range `0x141a4175d .. 0x141a417ba` weighted sums of two neighbouring values from three arrays with offsets
`0x14`, `0xb58`, `0x169c` are visible. This is the next point of research; its whole interface, the angle
conversion and the application of corrections are not recovered yet. openXplane's diagnostic lookup is not
declared equivalent to this function for now.

Update from the next stage: the angle-correction and fixed-lookup section of this function was moved to
`src/aero.rs`, separate from `sample_linear`. Formulas, implementation limits and checks are in
[AERO_LOOKUP.md](AERO_LOOKUP.md).

`inspect` now parses the AFL files found through each aircraft's references. If both candidates exist (the local
and the shared directory), both are checked; the original's search priority is not established yet.

## Verification

All 34 provided AFL files were read without errors: 29 files with one table, 3 with three, 2 with four - 46
tables and 33166 coefficient rows in total. All airfoils found for the three Cessna variants pass `inspect`.

For NACA 2412 at 5° the result is Cl `0.760000` and Cm `-0.035580` in all four tables; Cd: `0.020150`,
`0.009120`, `0.008080`, `0.007270`. These are values of the provided files, not the result of a flight
comparison.

13 unit tests pass; Clippy without warnings; offline build. The new tests use our own synthetic data: several
tables, a non-uniform grid, exact nodes and intermediate angles, bounds, NaN/Infinity, corruption and truncated
input. The original AFL files are not copied into the tests.

The local reports in `research/local/` are excluded from Git: `afl-loader-xrefs.json`,
`afl-reader-disassembly.txt`, `airfoil-3d-corrections.txt`, `airfoil-path-loading.txt`,
`airfoil-library-audit.txt`, `airfoil-aircraft-audit.txt`.
