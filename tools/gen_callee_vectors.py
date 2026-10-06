#!/usr/bin/env python3
"""Vectors for the pointer-following callees of callees.rs (engine_ratio 0x1408154c0, blend 0x1411daa80,
limit_a 0x1411a0900, limit_b 0x141218060) on lazily random-filled objects at absolute emulator addresses.

Also the force sinks (add_axial_force 0x1411767f0, add_normal_force 0x141176be0, add_side_force 0x141176e30,
add_world_force 0x141176a30), whose written words are compared (lines O).

Per trial one header line and the initial words (absolute hex addresses), then the answers of the stubbed
binding query 0x1407ace10 and the result:
  R <kind> <arg0> <arg1> <arg2> | <result>        kind: E (engine_ratio b n), B (blend obj mask),
                                                        A (limit_a obj i), P (limit_b obj n)
  W addr=word ...
  K id index answer                               binding query answers in call order

    python3 tools/gen_callee_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/callees.txt
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from xp_fuzz import Fuzz  # noqa: E402
from unicorn.x86_const import UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RAX, UC_X86_REG_XMM0  # noqa: E402

EXE = sys.argv[1]
SEED = 31


def hx(v):
    return f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}'


def main():
    emu = Emulator(EXE)
    fz = Fuzz(emu, EXE, [(0x140800000, 0x140900000), (0x1411a0000, 0x1411a1000), (0x1412180000 >> 4, 0x141219000),
                         (0x1411da000, 0x1411db000)], SEED)
    mark = emu.heap_top
    log = []

    def stub_binding(e):
        ans = fz.rng.randrange(2)
        e.uc.reg_write(UC_X86_REG_RAX, ans)
        log.append(f'K {e.uc.reg_read(UC_X86_REG_R8) & 0xffffffff} {e.uc.reg_read(UC_X86_REG_R9) & 0xffffffff} {ans}')

    emu.stubs[0x1407ace10] = stub_binding
    print('# callee vectors (tools/gen_callee_vectors.py)')
    for kind in 'EBAPXYZV':
        for trial in range(150 if kind in 'EBAP' else 50):
            emu.heap_top = mark
            fz.policy = {}
            log.clear()
            F = emu.alloc(0x100)
            Bx = emu.alloc(0x7000)
            Et = emu.alloc(0x68 * 4)
            Pt = emu.alloc(0x3770 * 4)
            O = emu.alloc(0x100)
            G = emu.alloc(0x1000)
            for name, addr, size in (('F', F, 0x100), ('B', Bx, 0x7000), ('E', Et, 0x68 * 4), ('P', Pt, 0x3770 * 4),
                                     ('O', O, 0x100), ('G', G, 0x1000)):
                fz.region(name, addr, size)
                emu.write(addr, bytes(size))
            fz.preset('B', 0x5ff8, Et & 0xffffffff)
            fz.preset('B', 0x5ffc, Et >> 32)
            fz.preset('B', 0x6010, Pt & 0xffffffff)
            fz.preset('B', 0x6014, Pt >> 32)
            fz.preset('O', 0, F & 0xffffffff)
            fz.preset('O', 4, F >> 32)
            fz.preset('O', 8, Bx & 0xffffffff)
            fz.preset('O', 12, Bx >> 32)
            fz.preset('O', 0x18, struct.unpack('<I', struct.pack('<f', fz.rng.choice([0.0, 0.5, 0.995, 1.0, fz.rng.uniform(-1, 1)])))[0])
            fz.preset('G', 0xdb4, struct.unpack('<I', struct.pack('<f', fz.rng.uniform(0.5, 3.0)))[0])
            if kind == 'B':
                # the blend object points at G for its second word
                fz.preset('O', 8, G & 0xffffffff)
                fz.preset('O', 12, G >> 32)
            for off in (0x2bc, 0x2d0, 0x2e4, 0x2f8, 0x310, 0x328, 0x294):
                fz.preset_f32('F', off, fz.rng.uniform(-3, 3))
            n = fz.rng.randrange(-1, 4)
            mask = fz.rng.randrange(0, 9)
            if kind == 'E':
                res = emu.call(0x1408154c0, ints=[Bx, n & 0xffffffff])
                result = hx(emu.xmm0_f32())
                head = f'R E {Bx:x} {n} 0'
            elif kind == 'B':
                emu.call(0x1411daa80, ints=[O, mask])
                result = hx(emu.xmm0_f32())
                head = f'R B {O:x} {mask} 0'
            elif kind == 'A':
                emu.call(0x1411a0900, ints=[O, abs(n)])
                result = str(emu.reg(UC_X86_REG_RAX) & 0xff)
                head = f'R A {O:x} {abs(n)} 0'
            elif kind == 'P':
                emu.call(0x141218060, ints=[O, abs(n)])
                result = str(emu.reg(UC_X86_REG_RAX) & 0xff)
                head = f'R P {O:x} {abs(n)} 0'
            else:
                # a non-finite force reaches the logging code of the original, which is not emulated
                vals = [fz.rng.uniform(-5, 5) for _ in range(3)]
                addr = {'X': 0x1411767f0, 'Y': 0x141176be0, 'Z': 0x141176e30}.get(kind)
                if kind == 'V':
                    force = [fz.rng.uniform(-5, 5) for _ in range(3)]
                    emu.call(0x141176a30, ints=[F], floats=[0, vals[0] if abs(vals[0]) < 1e30 else 1.0, vals[1] if abs(vals[1]) < 1e30 else 2.0, vals[2] if abs(vals[2]) < 1e30 else 3.0],
                             stack=[force[0], force[1], force[2]])
                    args = [vals[0] if abs(vals[0]) < 1e30 else 1.0, vals[1] if abs(vals[1]) < 1e30 else 2.0, vals[2] if abs(vals[2]) < 1e30 else 3.0] + force
                else:
                    emu.call(addr, ints=[F], floats=[0, vals[0], vals[1], vals[2]])
                    args = vals
                result = '0'
                head = f'R {kind} {F:x} 0 0 ' + ' '.join(hx(v) for v in args)
            print(head, '|', result)
            wr = fz.written()
            print('O', ' '.join(f'{fz.regions[n][0] + o:x}={w:08x}' for n in wr for o, w in wr[n].items()))
            words = {}
            for name in fz.regions:
                base = fz.regions[name][0]
                for off, w in fz.initial()[name].items():
                    words[base + off] = w
            print('W', ' '.join(f'{a:x}={w:08x}' for a, w in sorted(words.items())))
            for l in log:
                print(l)


if __name__ == '__main__':
    main()
