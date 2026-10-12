# Exact profile weights and execution completion checks

Date: 2026-10-12. Reference identity: [BASELINE.md](BASELINE.md).

## Span weights

`wing_element::profile_weights` now contains the recovered root/middle/tip weight stage
(`0x1411b6d3b..0x1411b6e8f`) used by the full wing evaluator. Ratios and name bytes are preserved,
equal bounds yield 0.5, equal adjacent names suppress the outer weight, and the middle remainder
is computed as two float64 subtractions followed by one float32 rounding. Overlapping ratios
may yield a negative middle; the original caller skips nonpositive profile weights.

The ramp explicitly preserves the +0 produced by ADDSS +0. A clamp alone can retain -0 after
optimized lowering. A strict corpus checks **2440 cases / 7320 output words** in debug and release.

The generator executes the entire original span block, including original CRT name comparisons,
with inline (capacity 15) and pointer-backed strings. Inputs cover equal/different/empty names,
embedded NULs, UTF-8, lengths 15/16/64, equal and unordered ratios, boundary fractions and
deterministic random cases. The output fixture stores equality flags; the generator records the
actual string inputs and does not stub their comparison or replace instructions.

This expands coverage of an already ported stage and fixes its zero rounding; it does not
complete the unported delta-wing coefficient block. No exhaustive IEEE or whole-game equality is claimed.

## Completion contract in the original-code emulator

The previous Emulator.call accepted whatever registers were present if Unicorn exhausted its
instruction budget before return. It now requires reaching the selected return/checkpoint address.
The shared run_range also uses an exact endpoint hook, including endpoints within a cached
translation block; incomplete stops and zero/unbounded budgets are rejected. Hooks are removed
on success and failure. Existing call arguments and optional `until` checkpoints remain supported.

Five synthetic tests require no original EXE: normal return, infinite loops in function/range
execution, checkpoints inside a cached block, premature external stops and zero budgets.
The geometry corpus and 2440 span cases regenerate byte for byte; the existing 400-case
straight-wing corpus also regenerates unchanged with the stricter completion contract.

```sh
python tools/test_emulate_xp.py
python tools/gen_exact_wing_span_vectors.py /path/to/X-Plane.exe > /tmp/span.txt
cmp /tmp/span.txt crates/xp-airfoil/tests/data/wing_span.txt
cargo test --offline -p xp-airfoil --test wing_span_exact
cargo test --release --offline -p xp-airfoil --test wing_span_exact
```

Python replay/tests require Unicorn; it is not an engine runtime dependency. Tests use the stated
baseline digest. The Rust corpus tests run without the original executable.
