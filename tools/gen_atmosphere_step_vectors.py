#!/usr/bin/env python3
"""Vectors for flight_step::atmosphere_step (the original 0x1412763c0(F)).

    python3 tools/gen_atmosphere_step_vectors.py Xplane12/X-Plane.exe TRIALS SEED > crates/xp-app/tests/data/atmosphere_step.txt
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase  # noqa: E402

EXE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3])


def main():
    case = VmCase(EXE, SEED, [(0x141270000, 0x141280000), (0x1406e0000, 0x1406f0000)])
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.randrange(2)))
    case.stub(0x141ba6750, lambda call, rng: call.ret_f32(rng.uniform(-40.0, 40.0)))
    case.stub(0x141ba6290, lambda call, rng: call.ret_f32(rng.uniform(-20.0, 20.0)))
    case.stub(0x141ba6df0, lambda call, rng: call.ret_f32(rng.uniform(0.0, 5000.0)))
    case.stub(0x141ba63a0, lambda call, rng: call.ret_f32(rng.uniform(0.2, 1.4)))
    print('# atmosphere_step vectors (tools/gen_atmosphere_step_vectors.py)')
    rng = case.rng
    for _ in range(TRIALS):
        case.reset()
        F = case.region('F', 0xc000)
        for i in range(0x803 * 2):
            case.emu.write_u32(0x14612bd90 + 4 * i, struct.unpack('<I', struct.pack('<f', 0.3 + ((i * 37) % 101) / 100.0))[0])
        alt = struct.unpack('<Q', struct.pack('<d', rng.uniform(-900.0, 14000.0)))[0]
        case.fz.preset('F', 0x3a0, alt & 0xffffffff)
        case.fz.preset('F', 0x3a4, alt >> 32)
        case.fz.preset_f32('F', 0x420, rng.uniform(0.0, 3.0))
        case.run(0x1412763c0, ints=[F])
        print(case.dump(f'{F:x}'))


if __name__ == '__main__':
    main()
