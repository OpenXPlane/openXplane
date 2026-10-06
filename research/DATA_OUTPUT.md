# Data-output lines (frame rate and others)

Source: `engine/dout/dout_lines.cpp` in the reference build. Extracted by `tools/extract_dout.py` into
`crates/xp-app/assets/dout/lines.tsv`.

## Established from the code

- `0x1416b4ed0` builds the line for index `0..0xac` (173 lines) with a jump table (`0x1416b4ed0..0x1416c9d54`),
  calling the line constructor `0x1416b3ce0`; indexes from `0xad` are an error.
- A line holds a label string of at most 95 characters, which is 8 cells of 12 characters. The constructor
  replaces every `_` with a space. A cell has a pointer/kind and a float value; kind 0 and 1 point at -1000.0,
  kind 3 at 1.0, anything else keeps the given pointer (live data).
- The frame-rate line (index 0) reads `f-act /sec`, `f-sim /sec`, `frame time`, `cpu time`, `gpu time`,
  `vblnk sync`, `grnd ratio`, `flit ratio`; when the vertical-blank value is unavailable (`0x140dad6d0`
  false) the build uses a second label without that cell (`_____-_____` marks an empty cell).
- `0x141008cd0` walks the enabled lines in display order (order table `0x142f288a0`, enable flags at
  `0x145e4e580 + 0xad`); the drawing and live-value code is `0x1416cc3a0` (14 KB).

## Estimated from a photograph, not from code

The three-row cell layout (upper label, value, lower label), the 8-character cell pitch, the teal text colour
and the value format (six characters with as many decimals as fit: `10.462`, `0.0956`, `1.0000`). These are
implemented in `crates/xp-app/src/dout.rs` and may differ from the original.

## Not established

The drawing routine `0x1416cc3a0`, the meaning of the cell kinds, the screen position, the original bitmap font,
and the value sources of every line. Lines 0 (frame rate), 3, 4, 8, 13, 17, 18, 20, 21 and 25 are drawn with values from the approximate flight
model (`dout::line_values`); cells without a source are empty (no GPU time, g-loads, magnetic values,
latitude and longitude). The numbers of the other lines of the table need engine, weather and electrical
systems that do not exist here.
