# Verified span weights and profile input coordinates

Date: 2026-10-12. Reference identity: [BASELINE.md](BASELINE.md).
Implementation: `wing_aero::span_weights`, `wing_aero::span_inputs`; original function `0x1411b6630`.

## Weights — 0x1411b6d3b..0x1411b6e8f

Root ramps from wing +0x60 to +0x5c; tip ramps from +0x64 to +0x68. Every ramp uses float32:
subtract bounds, divide 1 by the difference, subtract the start from span fraction, multiply,
add +0, clamp to 0..1. Equal bounds produce 0.5. Root/middle name equality suppresses root;
middle/tip equality suppresses tip. Ratios are not sorted or repaired.

The original compares std::string lengths and calls CRT `0x1423c0180` on equal lengths.
The generator executes this original comparison without stubs. It constructs both inline strings
(capacity 15) and pointer-backed strings, including equal/different bytes of equal lengths,
empty names, embedded NULs, UTF-8 and lengths 15/16/64. Rust delegates exact byte equality to `str == str`.
The fixture stores resulting equality flags; the generator retains the names and layout used to obtain them.

The middle weight is `f32((1.0_f64 - f64(root)) - f64(tip))`, **not** two float32 subtractions.
The old implementation rounded after the first subtraction. The fixture explicitly exposes this error.
Optimized Rust min lowering also produced -0 where the original ADDSS +0 produces +0; the port
explicitly canonicalizes a zero ramp. Both debug and release now match all **2440** cases.
Overlapping ratios can produce a negative middle weight. It is preserved rather than normalized;
the caller only evaluates a profile with a positive weight.

This is integrated: `flight::foil_weights` delegates to the recovered operation chain. Span fractions
still come from the prototype's existing element geometry, so complete original forces are not established.

## Position/noise coordinates — 0x1411b6c10..0x1411b6d60

Inputs: int element count wing +4, int index R13, coordinate arrays wing +0x5bc/+0x5e8/+0x614,
reference divisors aircraft +0x64f4/+0x64f8/+0x64fc.

1. Convert the index to float32, promote to float64, add 0.5, round to float32, divide by float32
   element count. This yields the span fraction used for weights.
2. Clamp the segment index to 0..elements-1. For each axis, call original clamped interpolation
   `0x1406ea0b0` with segment and segment+1 as abscissae, the corresponding coordinate array
   entries as ordinates, and **the integer index converted to float32** as interpolation input.
3. Divide the float32 interpolated coordinate by its corresponding reference value and store:
   x frame +0x128, y +0x120, z +0x158.

The previous research note inferred interpolation at the element centre. The instruction inputs
show otherwise: `index + 0.5` is used for weights only; coordinates use `index`. Their physical
meaning and construction by the earlier geometry code remain unresolved.

**1105** vectors cover elements 1..10, every valid index, out-of-range indices on both sides,
negative/reference-sign variations, arbitrary corner coordinates and deterministic random inputs.
All four outputs match bit for bit in debug/release. The generator executes original interpolation.

The API rejects nonfinite inputs, zero reference divisors, point counts outside 2..11, arithmetic
failure and indices outside the range exactly representable as float32. These are diagnostic restrictions,
not original checks. Out-of-range indices are tests of this isolated block; other parts of the full original
function may access such indices before this block and are not claimed safe.
The coordinates are not integrated into the prototype using guessed corner arrays or reference divisors.

## Reproduce

```sh
python tools/gen_wing_span_vectors.py Xplane12/X-Plane.exe > /tmp/wing_span.txt
cmp /tmp/wing_span.txt tests/data/wing_span.txt
python tools/gen_wing_coordinate_vectors.py Xplane12/X-Plane.exe > /tmp/wing_coordinates.txt
cmp /tmp/wing_coordinates.txt tests/data/wing_coordinates.txt
cargo test --offline --test wing_span --test wing_coordinates
cargo test --release --offline --test wing_span --test wing_coordinates
```

Requires Unicorn, verifies the executable hash and exact end boundaries. No original instruction
bytes are distributed. Recorded results cover stated inputs only, not every IEEE operand or the full caller.
REA/Hopper could identify the supplied PE but its launcher bridge failed before analysis readiness;
this result rests on the saved original disassembly and controlled original-code replay, not on a successful
REA decompilation. The pending geometry helpers are being investigated independently.
