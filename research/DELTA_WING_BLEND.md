# Final delta-wing coefficient blending

Date: 2026-10-12. Baseline identity: [BASELINE.md](BASELINE.md).
Original range: `0x1411b8779..0x1411b885a` (exclusive end), inside `0x1411b6630`.
Port: `wing_element::blend_delta_coefficients`.

The block takes existing straight-wing Cl/Cd/Cm/ratio, target delta-wing values and a weight.
Each channel uses the original float32 order:

```text
difference = target - base
scaled = difference * weight
moved = scaled + base
low = minss(base, target)
result = low if low > moved else minss(maxss(base, target), moved)
```

This is a bounded blend, not an unconstrained interpolation: weights outside 0..1 still cannot
move the result outside the two endpoints on the tested finite domain. SSE min/max operand order
preserves the original choice for equal and signed-zero bounds. The final stall flag is set when
`abs(blended_ratio) > 1.0`; it is recomputed, not inherited from the incoming profile stall flag.

Register mapping at entry:

| Quantity | Original source / destination |
|---|---|
| Base/target Cl | XMM3 / XMM4; XMM0 starts as base, XMM1 as target; output *R14 |
| Base/target Cd | XMM12 / frame +0x168; output *R15 |
| Base/target Cm | XMM15 / frame +0x170; output *RSI |
| Base/target ratio | XMM8 / XMM9; output R12+0x1bc+4*R13 |
| Weight | XMM2 |
| Stall output | R12+0x1e4+4*R13; XMM14 abs mask, XMM11 1.0 |

**2228 cases / 11140 output words** match original instructions bit for bit in debug and
release. Coverage includes signed zeros, equal and reversed endpoint order, weights -1/0/1/2,
ratios at +/-1 and adjacent floats, and 2000 deterministic random finite cases. No callee is
invoked or stubbed. Generator verifies the EXE hash and the exact completion boundary.

```sh
python tools/gen_delta_blend_vectors.py /path/to/X-Plane.exe > /tmp/delta_blend.txt
cmp /tmp/delta_blend.txt crates/xp-airfoil/tests/data/delta_blend.txt
cargo test --offline -p xp-airfoil --test delta_blend_exact
cargo test --release --offline -p xp-airfoil --test delta_blend_exact
```

The function requires both coefficient sets explicitly. The preceding delta-wing coefficient
construction, trigonometric/noise/time terms remain unported, so the full evaluator continues
to report or explicitly skip that path as before. This helper does not silently invent those
inputs and does not make swept-wing trajectories identical to the original. The initial three
pure curve ramps at `0x1411b7efa..0x1411b7fbd` are the next isolated boundary.
