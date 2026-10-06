# ACF loader schema extracted from the reference build

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the EXE only.

[ACF_LOADING.md](ACF_LOADING.md) confirmed a handful of properties by hand. The loader functions share one
code shape, so `tools/extract_acf_schema.py` reads the same facts for every property it can recognise.

## The shape

For each property the loader runs a block of straight-line code:

1. the key string is loaded (`leaq <string>(%rip)`, often as a 16-byte string_view `{pointer, length}` stored
   at `-0x30(%rbp)`);
2. the destination is put in `RDX` (`leaq OFF(%reg), %rdx`, `addq $OFF, %rdx` or a plain `mov`), where `OFF` is
   an offset into the object being filled;
3. for float properties with a unit conversion, the coefficient is moved into `XMM3`;
4. a reader function is called: `0x140c99f40` float, `0x140c99dc0` float times the coefficient in `XMM3`,
   `0x140c9a020` int, `0x140c9a240` string.

The tool prints `key, loader function, base register, offset, reader, coefficient`. It scans four loader
functions: `0x141a972a0` (aircraft-wide values, 1035 properties), `0x141aad640` (wing), `0x141aad150` (part
position and links) and `0x141aaabe0` (engine). On the reference build it recognises 1117 property reads: 662
float, 86 float with a coefficient, 339 int and 9 string.

```sh
python3 tools/extract_acf_schema.py Xplane12/X-Plane.exe > research/local/acf-schema.tsv
python3 tools/extract_acf_schema.py Xplane12/X-Plane.exe --key _semilen
```

## Agreement with the hand-confirmed facts

The output reproduces every value in [ACF_LOADING.md](ACF_LOADING.md) and [DATAREFS.md](DATAREFS.md):
`_m_empty` `+0x288c`, `_m_fuel_max_tot` `+0x2888`, `_m_max` `+0x2898` with the mass coefficient
`0.4535925090312958`, `_cgY` `+0x2850` and `_cgZ` `+0x2854` with the length coefficient `0.30480000376701355`.

## Wing fields (function `0x141aad640`, base = the wing object)

| Property | Offset | Type |
|---|---|---|
| `_is_right_mult` | `+0x00` | float |
| `_els` | `+0x04` | int |
| `_Croot`, `_Ctip`, `_semilen_SEG` | `+0x08`, `+0x0c`, `+0x10` | float, feet to metres (`0.3048`) |
| `_dihed_design`, `_dihed_full_up`, `_var_dihed` | `+0x20`, `+0x24`, `+0x2c` | float, float, int |
| `_sweep_design`, `_sweep_full_aft`, `_var_sweep` | `+0x30`, `+0x34`, `+0x3c` | float, float, int |
| `_sweep_rotation_span_os` | `+0x40` | float, feet to metres |
| `_incid_full_up`, `_incid_chrd_inc`, `_var_incid` | `+0x44`, `+0x48`, `+0x4c` | float, float, int |
| `_retract_max`, `_var_retract` | `+0x50`, `+0x58` | float, int |
| `_foil_rat_rot`, `_foil_rat_mid_inner`, `_foil_rat_mid_outer`, `_foil_rat_tip` | `+0x5c`, `+0x60`, `+0x64`, `+0x68` | float |
| `_plan_ellipse_frac` | `+0x6c` | float |
| `_afl_file_1`, `_afl_file_2`, `_afl_file_3` | `+0x35b8`, `+0x35d8`, `+0x35f8` | string |
| `_*_hyd0123` (aileron, elevator, rudder, spoiler, yaw brake, speed brake) | `+0x644 ..+0x66c` | int |

For old files the loader first reads `_is_right_mult` and, when a flag in the load context is zero, the legacy
keys `_afl_file_R0` (stored into both the first and the second airfoil slot) and `_afl_file_T0` (the third) and
skips the new keys; otherwise it reads `_afl_file_1/2/3`. What the flag means is not established. The three
airfoils sit about `0x20` apart as fixed-size string fields.

Arrays of 11 values (`_chord_for_RIB` from `+0x70`, then `_c_off_for_RIB` and `_c_rat_for_RIB`) are read with a
different reader (`0x140f15300`, element count passed in `R8`) and are not covered by the tool yet.

## Limits

- It recognises the straight-line pattern only. Properties that are read through loops, index formatting, other
  readers or other code shapes are missing; for example `_num_engn` (read to the stack and stored under a
  condition, see [DATAREFS.md](DATAREFS.md)) and the arrays above.
- The coefficient is tracked linearly through each function, so a value loaded once and reused after a branch is
  carried forward. It is a heuristic: the values printed for the engine block (`0.10471976548433304` for rpm to
  rad/s, `4.444426536560059`) were not checked against the code, and `4.4444` is not the usual pound-force to
  newton factor, so do not rely on them. The mass and length coefficients above were confirmed by hand.
- The object each loader fills (the "base") is only a register name; which runtime structure it is, and the
  meaning of the context flag, are not established.

## Checks

The tool was run on the reference build and its output compared with the hand-read disassembly of
`0x141aad640` (the first block, with the legacy-key branch, was read in full). The Rust side does not use this
table yet; `src/aircraft.rs` still holds only the six hand-confirmed values.
