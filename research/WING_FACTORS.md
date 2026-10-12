# Verified wing flow and angle factors

Date: 2026-10-07. Reference SHA256 is in [BASELINE.md](BASELINE.md).
This completes two numerical blocks of roadmap item “port the wing element function in verified pieces”.
It does not complete that function or the original flight model.

## Flow block: 0x1411b6706 .. 0x1411b6776

Inputs are the wing's float32 at `RSI+0x14` and the incoming XMM6.
The first operand has been associated with aspect ratio in the surrounding code. The second operand's
source and physical interpretation remain unresolved; the API calls it efficiency and requires it explicitly.

Instruction order:

```text
denominator = float32(float32(aspect * PI_f32) * efficiency)
ratio = float32(SLOPE_f32 / denominator)
flow = float32(1.0_double / (double(ratio) + 1.0_double))
lift_scale = float32((sqrt(double(flow) * 8.0_double + 1.0_double) + 1.0_double) * 0.25_double)
alpha_multiplier = float32(flow / lift_scale)
```

PI is `0x40490fdb` at `0x142515178`; SLOPE is `0x40b758b4` at `0x142672ac8`.
The square root on the supported positive domain is the original SSE instruction, not a stubbed math call.
Double conversions, accumulation and the final division must remain in this order to preserve output bits.
openXplane rejects nonpositive/nonfinite inputs and nonfinite/zero-flow results. These validation limits
are its diagnostic contract; they are not recovered error handling from the original.

## Angle block: 0x1411b68fc .. 0x1411b6979

Inputs are the wrapped angle in XMM9, a divisor factor in XMM3 and a multiplier factor in XMM6.
Both factors follow the same arithmetic chain:

```text
slope = float32(float32(1 - factor) / 70)
offset = float32(abs(alpha) - 20)
value = float32(float32(slope * offset) + factor)
result = clamp(value, min(factor,1), max(factor,1))
```

70 is float32 `0x428c0000` at `0x14250e648`; 20 is at `0x14250e5f4`.
The transition runs from 20° to 90°, not 20° to 70°.
The old prototype divided by 50, and its test repeated that assumption. It is now replaced with the
original operation order, including divide-before-multiply and the bound comparisons.

## Verification

`tools/gen_wing_vectors.py` executes original instructions in Unicorn on synthetic inputs, with the PE
sections mapped unchanged. It checks the executable hash. Entry registers and the small wing/frame memory
are set explicitly; no callees are stubbed and no original instruction bytes are shipped in the fixture.
Each range must reach its exclusive end address; the emulator now rejects an instruction-budget stop
before a function returns or a range ends. An exact code hook enforces the range boundary even when it
falls inside a translation block cached by an earlier run.

```sh
python tools/gen_wing_vectors.py Xplane12/X-Plane.exe > tests/data/wing_factors.txt
cargo test --offline --test wing_vectors
```

The generator needs Unicorn, as does `emulate_xp.py`; it does not need numpy.
1036 flow cases and 1144 angle-factor cases match bit for bit in `tests/wing_vectors.rs`.
Inputs cover small/large positive denominator operands, factors below zero and above one, signed zero,
both signs of angle, exact and near-boundary angles 20°, 70° and 90°, and deterministic random samples.

## Integration and remaining work

`src/wing_aero.rs` contains the recovered blocks. `flight::fade_to_one` delegates to it, and the prototype
loads the flow/lift factors once per surface, using its existing whole-surface aspect ratio and explicit
efficiency=1 assumption. Obtaining the original per-element denominator operands is still necessary.
The profile divisor in this prototype remains 1: flap/slat factor construction has not yet been ported.

All four local Cessna flight scenarios and the existing 1200 profile vectors continue to pass after the
change. This establishes integration with the prototype, not equality of aircraft trajectories.
Next: geometry helpers, viscosity/Reynolds computation, flap/slat factor, span mixing, induced drag,
delta-wing and ice blocks, then the caller's input computation and forces.

The full suite passes 87 tests (one tuning diagnostic remains intentionally ignored); Clippy has no warnings.
Both wing-block fixtures and the earlier profile vectors also pass in the optimized release build.
The incomplete-execution guards were checked using a synthetic infinite loop and reject it for both
function calls and block ranges.
