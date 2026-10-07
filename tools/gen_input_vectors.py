#!/usr/bin/env python3
"""Vectors for input::key_slot (0x1417da870) and input::bank_available (0x141185030).

    python3 tools/gen_input_vectors.py Xplane12/X-Plane.exe keyslot|avail TRIALS SEED > crates/xp-app/tests/data/input_KIND.txt

Header: `KIND obj arg result`. Stack words are not compared.
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import RET_ADDR  # noqa: E402
from xp_vmcase import VmCase, entry_rsp  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_XMM0  # noqa: E402

EXE, KIND, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])


def main():
    case = VmCase(EXE, SEED, [(0x1417da000, 0x1417db000), (0x141185000, 0x141186000), (0x14076b000, 0x14076c000), (0x141245000, 0x141247000), (0x14125e000, 0x141261000), (0x1407cc000, 0x1407cd000), (0x1407e7000, 0x1407e8000), (0x140819000, 0x14081a000)])
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
    import os
    trace = os.environ.get('XPTRACE')
    if trace:
        from unicorn import UC_HOOK_CODE
        from unicorn.x86_const import UC_X86_REG_XMM0
        specs = {}
        for part in trace.split(';'):
            addr, regs = part.split(':')
            specs[int(addr, 16)] = [int(r) for r in regs.split(',')]

        def on_code(uc, address, size, user):
            if address in specs:
                vals = []
                for r in specs[address]:
                    raw = uc.reg_read(UC_X86_REG_XMM0 + r) & 0xffffffffffffffff
                    vals.append('x%d=%g/%g' % (r, struct.unpack('<f', struct.pack('<I', raw & 0xffffffff))[0], struct.unpack('<d', struct.pack('<Q', raw))[0]))
                sys.stderr.write('%x %s\n' % (address, ' '.join(vals)))
        case.emu.uc.hook_add(UC_HOOK_CODE, on_code)
    rng = case.rng
    print(f'# input vectors {KIND} (tools/gen_input_vectors.py)')
    done = 0
    while done < TRIALS:
        case.reset()
        fz = case.fz
        OBJ = case.region('OBJ', 0xd000)
        B = case.region('B', 0x3000)
        sp = entry_rsp(0)
        fz.region('S', sp - 0x800, 0x900)
        fz.preset('S', 0x800, RET_ADDR & 0xffffffff)
        fz.preset('S', 0x804, RET_ADDR >> 32)
        arg = 0
        if KIND == 'keyslot':
            arg = rng.randrange(0, 8)
            for k in range(500):
                fz.preset('OBJ', 0x8cf8 + 4 * k, rng.randrange(0, 8))
                fz.preset('OBJ', 0x8cf8 + 0xba44 + 4 * k, rng.choice([0, 1, 1]))
            run = (0x1417da870, [OBJ, arg])
        elif KIND == 'start':
            FF = case.region('FF', 0xc000)
            BB = case.region('BB', 0x1000)
            case.region('SK', 0x10)
            case.stub(0x1407cdce0, lambda call, rng: None)
            case.stub(0x1407d6a00, lambda call, rng: None)
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.01, 0.02, 0.3, 1.0, 5.0, 40.0, rng.uniform(0, 3)])))
            fz.preset('FF', 0x20, BB & 0xffffffff, record=True)
            fz.preset('FF', 0x24, BB >> 32, record=True)
            fz.preset('FF', 0x6484, rng.choice([0, 1, 2]))
            fz.preset('FF', 0x6488, rng.choice([0, 1]))
            fz.preset('FF', 0x648c, rng.choice([0, 0, 1]))
            fz.preset('FF', 0x64a8, rng.choice([0, 1, 2, 3, 4, 5, 5, 6]))
            fz.preset_f32('FF', 0x6490, rng.choice([0.0, 0.5, 11.5, 13.0, rng.uniform(0, 100), 99.5, 100.0]))
            fz.preset_f32('FF', 0x6494, rng.choice([rng.uniform(0, 520), 316.5, 300.0, 15.0]))
            fz.preset_f32('FF', 0x6498, rng.choice([0.0, 0.3, 0.995, 1.0, rng.uniform(0, 1)]))
            fz.preset_f32('FF', 0x649c, rng.choice([0.0, rng.uniform(0, 30), rng.uniform(0, 80)]))
            fz.preset_f32('FF', 0x64a0, rng.uniform(0, 2))
            fz.preset_f32('FF', 0x64a4, rng.choice([0.0, 450.0, rng.uniform(0, 500)]))
            fz.preset_f32('FF', 0x5c, rng.uniform(-20, 40))
            fz.preset_f32('FF', 0x754, rng.uniform(0, 100))
            for off in (0xc58, 0xc5c, 0xc60, 0xc64):
                fz.preset_f32('BB', off, rng.uniform(1.0, 60.0))
            for off in (0xc7c, 0xc80, 0xc84, 0xc88):
                fz.preset_f32('BB', off, rng.uniform(0, 3))
            run = (0x141245750, [FF])
            OBJ = FF
        elif KIND == 'engines':
            FF = case.region('FF', 0xc000)
            BB = case.region('BB', 0x7000)
            EN = case.region('EN', 0x68 * 4)
            RT = case.region('RT', 0x2cc * 4)
            SM = case.region('SM', 0x388 * 4)
            PT = case.region('PT', 0x3770 * 2)
            fz.region('NZ', 0x14578f1f0, 0x100000)
            case.region('SK', 0x10)
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.01, 0.05, 0.3, 1.0, 5.0, 40.0, rng.uniform(0, 3)])))
            def put(name, off, ptr, base=0):
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            put('FF', 0x20, BB)
            put('FF', 0x68b0, RT)
            put('FF', 0x68c8, SM)
            put('BB', 0x5ff8, EN)
            put('BB', 0x6010, PT)
            n = rng.choice([1, 2, 3, 4])
            fz.preset('BB', 0x91c, n)
            fz.preset('BB', 0xa70, rng.choice([0, 1]))
            fz.preset('FF', 0x6760, rng.choice([0, 0, 0, 1]))
            fz.preset('FF', 0x28, rng.choice([0, 0, 1]))
            for off in (0x6880, 0x6884, 0x6888):
                fz.preset('FF', off, rng.choice([0, 0, 1]))
            for off in (0x1a84, 0x1aa4, 0x1b38, 0x1ac4):
                fz.preset('BB', off, rng.choice([0, 1]))
            fz.preset('BB', 0xaf8, rng.choice([0, 1, 2, 3]))
            for k in range(4):
                fz.preset('EN', 0x68 * k, rng.choice([0, 1, 2, 3, 5, 6, 7]))
                fz.preset_f32('EN', 0x68 * k + 0x64, rng.choice([rng.uniform(0.02, 3), 0.5, 1.0]))
                fz.preset_f32('EN', 0x68 * k + 0x20, rng.choice([0.0, rng.uniform(0, 2)]))
                fz.preset('RT', 0x2cc * k + 0x74, rng.choice([0, 1]))
                fz.preset_f32('RT', 0x2cc * k + 0x44, rng.choice([0.05, 0.3, 0.5, 0.95, rng.uniform(0, 1)]))
            for off in (0xd4, 0xe0):
                pass
            for off, lo, hi in ((0x64, -20, 60), (0x70, 0.2, 1.3), (0x400, 0, 80), (0x41c, 0, 100)):
                fz.preset_f32('FF', off, rng.uniform(lo, hi))
            for off in (0x1a80, 0x1aa0, 0x1b34, 0x1ac0, 0x1b20, 0x1b24, 0x7b0, 0x7b4, 0x7d4, 0x7ac, 0x7d8, 0x1aac, 0x1ab0, 0xb64, 0xb68):
                fz.preset_f32('BB', off, rng.uniform(0.2, 150))
            fz.preset_f32('PT', 0x7a0, rng.choice([0.0, 10.0, 60.0]))
            run = (0x14125e4f0, [FF])
            OBJ = FF
        elif KIND == 'airspeed':
            speed = rng.choice([rng.uniform(-400, 400), rng.uniform(-100, 100), rng.uniform(250, 900), 0.0])
            mode = rng.choice([0, 1])
            fz.preset_f32('OBJ', 0x5c, rng.uniform(-50, 40))
            fz.preset_f32('OBJ', 0x68, rng.choice([101325.0, rng.uniform(20000, 105000)]))
            fz.preset_f32('OBJ', 0x6c, rng.uniform(0.2, 1.3))
            fz.preset_f32('OBJ', 0x70, rng.uniform(0.2, 1.3))
            fz.preset_f32('OBJ', 0x74, rng.choice([340.29, rng.uniform(290, 340), 0.0]))
            run = (0x1407cc570, [OBJ, 0, mode], [speed])
            arg = f'{speed}'
        elif KIND == 'variation':
            a = rng.choice([rng.uniform(-95, 95), rng.uniform(-90, 90), 0.0, 90.0, -90.0, 12.5, 5.0])
            b = rng.choice([rng.uniform(-185, 185), rng.uniform(-180, 180), 0.0, 175.0, 180.0, -180.0, 17.0])
            case.region('T', 0x3000)
            import math

            def floor5(x):
                t = x / 5.0 - 0.5
                t = t - 0.5 if t < 0 else t + 0.5
                return int(t) * 5

            lat = max(-90, min(90, floor5(a)))
            lon = floor5(b)
            lon = -180 if lon < -180 else min(lon, 175)
            row = max(1, min(36, int((90 - lat) / 5)))
            col = max(0, min(71, int((lon + 180) / 5)))
            for r in (row, row - 1):
                for c in (col, (col + 1) % 72):
                    fz.preset_f32('T', 4 + 4 * (r * 72 + c), rng.uniform(-30, 30))
            T = case.regions['T']
            run = (0x14076b5d0, [T], [a, b])
            arg = f'{a.hex() if False else ""}'
        else:
            fz.preset('OBJ', 0x7c9c, rng.choice([-1, 0, 1, 2]))
            fz.preset('OBJ', 0x7d34, rng.choice([0, 1, 1]))
            fz.preset('OBJ', 0x7b90, OBJ + 0x100)
            fz.preset('OBJ', 0x7b94, 0)
            fz.preset('OBJ', 0x20, B & 0xffffffff, record=True)
            fz.preset('OBJ', 0x24, B >> 32, record=True)
            fz.preset('B', 0x2270, rng.choice([0, 1, 2, 3]))
            run = (0x141185030, [OBJ])
        try:
            case.run(run[0], ints=run[1], floats=[0.0] + run[2] if KIND in ('variation', 'airspeed') else [], stack=[], max_instructions=100000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        result = case.emu.reg(UC_X86_REG_RAX) & 0xffffffff
        if KIND in ('start', 'engines'):
            OBJ = run[1][0]
        if KIND == 'airspeed':
            result = case.emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
            arg = '%08x %d' % (struct.unpack('<I', struct.pack('<f', run[2][0]))[0], run[1][2])
        if KIND == 'variation':
            bits = lambda v: struct.unpack('<I', struct.pack('<f', v))[0]
            result = case.emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
            arg = f'{bits(run[2][0]):08x} {bits(run[2][1]):08x}'
            OBJ = run[1][0]
        text = case.dump(f'{KIND} {OBJ:x} {arg} {result:x}')
        lines = text.split('\n')
        lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not sp - 0x800 <= int(t.split('=')[0], 16) < sp + 0x100)
        print('\n'.join(lines))
        done += 1


if __name__ == '__main__':
    main()
