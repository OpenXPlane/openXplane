# Delta-wing target coefficients

Reference executable SHA256: `2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936`.
The original range `0x1411b7efa..0x1411b8779` constructs the four inputs to the
previously verified final delta blend. Diagnostics flags are explicitly zero.
The full range, including thickness lookup, CRT trigonometry and both noise
functions, runs without stubs in `tools/gen_delta_target_vectors.py`.

`wing_element::delta_coefficients` requires explicit angle, corrected sweep,
speed-like divisor, lift scale, midpoint thickness, extra coefficients,
coordinates, transverse reference scale, z reference, double time and noise table.
The first lift curve starts at slope * signed alpha. Past peak angle the exponent
is 0.25; past 60 degrees lift interpolates towards signed cos(alpha) at 90.
Moment is -0.15 * lift before extra terms. Drag uses the squared lift and thickness.
The noise activation uses abs(alpha) / peak angle. Both spatial noise coordinates
are divided by Aircraft+0x64f4; +0x64fc only offsets z. Noise2 time is multiplied
by four. Lift rounds to float32 between the two double noise factors.

The fixture records 1896 finite cases (396 boundary cases and 1500 random cases),
7584 coefficient words and 3792 original CRT endpoint words. With recorded CRT
results supplied through `DeltaTrig`, all coefficient words match exactly. On the
development macOS host, platform trigonometry matches 7560/7584 coefficient words;
the largest absolute discrepancy is 5.9604645e-8. The platform-path test allows
2e-6 absolute coefficient error; the arithmetic replay has no tolerance.
Double power rounded to float32 is local to this delta kernel.

The synthetic table recipe and hash match [NOISE2_EXACT.md](NOISE2_EXACT.md).
This is not evidence for its live initialization. Input validation bounds unsafe
conversion cases; malformed/nonfinite states and nonpositive divisors are rejected.
The whole positive-delta wing evaluator is a separate integration verification.
