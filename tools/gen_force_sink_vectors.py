#!/usr/bin/env python3
"""Vectors for flight_state::add_plugin_force (the original 0x1408e3230).

    python3 tools/gen_force_sink_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/force_sink.txt

The header of a case is `F a b c d p e` (the floats as float32 bits: the point `a, c`, the force `b, d, e` and the
fifth argument `p`). The stack frame words are not compared.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1408e0000, 0x1408e4000), (0x140c40000, 0x140c50000)])
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.0, 0.1)))
    # the finite-value check (repairs a non-finite word) is a diagnostic: it does nothing for finite values
    case.stub(0x1408e25a0, lambda call, rng: None, record=False)
    rng = case.rng
    print('# plugin force vectors (tools/gen_force_sink_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        F = case.region('F', 0xc000)
        fz = case.fz
        sp = entry_rsp(4)
        fz.region('S', sp - 0x800, 0x900)
        for off in (0xbcd0, 0xbcc8):
            fz.preset('F', off, 0)
        for off in (0x2c4, 0x2ec, 0x2d8, 0x300, 0x318, 0x330, 0x340):
            fz.preset_f32('F', off, rng.uniform(-50, 50))
        a, b, c, d, p, e = (rng.choice([0.0, rng.uniform(-5, 5), rng.uniform(-100, 100)]) for _ in range(6))
        for k, value in enumerate([bits(d), bits(p), bits(e), 0x42f05e60]):
            off = sp + 0x28 + 8 * k - (sp - 0x800)
            fz.preset('S', off, value)
            fz.preset('S', off + 4, 0 if k < 3 else 1)
        try:
            case.run(0x1408e3230, ints=[F], floats=[0.0, a, b, c], stack=[d, p, e, 0x142f05e60],
                     until=0x1408e379d, max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x} {bits(a):08x} {bits(b):08x} {bits(c):08x} {bits(d):08x} {bits(p):08x} {bits(e):08x}'
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
