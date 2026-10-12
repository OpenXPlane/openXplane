#!/usr/bin/env python3
"""Vectors for controls::engine_controls (the original 0x141260090) up to a checkpoint.

    python3 tools/gen_controls_vectors.py Xplane12/X-Plane.exe STAGE TRIALS SEED > crates/xp-app/tests/data/controls_N.txt
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402

EXE, STAGE, TRIALS, SEED = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])
UNTIL = {1: 0x1412605eb, 2: 0x1412609bb, 3: 0x141262177, 4: None}
FUNC = 0x141260090


def bool_stub(call, rng):
    call.ret_int(rng.randrange(2))


def void_stub(call, rng):
    pass


def rng_double(case, lo, hi):
    return struct.unpack('<Q', struct.pack('<d', case.rng.uniform(lo, hi)))[0]


def resize_stub(case):
    def stub(call, rng):
        vec, count = call.ints[0], call.ints[1] & 0xffffffff
        base = case.region(f'V{len(case.regions)}', max(16, 4 * count + 16))
        for off, v in ((0, base), (8, base + 4 * count), (16, base + 4 * count)):
            call.put(vec + off, v & 0xffffffff)
            call.put(vec + off + 4, v >> 32)
    return stub


def main():
    case = VmCase(EXE, SEED, [(0x141190000, 0x1412a0000), (0x140800000, 0x140900000), (0x141176000, 0x141177100),
                              (0x1406e0000, 0x1406f0000), (0x141230000, 0x141240000), (0x141a60000, 0x141a70000)])
    for addr in (0x1407ace10, 0x1417f12c0):
        case.stub(addr, bool_stub)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
    for addr in (0x1411bd470, 0x141190ed0, 0x1406307a0, 0x14117c380):
        case.stub(addr, void_stub)
    # the leaf callees of the engine kinds (replayed by the port)
    case.stub(0x141ba6750, lambda call, rng: call.ret_f32(rng.uniform(-40.0, 45.0)))
    case.stub(0x141ba64e0, lambda call, rng: call.ret_f32(rng.uniform(0.3, 1.4)))
    case.stub(0x1408be0f0, lambda call, rng: call.ret_f32(rng.uniform(-500.0, 12000.0)))
    case.stub(0x1410c9c60, lambda call, rng: call.ret_int(0x6f0000100000))
    case.stub(0x14067b2f0, lambda call, rng: call.ret_f32(rng.uniform(0.0, 1.0)))
    case.stub(0x140984e50, lambda call, rng: call.ret_f32(rng.uniform(-1.0, 1.0)))
    case.stub(0x1406fd4d0, resize_stub(case))
    print('# controls vectors (tools/gen_controls_vectors.py), stage', STAGE)
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        case.reset()
        F = case.region('F', 0xc000)
        B = case.region('B', 0x7000)
        E = case.region('E', 0x68 * 4)
        P = case.region('P', 0x3770 * 4)
        M = case.region('M', 0x2cc * 4)
        N = case.region('N', 0x388 * 4)
        D = case.region('D', 0x400)
        fz = case.fz
        for base, off, ptr in ((F, 0x20, B), (B, 0x5ff8, E), (B, 0x6010, P), (B, 0x61f8, D), (F, 0x68b0, M), (F, 0x68c8, N)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        fz.preset('F', 0xbce0, F & 0xffffffff)
        fz.preset('F', 0xbce4, F >> 32)
        fz.preset('F', 0xbce8, B & 0xffffffff)
        fz.preset('F', 0xbcec, B >> 32)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        fz.preset('F', 0x6760, case.rng.choice([0, 0, 0, 1]))
        fz.preset('F', 0x6764, case.rng.choice([0, 0, 1]))
        for off in (0x91c, 0x920):
            fz.preset('B', off, case.rng.randrange(1, 4))
        for e in range(4):
            fz.preset('E', 0x68 * e, case.rng.choice([0, 1, 2, 3, 4, 5, 6, 7]))
            fz.preset('P', 0x3770 * e, case.rng.choice([0, 3, 5, 6, 6]))
        for e in range(4):
            fz.preset('B', 0xb7c + 4 * e, case.rng.randrange(0, 3))
            fz.preset('B', 0xd38 + 4 * e, case.rng.randrange(0, 3))
        for j in range(4):
            fz.preset('B', 0xbbc + 4 * j, case.rng.randrange(0, 3))
        fz.preset('B', 0x990, case.rng.choice([0, 0, 1, 1, 2]))
        fz.preset('B', 0xa9c, case.rng.choice([0, 2, 3, 3, 2]))
        fz.preset('B', 0xc24, case.rng.choice([0, 1]))
        fz.preset('B', 0xab0, case.rng.choice([0, 1]))
        for e in range(4):
            fz.preset('M', 0x2cc * e + 0x298, case.rng.choice([0, 1, 2, 3]))
            fz.preset('M', 0x2cc * e + 0x228, case.rng.choice([0, 1]))
            fz.preset('M', 0x2cc * e + 0x70, case.rng.choice([0, 1, 1]))
            fz.preset('M', 0x2cc * e + 0x74, case.rng.randrange(0, 3))
            fz.preset('M', 0x2cc * e + 0x48, case.rng.randrange(0, 5))
        fz.preset('B', 0xb78, case.rng.randrange(0, 3))
        fz.preset('B', 0xc90, case.rng.randrange(0, 3))
        fz.preset('B', 0xc9c, case.rng.randrange(0, 3))
        fz.preset('B', 0xca0, case.rng.randrange(0, 3))
        extra = {}
        for address, value in ((0x142f01918, rng_double(case, -100.0, 100.0)), (0x142f01910, rng_double(case, 0.0, 1000.0))):
            case.emu.write_u32(address, value & 0xffffffff)
            case.emu.write_u32(address + 4, value >> 32)
            extra[address] = value & 0xffffffff
            extra[address + 4] = value >> 32
        for i in range(0x803 * 2):
            case.emu.write_u32(0x14612bd90 + 4 * i, struct.unpack('<I', struct.pack('<f', 0.3 + ((i * 37) % 101) / 100.0))[0])
        import os
        if os.environ.get('XP_HOOK'):
            from unicorn import UC_HOOK_CODE
            from unicorn.x86_const import (UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3, UC_X86_REG_RSP, UC_X86_REG_RSI)
            def ch(uc, address, size, user):
                if address == 0x141199418:
                    f = lambda r: struct.unpack('<f', struct.pack('<I', uc.reg_read(r) & 0xffffffff))[0]
                    from unicorn.x86_const import UC_X86_REG_XMM6, UC_X86_REG_XMM15
                    sys.stderr.write(f'MAPS a={f(UC_X86_REG_XMM6)} b={f(UC_X86_REG_XMM2)} drag={f(UC_X86_REG_XMM15)}\n')
                if address == 0x141199792:
                    f = lambda r: struct.unpack('<f', struct.pack('<I', uc.reg_read(r) & 0xffffffff))[0]
                    rsp = uc.reg_read(UC_X86_REG_RSP)
                    st = struct.unpack('<f', bytes(uc.mem_read(rsp + 0x20, 4)))[0]
                    sys.stderr.write(f'LINEAR x0={f(UC_X86_REG_XMM0)} x1={f(UC_X86_REG_XMM1)} x2={f(UC_X86_REG_XMM2)} x3={f(UC_X86_REG_XMM3)} st={st} rsi={uc.reg_read(UC_X86_REG_RSI):#x}\n')
            case.emu.uc.hook_add(UC_HOOK_CODE, ch)
        if os.environ.get('XP_WATCH'):
            from unicorn import UC_HOOK_MEM_WRITE
            from unicorn.x86_const import UC_X86_REG_RIP
            def wh(uc, access, address, size, value, user):
                if F + 0x2b0 <= address < F + 0x2d0:
                    sys.stderr.write(f'write F+{address - F:#x} = {value:#x} at {uc.reg_read(UC_X86_REG_RIP):#x}\n')
            case.emu.uc.hook_add(UC_HOOK_MEM_WRITE, wh)
        try:
            case.run(FUNC, ints=[F], until=UNTIL[STAGE], max_instructions=600000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        print(case.dump(f'{F:x} {entry_rsp(0):x}', extra_words=extra))
        done += 1


if __name__ == '__main__':
    main()
