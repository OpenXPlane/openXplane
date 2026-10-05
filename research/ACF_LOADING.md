# Reading ACF parameters and OBJ numbers - 2026-10-06

The provided `X-Plane.exe` (PE32+ AMD64) was examined. The numeric resource version is `12.4.3.11`; it does not
establish the name of a public release. SHA256: `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
All addresses below are virtual addresses of this build. They are confirmed by disassembly, not only by a
search for matching bytes.

## Aircraft parameters

In function `0x141a972a0` the property handler calls helper `0x140c99dc0`: it reads a number through
`0x140c99f40`, multiplies it with a `mulss` instruction by the coefficient passed in, and stores a float32.

| Property | String reference | Read call | Coefficient |
|---|---|---|---|
| `_m_empty` | `0x141aa60d3` | `0x141aa610c` | mass |
| `_m_fuel_max_tot` | `0x141aa6111` | `0x141aa614a` | mass |
| `_m_max` | `0x141aa61cb` | `0x141aa6204` | mass |
| `_cgY` | `0x141a973fa` | `0x141a97436` | length |
| `_cgZ` | `0x141a9744b` | `0x141a97478` | length |

The mass coefficient at `0x142613b30` has the bits `0x3ee83d47`: `0.4535925090312958`. The length coefficient at
`0x14250e208` has the bits `0x3e9c0ebf`: `0.30480000376701355`. These convert pounds to kilograms and feet to
metres, with the rounding chosen by the original build.

For `_num_engn` the reference is at `0x141a99142`, and the call of the integer reader `0x140c9a020` is at
`0x141a9916a`.

The implementation is in `src/aircraft.rs`; the command is `aircraft-info`. It currently requires ACF 1200 and
rejects missing, repeated, non-numeric or infinite required values and negative masses. This is openXplane's
validation policy; how the original handles such errors is not established yet. Maximum fuel mass is not taken
for the current fuel load. CG coordinates are kept in the ACF frame: axis transformations and their use in
physics are not implemented yet.

## OBJ particulars

The number reader `0x140873240` accumulates the value and a decimal divisor in double, then the caller converts
the result to float32. A dot starts the fractional part; a repeated dot does not reset the divisor. So the stock
spelling `-2.5.000000` is read as `-2.5`. If the line has no next argument, the number reader returns zero. This
explains the six-number `ANIM_trans` and the five-number `ANIM_rotate` in `lights.obj`: the missing calibration
values are zero.

The `ANIM_rotate_key` branch passes the key and the value to function `0x1408751b0` (call `0x140878304`). It
finds the position by binary search, inserts the key into the sorted sequence, and replaces the value of an
existing equal key. So the order `0, -20, +20` is allowed.

These rules are implemented in `src/obj8.rs`; non-standard entries are flagged in the diagnostics. Required
coordinates stay required. Our scanner accepts signs, digits and dots and rejects other characters and
exponents; full equivalence with the original scanner on arbitrary text is not claimed. The behaviour was
established by static analysis; comparison with the original running on Windows has not been done yet.

## Check of the provided content

| Cessna variant | OBJ parsed | Declared triangles | Empty mass, kg | Maximum mass, kg |
|---|---:|---:|---:|---:|
| Standard | 42/42 | 191536 | 780.632690 | 1160.289917 |
| G1000 | 27/27 | 186071 | 780.632690 | 1160.289673 |
| Seaplane | 44/44 | 193077 | 780.632690 | 1006.975342 |

In all three variants the base textures were found, the maximum fuel mass is `158.757385` kg, CG Y/Z is
`0.091440` / `0.883920` m, and there is one engine. The triangle counts are for the full audit of the files, not
for a viewer frame. Unsupported commands for instruments, lights, materials and controls are still printed; a
successful geometry parse does not mean full compatibility.

Checks: 11 unit tests, Clippy without warnings, offline build. After the changes a 1280×800 frame was saved
again through Apple M4 / Metal; the Cessna image was checked visually (`research/local/cessna-preview.png`).
Local disassemblies and reports are excluded from Git: `acf-properties-disassembly.txt`,
`acf-float-reader.txt`, `acf-mass-loading.txt`, `obj-number-reader.txt`, `obj-simple-animations.txt`,
`obj-key-insertion.txt`, and the `*-mesh-audit.txt` files in `research/local/`.
