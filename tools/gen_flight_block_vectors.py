#!/usr/bin/env python3
"""Vectors for blocks of update_flight (0x1412656b0), run in the emulator from the block's start address to its end
address on random objects at absolute addresses (the registers the block reads are set by the case):

    python3 tools/gen_flight_block_vectors.py Xplane12/X-Plane.exe BLOCK TRIALS SEED > crates/xp-app/tests/data/flight_BLOCK.txt

BLOCK: aspect (0x141265f7d..0x14126644a, the aspect-ratio factors of the wings), thrust (0x14126644a..0x141266b52,
the engine controls call, rocket and pitch-tilt thrust, blown flaps), element (0x141266b52..0x141267978, the
element loop: airflow, element force and aerodynamic force of every wing element; the stack frame is at `rbp`
of the header).
"""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from xp_vmcase import VmCase, entry_rsp  # noqa: E402
from unicorn.x86_const import (UC_X86_REG_R12, UC_X86_REG_R13, UC_X86_REG_R14, UC_X86_REG_R15, UC_X86_REG_RBP, UC_X86_REG_RSI, UC_X86_REG_XMM8, UC_X86_REG_XMM11,  # noqa: E402
                               UC_X86_REG_XMM12, UC_X86_REG_XMM13, UC_X86_REG_XMM14, UC_X86_REG_XMM15)

EXE, BLOCK, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
BLOCKS = {'aspect': (0x141265f7d, 0x14126644a), 'thrust': (0x14126644a, 0x141266b52), 'element': (0x141266b52, 0x141267978),
          'body': (0x141267978, 0x1412686a9), 'parts': (0x141269920, 0x14126a791), 'motion': (0x1412709ff, 0x141271fa2), 'integrate': (0x141271fa2, 0x14127223b)}
SIM_TIME = 0x142f01918
RANGES = [(0x141265000, 0x141275000), (0x1411d0000, 0x1411e0000), (0x141210000, 0x141220000), (0x141290000, 0x1412a0000),
          (0x140860000, 0x140870000), (0x1406e0000, 0x1406f0000), (0x1411a0000, 0x1411d0000), (0x140910000, 0x140911000),
          (0x141220000, 0x141230000), (0x140810000, 0x140820000), (0x1407d0000, 0x1407e0000), (0x1408b0000, 0x1408c0000),
          (0x140900000, 0x140910000), (0x141a50000, 0x141a60000), (0x1411b0000, 0x1411c0000), (0x141180000, 0x141190000),
          (0x1411e0000, 0x1411f0000), (0x141a60000, 0x141a70000), (0x140620000, 0x140630000), (0x140f20000, 0x140f30000), (0x141170000, 0x141180000),
          (0x1407ce000, 0x1407cf000), (0x140880000, 0x14088b000), (0x140f5f000, 0x140f60000), (0x1411d9000, 0x1411da000)]
RECORD_VECTOR = 0x146125768
RECORDING_ID = 0x142f2e3dc


def main():
    start, end = BLOCKS[BLOCK]
    case = VmCase(EXE, SEED, RANGES)
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.randrange(2) if rng.random() < 0.2 else 0))
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.randrange(2)))
    case.stub(0x141260090, lambda call, rng: None)
    for diagnostic in (0x1405dcad0, 0x141a67610):  # the log message of a repaired value
        case.stub(diagnostic, lambda call, rng: None)

    def airflow(call, rng):
        call.put_f32(call.ints[2], rng.uniform(-50, 50))
        call.put_f32(call.stack[0], rng.uniform(-50, 50))
        call.put_f32(call.stack[2], rng.uniform(-50, 50))

    def element_force(call, rng):
        for pointer in (call.stack[2], call.stack[3], call.stack[4]):
            call.put_f32(pointer, rng.uniform(-1, 1))

    case.stub(0x14121b580, airflow)
    case.stub(0x141260090, lambda call, rng: None)
    case.stub(0x1411b9840, element_force)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
    if BLOCK == 'integrate':
        def segment(call, rng):
            for pointer in (call.stack[0], call.stack[1], call.stack[2]):
                call.put_f32(pointer, rng.uniform(-3, 3))

        def init(call, rng):
            call.put(call.ints[0] - 0x42e40 + 0x42f88, rng.choice([0, 1]))

        def probe(call, rng):
            if rng.random() < 0.6:
                call.put_f32(call.ints[3], rng.uniform(-100, 3000))
            call.ret_int(1 if rng.random() < 0.4 else 0)

        case.stub(0x141963d30, segment)
        case.stub(0x141962b00, init)
        case.stub(0x14195f4b0, probe)
        case.stub(0x141962750, lambda call, rng: call.ret_f32(rng.uniform(-50, 3000)))
        case.stub(0x140f5c540, lambda call, rng: call.ret_int(rng.randrange(2)))
        case.stub(0x140f42620, lambda call, rng: call.ret_int(rng.randrange(1 << 40)))
        case.stub(0x140f3c2d0, lambda call, rng: None)
    print('# update_flight block vectors', BLOCK, hex(start), hex(end))
    rng = case.rng
    done = attempts = 0
    while done < TRIALS and attempts < TRIALS * 20:
        attempts += 1
        case.reset()
        F = case.region('F', 0x44000)
        B = case.region('B', 0x7000)
        W = case.region('W', 0x36c8 * 48)
        X = case.region('X', 0x2d8 * 48)
        fz = case.fz
        for base, off, ptr in ((F, 0x20, B), (B, 0x6028, W), (F, 0x6940, X)):
            name = {F: 'F', B: 'B'}[base]
            fz.preset(name, off, ptr & 0xffffffff, record=True)
            fz.preset(name, off + 4, ptr >> 32, record=True)
        for off in (0xbcc8, 0xbcd0):
            fz.preset('F', off, 0)
        for i in range(48):
            fz.preset('W', 0x36c8 * i + 4, rng.randrange(1, 7) if BLOCK == 'element' else rng.randrange(1, 11))
            fz.preset('W', 0x36c8 * i + 0x678, rng.choice([0, 1, 1, 1]) if BLOCK != 'element' or i < 3 else 0)
            fz.preset_f32('W', 0x36c8 * i + 0xf4, rng.uniform(-80.0, 80.0))
        extra = {}
        if BLOCK == 'thrust':
            E = case.region('E', 0x68 * 4)
            M = case.region('M', 0x2cc * 4)
            for base, off, ptr in ((B, 0x5ff8, E), (F, 0x68b0, M)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('B', 0x91c, rng.randrange(0, 4))
            for e in range(4):
                fz.preset('E', 0x68 * e, rng.choice([0, 1, 2, 5, 6, 6, 7]))
                fz.preset('M', 0x2cc * e + 0x298, rng.choice([0, 1, 2, 3]))
            fz.preset('F', 0x6514, rng.choice([0, 1, 1]))
            fz.preset('F', 0x57c, rng.choice([0, 1, 1]))
            fz.preset('F', 0x6518, rng.choice([0x264, 0x265, 0x266, 0x267, 0x268, 0x269, 0x26a, 0x264]))
            t = struct.unpack('<Q', struct.pack('<d', rng.uniform(0.0, 100.0)))[0]
            case.emu.write_u32(SIM_TIME, t & 0xffffffff)
            case.emu.write_u32(SIM_TIME + 4, t >> 32)
            extra = {SIM_TIME: t & 0xffffffff, SIM_TIME + 4: t >> 32}
            fz.preset_f32('F', 0x6538, rng.uniform(0.0, 120.0))
            fz.preset_f32('B', 0x21e8, rng.choice([-1.0, 1.0, 2.5]))
            fz.preset_f32('B', 0x21e4, rng.uniform(0.0, 1.0))
            fz.preset_f32('F', 0x184, rng.uniform(0.0, 1.5))
        if BLOCK == 'body':
            T = case.region('T', 0x34c8 * 39)
            E = case.region('E', 0x68 * 4)
            for base, off, ptr in ((B, 0x6040, T), (B, 0x5ff8, E)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            for e in range(4):
                fz.preset('E', 0x68 * e, rng.choice([0, 1, 5, 6, 7]))
            for k in range(39):
                base = 0x34c8 * k
                fz.preset('T', base + 0x588, 1 if k < 3 else 0)
                fz.preset('T', base + 0x54, 0)
                fz.preset('T', base + 0x5f0, rng.randrange(0, 0x30))
                fz.preset('T', base + 0x68, rng.randrange(-1, 4))
                if k < 3:
                    for off in (0x10, 0x14, 0x18):
                        fz.preset_f32('T', base + off, rng.uniform(0.05, 20.0))
                    for off in (0x58, 0x5c, 0x60):
                        fz.preset_f32('T', base + off, rng.uniform(-5.0, 0.0))
                    for off in (0x64, 0x68, 0x6c):
                        fz.preset_f32('T', base + off, rng.uniform(0.0, 5.0))
                    fz.preset('T', base + 0x654, rng.randrange(1, 4))
                    fz.preset('T', base + 0x658, rng.randrange(2, 19))
                    for row in range(4):
                        for col in range(18):
                            for axis in range(3):
                                fz.preset_f32('T', base + 0x65c + row * 0xd8 + col * 12 + 4 * axis, rng.uniform(-3.0, 3.0))
            for off in (0x6c, 0x74):
                fz.preset_f32('F', off, rng.uniform(0.5, 400.0))
            fz.preset('F', 0x28, 7)
        if BLOCK == 'parts':
            P = case.region('P', 0x3770 * 4)
            M = case.region('M', 0x2cc * 4)
            N = case.region('N', 0x388 * 4)
            for base, off, ptr in ((B, 0x6010, P), (F, 0x68b0, M), (F, 0x68c8, N)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('B', 0x91c, rng.randrange(0, 5))
            fz.preset('B', 0xc08, rng.choice([0, 1, 1]))
            fz.preset_f32('B', 0xc0c, rng.choice([0.2, 0.7, 3.0, 12.0, rng.uniform(0.0, 20.0)]))
            fz.preset_f32('B', 0xc10, rng.choice([-1.0, 0.5, 2.0, rng.uniform(0.0, 3.0)]))
            fz.preset_f32('B', 0xc14, rng.choice([-1.0, 0.5, 2.0, rng.uniform(0.0, 3.0)]))
            for off in (0x29c, 0x2a8, 0x2b4):
                fz.preset_f32('F', off, rng.choice([float('nan'), float('inf'), -float('inf')] + [rng.uniform(-60, 60)] * 12))
            fz.preset_f32('F', 0x6c, rng.uniform(0.2, 1.3))
            for k in range(4):
                fz.preset_f32('M', 0x2cc * k + 0x44, rng.uniform(0.0, 5.0))
                fz.preset_f32('N', 0x388 * k + 0x84, rng.choice([float('nan'), rng.uniform(-3, 3), rng.uniform(-3, 3)]))
                for off in (0x790, 0x794, 0x798):
                    fz.preset_f32('P', 0x3770 * k + off, rng.uniform(-3.0, 3.0))
                for off in (0x79c, 0x7a0, 0x7a4):
                    fz.preset_f32('P', 0x3770 * k + off, rng.uniform(-90.0, 90.0))
            case.emu.uc.reg_write(UC_X86_REG_RSI, 0)
            case.emu.uc.reg_write(UC_X86_REG_XMM8, struct.unpack('<Q', struct.pack('<d', 0.5))[0])
        if BLOCK == 'motion':
            TB = case.region('TB', 36 * 8)
            for i in range(8):
                fz.preset('TB', 36 * i + 8, rng.choice([0, 1, 0, 2]))
            table = 0x14611ac80
            case.emu.write_u32(table + 8, TB & 0xffffffff)
            case.emu.write_u32(table + 12, TB >> 32)
            extra = {table + 8: TB & 0xffffffff, table + 12: TB >> 32}
            for address in (0x145899fd0, 0x145899fd4, 0x145899fd8, 0x145899fdc, 0x145899fe0, 0x145899fe4):
                value = rng.choice([0, 1, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff]) if address >= 0x145899fe0 else rng.choice([0, 0, 0, 1])
                case.emu.write_u32(address, value)
                extra[address] = value
            case.emu.uc.reg_write(UC_X86_REG_R14, 1)
            fz.preset_f32('F', 0x78, rng.uniform(8.0, 10.0))
            fz.preset_f32('F', 0x288, rng.uniform(100.0, 3000.0))
            for off in (0x2f4, 0x2e0, 0x2cc, 0x30c, 0x324, 0x33c):
                fz.preset_f32('F', off, rng.choice([float('nan')] + [rng.uniform(-5000, 5000)] * 24 + [rng.uniform(-400000, 400000)] * 4))
            for off in (0x378, 0x380, 0x388):
                fz.preset_f64('F', off, rng.uniform(-5000.0, 5000.0))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1.0, 1.0))
            for off in (0x3cc, 0x3d0, 0x3d4):
                fz.preset_f32('F', off, rng.uniform(-3.0, 3.0))
            fz.preset_f32('F', 0x538, rng.choice([0.0, 5.0, 20.0]))
            fz.preset_f32('F', 0x53c, rng.uniform(-1.0, 1.0))
            fz.preset('F', 0x24c, rng.choice([0, 1, 1, 1, 1]))
            fz.preset('F', 0x28, rng.choice([0, 0, 7, 12345]))
            fz.preset('F', 0x42f84, rng.randrange(0, 8))
            fz.preset_f32('F', 0x42f5c, rng.choice([-10.0, 100.0, 1000.0, 20000.0]))
            for off in (0x368, 0x36c, 0x370):
                fz.preset_f32('F', off, rng.choice([rng.uniform(-60, 60), rng.uniform(-300, 300), rng.uniform(-90, 90), rng.uniform(-110, 110)]))
        if BLOCK == 'integrate':
            LV = case.region('LV', 16 * 6)
            count = rng.randrange(0, 5)
            for base, off, ptr in ((F, 0x69b8, LV), (F, 0x69c0, LV + 16 * count)):
                fz.preset('F', off, ptr & 0xffffffff, record=True)
                fz.preset('F', off + 4, ptr >> 32, record=True)
            empty0, empty1 = rng.randrange(1 << 32), rng.randrange(1 << 60)
            for address, value in ((0x142f03778, empty0), (0x142f03780, empty1 & 0xffffffff), (0x142f03784, empty1 >> 32)):
                case.emu.write_u32(address, value)
                extra[address] = value
            for k in range(count):
                mode = rng.choice([0, 1, 2, 3, 3])
                first = empty0 if mode in (0, 1) else rng.randrange(1 << 32)
                second = empty1 if mode in (0, 2) else rng.randrange(1 << 60)
                fz.preset('LV', 16 * k, first)
                fz.preset('LV', 16 * k + 4, rng.randrange(1 << 32))
                fz.preset('LV', 16 * k + 8, second & 0xffffffff)
                fz.preset('LV', 16 * k + 12, second >> 32)
            scale = struct.unpack('<Q', struct.pack('<d', rng.choice([1.0, 0.5, 2.0, 0.0])))[0]
            case.emu.write_u32(0x142f01898, scale & 0xffffffff)
            case.emu.write_u32(0x142f0189c, scale >> 32)
            extra[0x142f01898] = scale & 0xffffffff
            extra[0x142f0189c] = scale >> 32
            for off in (0x368, 0x36c, 0x370, 0x3cc, 0x3d0, 0x3d4):
                fz.preset_f32('F', off, rng.uniform(-60, 60) if off < 0x3cc else rng.uniform(-1.5, 1.5))
            qv = [rng.uniform(-1, 1) for _ in range(4)]
            length = sum(v * v for v in qv) ** 0.5 or 1.0
            for k in range(4):
                fz.preset_f32('F', 0x3e4 + 4 * k, qv[k] / length)
            for k in range(3):
                fz.preset_f32('B', 0x64f4 + 4 * k, rng.choice([0.0, rng.uniform(-3, 3), rng.uniform(-30, 30)]))
            fz.preset('F', 0x42f88, rng.choice([0, 1, 1]))
            fz.preset('F', 0x42f84, rng.randrange(-1, 5))
            fz.preset('F', 0x28, rng.randrange(0, 10))
            for off in (0x42f60, 0x42f64, 0x42f68, 0x42f6c, 0x42f74):
                fz.preset_f32('F', off, rng.uniform(-50, 50))
            fz.preset_f32('F', 0x42f70, rng.choice([0.0, 0.005, -0.005, 0.5, -0.7, 1.0, 3.0]))
            case.emu.uc.reg_write(UC_X86_REG_XMM12, 0x3c8efa36)
        if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate'):
            S = case.region('S', 0x2000)
            V = case.region('V', 0x4000)
            if BLOCK != 'motion':
                fz.preset('F', 0x28, rng.choice([7, 12345]))
            for w in range(48) if BLOCK == 'element' else []:
                fz.preset('W', 0x36c8 * w + 0x58, rng.choice([0, 1]))
            case.emu.write_u32(RECORDING_ID, 12345)
            for address, value in ((RECORD_VECTOR + 8, V), (RECORD_VECTOR + 16, V + 0x4000)):
                case.emu.write_u32(address, value & 0xffffffff)
                case.emu.write_u32(address + 4, value >> 32)
            extra = {**extra, RECORDING_ID: 12345}
            for address, value in ((RECORD_VECTOR + 8, V), (RECORD_VECTOR + 16, V + 0x4000)):
                extra[address] = value & 0xffffffff
                extra[address + 4] = value >> 32
            rbp = S + 0x400
            if BLOCK == 'motion':
                for off, lo, hi in ((0x1750, 500.0, 5000.0), (0x1758, 500.0, 5000.0), (-0x78, 300.0, 4000.0)):
                    fz.preset_f32('S', 0x400 + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_RBP, rbp)
            case.emu.uc.reg_write(UC_X86_REG_XMM15, 0x80000000)
            case.emu.uc.reg_write(UC_X86_REG_XMM11, 0x3c8efa36)
            case.emu.uc.reg_write(UC_X86_REG_R13, 0xffffffffffffffff)
        case.emu.uc.reg_write(UC_X86_REG_R15, F)
        case.emu.uc.reg_write(UC_X86_REG_R14, 1 if BLOCK == 'motion' else 0)
        case.emu.uc.reg_write(UC_X86_REG_R12, 0)
        if BLOCK != 'integrate':
            case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        case.emu.uc.reg_write(UC_X86_REG_XMM13, 0)
        case.emu.uc.reg_write(UC_X86_REG_XMM14, struct.unpack('<I', struct.pack('<f', 1.0))[0])
        try:
            case.run(start, until=end, max_instructions=3_000_000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x}' + (f' {rbp:x}' if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate') else '') + (f' {entry_rsp(0) - 0xc0:x}' if BLOCK == 'integrate' else '')
        out = case.dump(header, extra_words=extra)
        if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate'):
            lines = out.split('\n')
            lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not S <= int(t.split('=')[0], 16) < S + 0x2000)
            out = '\n'.join(lines)
        print(out)
        done += 1


if __name__ == '__main__':
    main()
