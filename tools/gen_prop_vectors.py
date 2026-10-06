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
  Y xmm6 .. xmm15 (low words) r15
  X0..X3 off=word               initial words of the element records F[0x68e0 + b*0x18] (3 records of 0x2d8)
  L e retain ice x1 x2 x3 | out1 out2 out3 x4[4] x1bc stall   the get_el_force call (stubbed)
  Z id                          the global 0x142f2e3dc (the recorded id)
  Q w0 .. w19                   the pass record pushed by 0x141219d90 (stubbed)
  V x z y in1 in2 in3 out1 out2 out3   the wash adjustment of the second airflow call (0x14117d970)
  H phase (double hex) / G a b h  time-phase and terrain-probe answers
"""
import math
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator, STACK_TOP  # noqa: E402
from xp_fuzz import Fuzz  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_RAX, UC_X86_REG_RBP, UC_X86_REG_RIP, UC_X86_REG_RSP, UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2,
    UC_X86_REG_XMM3, UC_X86_REG_RDX, UC_X86_REG_R15)
from unicorn.x86_const import (UC_X86_REG_XMM6, UC_X86_REG_XMM7, UC_X86_REG_XMM8, UC_X86_REG_XMM9, UC_X86_REG_XMM10,  # noqa: E402
                               UC_X86_REG_XMM11, UC_X86_REG_XMM12, UC_X86_REG_XMM13, UC_X86_REG_XMM14, UC_X86_REG_XMM15)
XMM = {6: UC_X86_REG_XMM6, 7: UC_X86_REG_XMM7, 8: UC_X86_REG_XMM8, 9: UC_X86_REG_XMM9, 10: UC_X86_REG_XMM10,
       11: UC_X86_REG_XMM11, 12: UC_X86_REG_XMM12, 13: UC_X86_REG_XMM13, 14: UC_X86_REG_XMM14, 15: UC_X86_REG_XMM15}

ENTRY = 0x1411bd470
CHECKPOINTS = {1: 0x1411bda66, 2: 0x1411be62a, 3: 0x1411bf1c8, 4: 0x1411bfc91, 5: 0x1411c0a82, 6: 0x1411c1935, 7: 0x1411c2145}
NOISE_TABLE = 0x14578f1f0
EXE = sys.argv[1]
SEGMENT, TRIALS, SEED = int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4])


def noise_value(i):
    # a deterministic table: the original's runtime table is not recovered, so any fixed values do
    h = (i * 2654435761 + 12345) & 0xffffffff
    h ^= h >> 15
    h = (h * 2246822519) & 0xffffffff
    h ^= h >> 13
    return (h & 0xffffff) / 16777216.0


def noise_table():
    return struct.pack('<262144f', *[noise_value(i) for i in range(262144)])


def words(d):
    return ' '.join(f'{o:x}={w:08x}' for o, w in sorted(d.items()))


def main():
    emu = Emulator(EXE)
    fz = Fuzz(emu, EXE, [(0x141170000, 0x1412a0000), (0x140800000, 0x140a00000), (0x1406e0000, 0x140700000)], SEED)
    heap_mark = emu.heap_top
    emu.write(NOISE_TABLE, noise_table())
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

    def stub_phase(e):
        v = fz.rng.uniform(0, 1000)
        e.uc.reg_write(UC_X86_REG_XMM0, struct.unpack('<Q', struct.pack('<d', v))[0])
        log.append(f'H {struct.unpack("<Q", struct.pack("<d", v))[0]:016x}')

    def stub_terrain(e):
        rdx, r8, r9 = (e.uc.reg_read(r) for r in (UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9))
        a = [e.read_f32(rdx + 4 * i) for i in range(3)]
        b = [e.read_f32(r8 + 4 * i) for i in range(3)]
        h = e.read_f32(r9)
        if fz.rng.random() < 0.6:
            h = fz.rng.uniform(-30, 30)
            e.write_f32(r9, h)
        hx = lambda v: f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}'
        log.append('G ' + ' '.join(hx(v) for v in a + b) + ' ' + hx(h))

    def stub_wash(e):
        rsp = e.uc.reg_read(UC_X86_REG_RSP)
        out1 = e.uc.reg_read(UC_X86_REG_R8)
        out2 = e.read_u64(rsp + 0x28)
        out3 = e.read_u64(rsp + 0x38)
        y = e.read_f32(rsp + 0x30)
        x = struct.unpack('<f', struct.pack('<I', e.uc.reg_read(UC_X86_REG_XMM1) & 0xffffffff))[0]
        z = struct.unpack('<f', struct.pack('<I', e.uc.reg_read(UC_X86_REG_XMM3) & 0xffffffff))[0]
        old = [e.read_f32(p) for p in (out1, out2, out3)]
        new = [v + fz.rng.uniform(-0.3, 0.3) for v in old]
        for p, v in zip((out1, out2, out3), new):
            e.write_f32(p, v)
        hx = lambda v: f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}'
        newf = [struct.unpack('<f', struct.pack('<f', v))[0] for v in new]
        log.append('V ' + ' '.join(hx(v) for v in (x, z, y, *old, *newf)))

    def stub_element(e):
        rsp = e.uc.reg_read(UC_X86_REG_RSP)
        idx = e.read_u32(rsp + 0x28)
        ice = e.read_f32(rsp + 0x30)
        outs = [e.read_u64(rsp + 0x38 + 8 * i) for i in range(3)]
        extras = [e.read_f32(rsp + 0x50 + 8 * i) for i in range(3)]
        xptr = e.uc.reg_read(UC_X86_REG_R8)
        retain = e.uc.reg_read(UC_X86_REG_R9) & 0xffffffff
        hx = lambda v: f'{struct.unpack("<I", struct.pack("<f", v))[0]:08x}'
        vals = [fz.rng.uniform(-3, 3) for _ in range(3)]
        for p, v in zip(outs, vals):
            e.write_f32(p, v)
        x4 = [fz.rng.uniform(-2, 2) for _ in range(4)]
        for off, v in zip((0xf4, 0x11c, 0x144, 0x16c), x4):
            e.write_f32(xptr + off + 4 * idx, v)
            fz.note_write(xptr + off + 4 * idx)
        x1bc = fz.rng.uniform(-2, 2)
        e.write_f32(xptr + 0x1bc + 4 * idx, x1bc)
        fz.note_write(xptr + 0x1bc + 4 * idx)
        stall = fz.rng.randrange(2)
        e.write_u32(xptr + 0x1e4 + 4 * idx, stall)
        fz.note_write(xptr + 0x1e4 + 4 * idx)
        log.append('L ' + ' '.join([str(idx), str(retain), hx(ice)] + [hx(v) for v in extras]) + ' | '
                   + ' '.join(hx(struct.unpack('<f', struct.pack('<f', v))[0]) for v in vals + x4 + [x1bc]) + f' {stall}')

    def stub_record(e):
        words = [e.read_u32(e.uc.reg_read(UC_X86_REG_RDX) + 4 * i) for i in range(20)]
        log.append('Q ' + ' '.join(f'{w:08x}' for w in words))

    emu.stubs[0x141219d90] = stub_record
    emu.stubs[0x1411b9840] = stub_element
    emu.stubs[0x14117d970] = stub_wash
    emu.stubs[0x140c81ea0] = stub_phase
    emu.stubs[0x14195f4b0] = stub_terrain
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
        X = [emu.alloc(0x2d8 * 3) for _ in range(4)]
        P = emu.alloc(0x3770 * 3)
        R = emu.alloc(0x2000)
        for name, addr, size in [('F', F, 0x44000), ('B', B, 0x7000), ('E', E, 0x68 * 3), ('P', P, 0x3770 * 3),
                                 ('R', R, 0x2000)] + [(f'X{i}', X[i], 0x2d8 * 3) for i in range(4)]:
            fz.region(name, addr, size)
            emu.write(addr, bytes(size))
        n = fz.rng.randrange(3)
        fz.preset('F', 0x20, B & 0xffffffff, record=False)
        fz.preset('F', 0x24, B >> 32, record=False)
        fz.preset('B', 0x5ff8, E & 0xffffffff, record=False)
        fz.preset('B', 0x5ffc, E >> 32, record=False)
        fz.preset('B', 0x6010, P & 0xffffffff, record=False)
        fz.preset('B', 0x6014, P >> 32, record=False)
        for i in range(4):
            fz.preset('F', 0x68e0 + 0x18 * i, X[i] & 0xffffffff, record=False)
            fz.preset('F', 0x68e4 + 0x18 * i, X[i] >> 32, record=False)
        fz.preset_f32('P', n * 0x3770 + 0x3730, fz.rng.uniform(0.3, 3.0))
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0, record=False)
        base_p = n * 0x3770
        fz.preset('P', base_p + 0x8c, fz.rng.randrange(2, 5))
        fz.preset('P', base_p + 0, fz.rng.choice([0, 1, 3, 6, 7, 2]))
        f28 = fz.rng.randrange(1, 4)
        fz.preset('F', 0x28, f28)
        recording = f28 if fz.rng.random() < 0.5 else 0
        emu.write_u32(0x142f2e3dc, recording)
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
        out = [f'T {n} {early} {flag["v"]}', f'Z {recording}']
        for name in ('F', 'B', 'E', 'P', 'R', 'X0', 'X1', 'X2', 'X3'):
            out.append(f'{name} {words(init[name])}')
        out.extend(log)
        wr = fz.written()
        out.append('O R ' + words(wr['R']))
        out.append('O F ' + words(wr['F']))
        for i in range(4):
            out.append(f'O X{i} ' + words(wr[f'X{i}']))
        if not early:
            slots = {}
            for off in list(range(-0x100, 0x260, 4)) + list(range(0x8d0, 0x8f8, 4)):
                w = emu.read_u32(rbp + off)
                if w:
                    slots[off] = w
            out.append('S ' + ' '.join(f'{o}={w:08x}' for o, w in sorted(slots.items())))
            regs = [UC_X86_REG_XMM0 + i for i in range(6, 16)] if False else [XMM[i] for i in range(6, 16)]
            out.append('Y ' + ' '.join(f'{emu.uc.reg_read(r) & 0xffffffff:08x}' for r in regs) + ' '
                       + f'{emu.uc.reg_read(UC_X86_REG_R15) & 0xffffffff}')
        print('\n'.join(out))
        done += 1


if __name__ == '__main__':
    main()
