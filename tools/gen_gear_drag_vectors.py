#!/usr/bin/env python3
"""Vectors for flight_state::apply_drag (the original 0x14119e5b0, the drag of a gear point).

    python3 tools/gen_gear_drag_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/gear_drag.txt

The header of a case is `F x y z drag` (float32 bits). The point transform 0x1407ac020 (and the probe's grid height and normal helpers)
are replayed (their results land in the callee frame, which the test rebases onto its own scratch).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp, setup_terrain, stub_terrain_helpers  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x14119e000, 0x14119f000), (0x1408e0000, 0x1408e4000), (0x140c40000, 0x140c50000)])
    rng = case.rng

    def transform(call, rng):
        # 0x1407ac020 writes the point to r8 (x), stack arg 5 (y) and stack arg 7 (z)
        for address in (call.ints[2], call.stack[0], call.stack[2]):
            call.put_f32(address, rng.choice([0.0, rng.uniform(-10, 10)]))

    case.stub(0x1407ac020, transform)
    stub_terrain_helpers(case)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.0, 0.1)))
    case.stub(0x1408e25a0, lambda call, rng: None, record=False)
    print('# gear drag vectors (tools/gen_gear_drag_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        F = case.region('F', 0x44000)
        fz = case.fz
        sp = entry_rsp(1)
        fz.region('S', sp - 0x800, 0x900)
        for off in (0xbcd0, 0xbcc8):
            fz.preset('F', off, 0)
        for off in (0x2c4, 0x2ec, 0x2d8, 0x300, 0x318, 0x330, 0x340, 0x3cc, 0x3d0, 0x3d4, 0x430, 0x434, 0x440, 0x444,
                    0x450, 0x454, 0x368, 0x36c, 0x370):
            fz.preset_f32('F', off, rng.uniform(-1, 1) if off >= 0x3cc and off < 0x460 else rng.uniform(-50, 50))
        setup_terrain(case, rng)
        x, y, z = (rng.choice([0.0, rng.uniform(-5, 5)]) for _ in range(3))
        drag = rng.choice([0.0, rng.uniform(0, 4), rng.uniform(0, 60)])
        fz.preset('S', 0x828, bits(drag))
        fz.preset('S', 0x82c, 0)
        try:
            case.run(0x14119e5b0, ints=[F], floats=[0.0, x, y, z], stack=[bits(drag)], until=0x14119e95b,
                     max_instructions=200000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x} {bits(x):08x} {bits(y):08x} {bits(z):08x} {bits(drag):08x}'
        text = case.dump(header)
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
