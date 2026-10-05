# Dataref registration table and the ACF-backed datarefs

Date: 2026-10-06. The reference and SHA256 are in [BASELINE.md](BASELINE.md). Static analysis of the EXE only;
the original was not run.

## Record layout

The records that register a dataref sit in `.data` with a stride of 128 bytes (16 qwords). Observed at
`sim/aircraft/weight/acf_m_empty` (`0x142f3cfe0`), `sim/aircraft/engine/acf_num_engines` (`0x142f33260`) and
`sim/time/total_running_time_sec` (`0x142fa0260`), where the next record starts exactly `0x80` bytes later:

| Slot | Content |
|---|---|
| 0 | VA of the NUL-terminated name in `.rdata` |
| 1 | low dword: type bits (1 int, 2 float, 4 double, 8 float array, 16 int array, 32 data); high dword: 1 when writable |
| 2, 3 | get / set int |
| 4, 5 | get / set float |
| 6, 7 | get / set double |
| 8, 9 | get / set int array |
| 10, 11 | get / set float array |
| 12, 13 | get / set data |
| 14, 15 | not interpreted |

The type bits and the slot order match the public `XPLMRegisterDataAccessor` argument order; the observed
records are consistent with it (an int dataref has only slots 2 and 3, a float dataref only slots 4 and 5).
The meaning of slots 14 and 15 was not established.

`tools/extract_datarefs.py` enumerates every record that satisfies the layout: the name points into `.rdata` at a
printable string containing `/`, the type word is valid, and each accessor slot is zero or points into `.text`.
These are byte-pattern candidates, not decoded runtime structures. On the reference build it finds 5503 records
with 5503 unique names and no duplicates:

| Type | Writable | Read-only |
|---|---:|---:|
| int | 1682 | 363 |
| float | 1549 | 481 |
| float array | 443 | 290 |
| int array | 159 | 67 |
| float and double | 60 | 260 |
| data | 39 | 110 |

Every `int` record has a get-int accessor and every `float` record a get-float accessor. The full listing
(name, type, access, accessor VAs) is written to `research/local/datarefs.tsv` and is not published:

```sh
python3 tools/extract_datarefs.py Xplane12/X-Plane.exe > research/local/datarefs.tsv
python3 tools/extract_datarefs.py Xplane12/X-Plane.exe --prefix sim/aircraft/weight/
```

## ACF properties and their datarefs

For six ACF properties the loader's destination field and a dataref's getter were compared in the disassembly.
The loader code is in [ACF_LOADING.md](ACF_LOADING.md); the field is an offset in the aircraft object.

| ACF property | Loader stores at | Dataref | Type | Access | Getter reads |
|---|---|---|---|---|---|
| `_m_empty` | `+0x288c` (`0x141aa60f5`) | `sim/aircraft/weight/acf_m_empty` | float | rw | `+0x288c` (`0x1418b7780`) |
| `_m_fuel_max_tot` | `+0x2888` (`0x141aa6133`) | `sim/aircraft/weight/acf_m_fuel_tot` | float | rw | `+0x2888` (`0x1418b92b0`) |
| `_m_max` | `+0x2898` | `sim/aircraft/weight/acf_m_max` | float | rw | `+0x2898` (`0x1418b7c80`) |
| `_cgY` | `+0x2850` (`0x141a97425`) | `sim/aircraft/weight/acf_cgY_original` | float | ro | `+0x2850` (`0x1418b6cd0`) |
| `_cgZ` | `+0x2854` (`0x141a9746e`) | `sim/aircraft/weight/acf_cgZ_original` | float | ro | `+0x2854` (`0x1418b6de0`) |
| `_num_engn` | `+0x91c` (`0x141a991ac`) | `sim/aircraft/engine/acf_num_engines` | int | rw | `+0x91c` (`0x1418675c0`) |

The stored values are the converted ones: kilograms for masses and metres for lengths, so a dataref read returns
the same number as `aircraft-info`. The `_m_max` offset was read from the instruction before the call at
`0x141aa6204`.

One condition differs: the `_num_engn` value is first read to the stack and stored into `+0x91c` only if a flag
at the start of the loader's context object (`0x6cf8(%rbp)`) is zero (`cmpl $0x0, (%rax)` at `0x141a991a3`). What
this flag means is not established, so the stored value is always applied in our registry.

The getters reach the aircraft object through a shared helper (`0x141a64260`, called with a table index);
how it selects the object, and so how datarefs behave with several aircraft loaded, is not recovered.

## Implementation and limits

`src/dataref.rs` has `Registry::register_aircraft`, which registers these six datarefs with the types and
writability above, all or nothing. The `datarefs <acf>` command prints them for an ACF:

```sh
cargo run --offline -- datarefs "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

Not done: the other 5497 datarefs of the catalog have no backing values; commands are not recovered; custom
datarefs created by scripts (for example `laminar/c172/...`) are not modelled; writes through the registry do not
propagate to the loaded aircraft object; the accessor of the running-time dataref is described in
[RUNTIME_TIME.md](RUNTIME_TIME.md).

## Checks

34 unit tests pass, Clippy without warnings, offline build. The new tests check the registered types and
writability, the read-only `acf_cgY_original`/`acf_cgZ_original`, a write to `acf_m_empty`, and that a duplicate
name leaves the registry unchanged. On the stock Cessna the command prints empty mass `780.632690`, maximum mass
`1160.289917`, fuel `158.757385`, CG Y/Z `0.091440`/`0.883920` and one engine, equal to `aircraft-info`.
