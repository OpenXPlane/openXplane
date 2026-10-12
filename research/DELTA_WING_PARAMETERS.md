# Initial delta-wing parameter curves

Date: 2026-10-12. Baseline identity: [BASELINE.md](BASELINE.md).
Original range `0x1411b7efa..0x1411b7fbd` (exclusive endpoint).
Port: `wing_element::delta_parameters`; sole input is the sweep float at frame -0x58.

The original calls pure helper `0x14082b800` three times with common bounds 45..75 degrees
and exponent 0.75. The three output pairs are 0.07..0.035 (lift slope, frame +0x178),
25..40 (peak angle, +0x158), 0.9..1.5 (peak lift, +0x180). Names describe subsequent use;
this stage has no time, runtime noise or aircraft state inputs.

The port retains float32 reciprocal, offset, clamping, ordinate subtraction, scaling and addition.
The power is evaluated in double and rounded once to float32 locally for these three curves.
The initially delegated platform float32 powf differed in five final parameter words by one ULP;
the double-power version matches the original corpus. Generic engine::curve/signed_pow are
unchanged; their other domains have not been established exact by this test.

**2027 cases / 6081 outputs** compare bit for bit in debug and release on the tested host.
Coverage includes 45/75 and adjacent float32 sweeps, both signs of zero, negative/large sweeps
and 2000 deterministic random values. The generator runs all three original curve calls and
their original CRT power implementation without stubs, checks the baseline digest, exact
endpoint and unchanged CRT selector. This establishes this corpus, not exhaustive libm equality.

```sh
python tools/gen_delta_parameter_vectors.py /path/to/X-Plane.exe > /tmp/delta_parameters.txt
cmp /tmp/delta_parameters.txt crates/xp-airfoil/tests/data/delta_parameters.txt
cargo test --offline -p xp-airfoil --test delta_parameters_exact
cargo test --release --offline -p xp-airfoil --test delta_parameters_exact
```

The initial and [final blend](DELTA_WING_BLEND.md) stages are available as explicit-input APIs.
The intermediate coefficient construction and time/noise path remain unported; the full evaluator
continues to reject or explicitly skip that branch rather than use invented intermediate results.
