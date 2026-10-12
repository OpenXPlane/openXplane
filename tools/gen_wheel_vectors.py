#!/usr/bin/env python3
"""Vectors for wheel::wheel_model (the original 0x1411c8690, one frame of a gear wheel's ground contact).

    python3 tools/gen_wheel_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/wheel_model.txt

The header of a case is `W aircraft gear result`. The rbp frame of the original is not compared (its words are
filtered); the callees outside the port are replayed: the time source, the engine flag, the debug query, the
binding query, the geoid height, the planet lookup, the sound trigger and the vector growth.
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX  # noqa: E402
from xp_vmcase import VmCase, entry_rsp, setup_strut_world, setup_wheel_world  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])
IDS = [0x88, 0x89, 0x8a, 0x8b]


def bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


def main():
    case = VmCase(EXE, SEED, [(0x1411c8000, 0x1411cc000), (0x141240000, 0x141242000), (0x1410c9000, 0x1410ca000),
                              (0x141182000, 0x141184000), (0x1412029d0, 0x141203000), (0x1411dd000, 0x1411df000),
                              (0x141a6b000, 0x141a6c000), (0x141294000, 0x141295000), (0x141964000, 0x141965000),
                              (0x1408e2000, 0x1408e4000), (0x141296000, 0x141297000), (0x1407d6000, 0x1407d7000),
                              (0x1407ac000, 0x1407ae000), (0x140f2a000, 0x140f2c000), (0x140816000, 0x140817000),
                              (0x1419f9000, 0x1419fa000), (0x141a64000, 0x141a65000), (0x140984000, 0x140985000),
                              (0x1406ea000, 0x1406eb000), (0x140cb9000, 0x140cbc000), (0x140cc3000, 0x140cc4000),
                              (0x14089c000, 0x14089d000), (0x141238000, 0x141239000)])
    rng = case.rng
    print('# wheel model vectors (tools/gen_wheel_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        F = case.region('F', 0x44000)
        B = case.region('B', 0x6700)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff, record=False)
        fz.preset('S', 0x804, RET_ADDR >> 32, record=False)
        setup_strut_world(case, rng, F, B, mesh_triangles=(20, 40), mesh_extent=rng.choice([2.0, 3.5, 6.0]), mesh_heights=(-1.2, 0.4))

        G, WRS = setup_wheel_world(case, rng, F, B, gears=3)
        gear = rng.randrange(0, 3)
        WR = WRS[gear]
        try:
            case.run(0x1411c8690, ints=[WR, 0, gear, 0], max_instructions=2000000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xff
        import os
        if os.environ.get('XP_FRAME') == str(done):
            rbp = sp - 0x1d0 - 0x8  # entry rsp - 0x1d8
            rbp = (sp + 0x0) - 0x1d8
            for off in range(-0x80, 0x200, 4):
                w = case.emu.read_u32(rbp + off)
                sys.stderr.write(f'frame {off:+#x}: {w:08x} {struct.unpack("<f", struct.pack("<I", w))[0]!r}\n')
        text = case.dump(f'{WR:x} 0 {gear:x} {result:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
