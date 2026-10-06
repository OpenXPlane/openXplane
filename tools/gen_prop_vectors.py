#!/usr/bin/env python3
"""Checkpoint vectors for prop::prop_force (the original 0x1411bd470) on lazily random-filled objects.

Per trial: the initial words of F, B, E (engine record), P (part record) and R (output record) that the
function read, the answers of the stubbed callees in call order, and at the checkpoint the frame words and the
xmm registers. Usage:

    python3 tools/gen_prop_vectors.py Xplane12/X-Plane.exe SEGMENT TRIALS SEED > crates/xp-app/tests/data/prop_N.txt

Lines:
  T n early flag      (flag: the engine flag 0x1417f12c0, constant for the trial)
  F|B|E|P|R off=word ...        initial words (hex)
  W x y z (double hex) w1 w2 w3 wind sampler: position it was asked for and the three floats it stored
  D dt (double hex)             the time step answers, in call order
  O R|F off=word ...            final words of the written offsets of R / F
  S off=word ...               frame words (signed rbp offsets, decimal) in the checked ranges
  X xmm6 xmm7 xmm8 xmm9 xmm11 xmm13 xmm15 r15
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator, STACK_TOP  # noqa: E402
from xp_fuzz import Fuzz  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_RAX, UC_X86_REG_RBP, UC_X86_REG_RIP, UC_X86_REG_RSP, UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2,
    UC_X86_REG_XMM3, UC_X86_REG_RDX, UC_X86_REG_XMM6, UC_X86_REG_XMM7, UC_X86_REG_XMM8, UC_X86_REG_XMM9,
    UC_X86_REG_XMM11, UC_X86_REG_XMM13, UC_X86_REG_XMM15, UC_X86_REG_R15)

ENTRY = 0x1411bd470
CHECKPOINTS = {1: 0x1411bda66, 2: 0x1411be62a}
EXE = sys.argv[1]
SEGMENT, TRIALS, SEED = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])


def words(d):
    return ' '.join(f'{o:x}={w:08x}' for o, w in sorted(d.items()))


def main():
    emu = Emulator(EXE)
    fz = Fuzz(emu, EXE, [(0x141170000, 0x1412a0000), (0x140800000, 0x140a00000), (0x1406e0000, 0x140700000)], SEED)
    heap_mark = emu.heap_top
    log = []
    flag = {'v': 0}

    def stub_flag(e):
        e.uc.reg_write(UC_X86_REG_RAX, flag['v'])

    def stub_sanitize(e):
        ptr = e.uc.reg_read(UC_X86_REG_RDX)
        if not math.isfinite(e.read_f32(ptr)):
            e.write_f32(ptr, 0.0)

    def stub_wind(e):
        rsp = e.uc.reg_read(UC_X86_REG_RSP)
        pointers = [e.read_u64(rsp + 0x28 + 8 * i) for i in range(3)]
        seen = [e.uc.reg_read(r) & 0xffffffffffffffff for r in (UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3)]
        wind = [fz.rng.uniform(-30, 30) for _ in range(3)]
        for p, v in zip(pointers, wind):
            e.write_f32(p, v)
        log.append('W ' + ' '.join(f'{s:016x}' for s in seen) + ' ' + ' '.join(
            f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}' for v in wind))

    def stub_time(e):
        dt = fz.rng.uniform(0.004, 0.06)
        e.uc.reg_write(UC_X86_REG_XMM0, struct.unpack('<Q', struct.pack('<d', dt))[0])
        log.append(f'D {struct.unpack("<Q", struct.pack("<d", dt))[0]:016x}')

    emu.stubs[0x140c448c0] = stub_time
    emu.stubs[0x1417f12c0] = stub_flag
    emu.stubs[0x141176330] = stub_sanitize
    emu.stubs[0x141ba80a0] = stub_wind
    print('# prop_force checkpoint vectors (tools/gen_prop_vectors.py), segment', SEGMENT)
    done = 0
    attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        emu.heap_top = heap_mark
        fz.policy = {}
        F = emu.alloc(0x44000)
        B = emu.alloc(0x7000)
        E = emu.alloc(0x68 * 3)
        P = emu.alloc(0x3770 * 3)
        R = emu.alloc(0x2000)
        for name, addr, size in (('F', F, 0x44000), ('B', B, 0x7000), ('E', E, 0x68 * 3), ('P', P, 0x3770 * 3),
                                 ('R', R, 0x2000)):
            fz.region(name, addr, size)
            emu.write(addr, bytes(size))
        n = fz.rng.randrange(3)
        fz.preset('F', 0x20, B & 0xffffffff, record=False)
        fz.preset('F', 0x24, B >> 32, record=False)
        fz.preset('B', 0x5ff8, E & 0xffffffff, record=False)
        fz.preset('B', 0x5ffc, E >> 32, record=False)
        fz.preset('B', 0x6010, P & 0xffffffff, record=False)
        fz.preset('B', 0x6014, P >> 32, record=False)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0, record=False)
        base_p = n * 0x3770
        fz.preset('P', base_p + 0x8c, fz.rng.randrange(2, 5))
        fz.preset('P', base_p + 0, fz.rng.choice([0, 1, 3, 6, 7, 2]))
        fz.preset_f32('R', 0x1c, fz.rng.uniform(0, 1))
        fz.preset_f32('P', base_p + 0x10, fz.rng.choice([2.0, 2.0, 1.0, fz.rng.uniform(0, 3)]))
        # fractions near the thresholds of the first block
        log.clear()
        flag['v'] = fz.rng.randrange(2)
        fz.preset('B', 0xa78, fz.rng.choice([0, 0, 0, 1]))
        until = CHECKPOINTS[SEGMENT]
        # the engine index selects its own record: the function indexes with n
        emu.uc.reg_write(UC_X86_REG_RAX, 0)
        try:
            emu.call(ENTRY, ints=[R, F, n], until=until, max_instructions=400000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        rip = emu.uc.reg_read(UC_X86_REG_RIP)
        early = 0 if rip == until else 1
        entry_rsp = ((STACK_TOP - 0x1000 - 0x20) & ~0xf) - 8
        rbp = entry_rsp - 0x8c8
        init = fz.initial()
        out = [f'T {n} {early} {flag["v"]}']
        for name in ('F', 'B', 'E', 'P', 'R'):
            out.append(f'{name} {words(init[name])}')
        out.extend(log)
        wr = fz.written()
        out.append('O R ' + words(wr['R']))
        out.append('O F ' + words(wr['F']))
        if not early:
            slots = {}
            for off in list(range(-0x100, 0x260, 4)) + list(range(0x8d0, 0x8f8, 4)):
                w = emu.read_u32(rbp + off)
                if w:
                    slots[off] = w
            out.append('S ' + ' '.join(f'{o}={w:08x}' for o, w in sorted(slots.items())))
            regs = [UC_X86_REG_XMM6, UC_X86_REG_XMM7, UC_X86_REG_XMM8, UC_X86_REG_XMM9, UC_X86_REG_XMM11,
                    UC_X86_REG_XMM13, UC_X86_REG_XMM15]
            out.append('X ' + ' '.join(f'{emu.uc.reg_read(r) & 0xffffffff:08x}' for r in regs) + ' '
                       + f'{emu.uc.reg_read(UC_X86_REG_R15) & 0xffffffff}')
        print('\n'.join(out))
        done += 1


if __name__ == '__main__':
    main()
