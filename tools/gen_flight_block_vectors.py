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
from unicorn.x86_const import (UC_X86_REG_R12, UC_X86_REG_R13, UC_X86_REG_R11, UC_X86_REG_R14, UC_X86_REG_R15, UC_X86_REG_RBP, UC_X86_REG_RBX, UC_X86_REG_RIP, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_RDI, UC_X86_REG_RSI, UC_X86_REG_XMM10, UC_X86_REG_XMM6, UC_X86_REG_XMM7, UC_X86_REG_XMM8, UC_X86_REG_XMM9, UC_X86_REG_XMM11,  # noqa: E402
                               UC_X86_REG_XMM12, UC_X86_REG_XMM13, UC_X86_REG_XMM14, UC_X86_REG_XMM15)

EXE, BLOCK, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
BLOCKS = {'aspect': (0x141265f7d, 0x14126644a), 'thrust': (0x14126644a, 0x141266b52), 'element': (0x141266b52, 0x141267978),
          'body': (0x141267978, 0x1412686a9), 'parts': (0x141269920, 0x14126a791), 'motion': (0x1412709ff, 0x141271fa2), 'integrate': (0x141271fa2, 0x14127223b), 'velocity': (0x14127223b, 0x141272395), 'geodetic': (0x141272395, 0x1412728f7), 'angles': (0x1412728f7, 0x141272a96), 'path': (0x141272a96, 0x14127307c), 'instruments': (0x14127307c, 0x141273779), 'coeff': (0x141273779, 0x141273dfb), 'late': (0x141273dfb, 0x14127402d), 'arm': (0x14126a791, 0x14126aad6), 'gear': (0x14126883f, 0x141269920), 'wheels': (0x1412686a9, 0x14126883f), 'hook': (0x14126aad6, 0x14126b3ee), 'tow': (0x14126b3e8, 0x14126b548), 'floats': (0x14126b548, 0x14126b757), 'waves': (0x14126b757, 0x14126bf30), 'pull': (0x14126bf30, 0x14126c034), 'gtarget': (0x14126c034, 0x14126c4e4), 'steer': (0x14126c4e4, 0x14126c7dc), 'gstate': (0x14126c7dc, 0x14126d1f2), 'wcontact': (0x14126d1f2, 0x14126d5d3), 'wprobe': (0x14126d5d3, 0x14126dd98), 'bsurf': (0x14126de36, 0x14126e4c8), 'bblend': (0x14126e4c8, 0x14126e974), 'bloop': (0x14126dd98, 0x14126e9a0), 'ground': (0x14126e9a0, 0x14126ef74), 'gdrag': (0x14126ef74, 0x14126f37f), 'reset': (0x141265731, 0x14126580a), 'vec': (0x141265810, 0x1412659b9), 'strips': (0x1412659b9, 0x141265de3), 'wstrips': (0x141265de3, 0x141265f7d), 'tail': (0x14127402d, 0x141274060)}
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
    if BLOCK == 'geodetic':
        def put_f64(call, address, value):
            bits64 = struct.unpack('<Q', struct.pack('<d', value))[0]
            call.put(address, bits64 & 0xffffffff)
            call.put(address + 4, bits64 >> 32)

        def geographic(call, rng):  # 0x1406eaf20: three doubles through three pointers
            for pointer in (call.ints[1], call.ints[2], call.ints[3]):
                put_f64(call, pointer, rng.uniform(-180, 180))

        def local_matrix(call, rng):  # 0x1419f7ee0: three rows of three floats, 16 bytes apart
            for row in range(3):
                for col in range(3):
                    call.put_f32(call.ints[1] + 0x10 * row + 4 * col, rng.uniform(-1, 1))

        def euler(call, rng):  # 0x1419f6fd0: three angles through three pointers
            for pointer in (call.ints[1], call.ints[2], call.ints[3]):
                call.put_f32(pointer, rng.uniform(-180, 180))

        case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
    print('# update_flight block vectors', BLOCK, hex(start), hex(end))
    rng = case.rng
    dirty = []
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
        if BLOCK == 'geodetic':
            CX = case.region('CX', 0x300)
            case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
            for off in range(0x200, 0x278, 8):
                fz.preset_f64('CX', off, rng.uniform(-3.0, 3.0))
            fz.preset_f64('CX', 0xb0, 6378137.0)
            fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
            fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
            fz.preset_f64('CX', 0xd0, rng.uniform(0.0, 0.01))
            axis = rng.choice([0, 0, 0, 1, 2, 3])
            if axis:
                for off in (0x200, 0x220, 0x240, 0x260):
                    fz.preset_f64('CX', off, 0.0)
                fz.preset_f64('CX', 0x268, [0.0, 1.0, -1.0][axis - 1])
                if axis == 1:
                    for off in (0x208, 0x228, 0x248):
                        fz.preset_f64('CX', off, 0.0)
                    fz.preset_f64('CX', 0x270, rng.choice([0.0, 1.0, -1.0]))
                    fz.preset_f64('CX', 0x230, 0.0)
                    fz.preset_f64('CX', 0x210, 0.0)
                    fz.preset_f64('CX', 0x250, 0.0)
            for off in (0x368, 0x36c, 0x370, 0x438, 0x43c, 0x448, 0x44c, 0x458, 0x45c):
                fz.preset_f32('F', off, rng.uniform(-60, 60))
            for off in (0x3e0, 0x3dc, 0x3d8, 0x350, 0x358, 0x348):
                fz.preset_f32('F', off, rng.uniform(-180, 180))
            for off in (0x378, 0x380, 0x388, 0x390, 0x398):
                fz.preset_f64('F', off, rng.uniform(-5000, 5000))
            case.emu.uc.reg_write(UC_X86_REG_XMM12, 0x3c8efa36)
            case.emu.uc.reg_write(UC_X86_REG_XMM8, 0)
        if BLOCK == 'path':
            CX = case.region('CX', 0x300)
            case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
            case.stub(0x1407d76f0, lambda call, rng: call.ret_f32(rng.choice([rng.uniform(-5, 30), rng.uniform(0, 8), 20.0])))
            case.stub(0x1406e2be0, lambda call, rng: call.ret_f64(rng.uniform(0, 5000)))
            g = struct.unpack('<Q', struct.pack('<d', rng.choice([0.5, 1.0, 2.0, 4.0, 2.0])))[0]
            case.emu.write_u32(0x142f01920, g & 0xffffffff)
            case.emu.write_u32(0x142f01924, g >> 32)
            extra[0x142f01920] = g & 0xffffffff
            extra[0x142f01924] = g >> 32
            speeds = [rng.uniform(0, 0.2) for _ in range(3)] if rng.random() < 0.45 else [rng.choice([0.0, 0.3, 0.9, 5.0, 40.0]) for _ in range(3)]
            for off, v in zip((0x368, 0x36c, 0x370), speeds):
                fz.preset_f32('F', off, v)
            case.emu.uc.reg_write(UC_X86_REG_XMM7, struct.unpack('<I', struct.pack('<f', speeds[0]))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM9, struct.unpack('<I', struct.pack('<f', speeds[2]))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM11, 0)
            fz.preset('F', 0x65b0, rng.choice([0, 0, 0, 1]))
            fz.preset('F', 0x65b4, rng.choice([0, 0, 0, 1]))
            fz.preset('F', 0x24c, rng.choice([0, 1]))
            for off in (0x390, 0x398):
                fz.preset_f64('F', off, rng.choice([rng.uniform(-90, 90), 0.0]))
            for off in (0x65b8, 0x65c0, 0x65c8, 0x65d0, 0x65d8, 0x65e0, 0x65e8, 0x65f0, 0x65f8, 0x6600, 0x6608, 0x6610):
                fz.preset_f64('F', off, rng.choice([rng.uniform(-90, 90)] * 12 + [0.0]))
        if BLOCK == 'instruments':
            for address in (0x14076b5d0, 0x141244b60, 0x1407d7bc0, 0x1407cc570):
                case.stub(address, lambda call, rng: call.ret_f32(rng.uniform(-30, 30)))
            for off in (0x368, 0x36c, 0x370, 0x400, 0x404, 0x408, 0x414, 0x3d0, 0x344, 0x34c, 0x350, 0x358):
                fz.preset_f32('F', off, rng.choice([rng.uniform(-60, 60), rng.uniform(-3, 3), rng.uniform(-200, 200)]) if off in (0x358,) else rng.uniform(-60, 60))
            for off in (0x70, 0x6c, 0x74):
                fz.preset_f32('F', off, rng.uniform(0.2, 1.5))
            for off in (0x24a4, 0x2498):
                fz.preset_f32('B', off, rng.uniform(0.05, 5.0))
            for off in (0x500, 0x504, 0x508, 0x50c, 0x510, 0x514, 0x518, 0x51c, 0x520, 0x524, 0x528, 0x52c, 0x530, 0x534, 0x538, 0x53c, 0x6638, 0x663c):
                fz.preset_f32('F', off, rng.uniform(-20, 20))
            for off in (0x390, 0x398):
                fz.preset_f64('F', off, rng.uniform(-90, 90))
            case.emu.uc.reg_write(UC_X86_REG_XMM8, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM11, 0)
        if BLOCK == 'coeff':
            case.stub(0x141a6ba30, lambda call, rng: [call.put_f32(call.ints[2] + 4 * k, rng.choice([rng.uniform(-2, 2), 0.0])) for k in range(3)] and None)
            fz.preset('F', 0x42f84, rng.randrange(0, 9))
            fz.preset('F', 0x24c, rng.choice([0, 1]))
            for off in (0x460, 0x464, 0x468, 0x46c):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
            for off in (0x2c0, 0x2d4, 0x2e8):
                fz.preset_f32('F', off, rng.uniform(-3000, 3000))
            fz.preset_f32('F', 0x344, rng.choice([rng.uniform(0.7, 1.3), rng.uniform(-3, 3)]))
            fz.preset_f32('F', 0x424, rng.uniform(1.0, 500.0))
            fz.preset_f32('F', 0x288, rng.uniform(500, 5000))
            fz.preset_f32('B', 0x2880, rng.uniform(5, 30))
            fz.preset_f32('B', 0x288c, rng.choice([rng.uniform(100, 6000), 100.0]))
            for off in (0x28e8, 0x28ec, 0x28f0):
                fz.preset_f32('B', off, rng.choice([rng.uniform(0.1, 3.0)] * 5 + [-1.0, 0.0]))
            for off in (0x2898, 0x28e4, 0x2888, 0x28f8, 0x28fc):
                fz.preset_f32('B', off, rng.uniform(-1, 1))
            fz.preset('B', 0, rng.choice([0x400, 0x4af, 0x4b0, 0x500, 0x600]))
            fz.preset_f32('B', 0x64f4, rng.uniform(0, 3))
            for k in range(9):
                fz.preset_f32('F', 0xbbb4 + 4 * k, rng.choice([rng.uniform(0.1, 3.0), 0.0, rng.uniform(0.1, 3.0)]))
            case.emu.uc.reg_write(UC_X86_REG_RDI, 0)
            case.emu.uc.reg_write(UC_X86_REG_RSI, 1)
            case.emu.uc.reg_write(UC_X86_REG_XMM8, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<Q', struct.pack('<d', 0.5))[0])
        if BLOCK == 'late':
            for address in (0x141245750, 0x14125e4f0):
                case.stub(address, lambda call, rng: None)
            fz.preset('B', 0xc54, rng.choice([0, 1]))
            fz.preset('B', 0x8c4, rng.choice([0, 1, 1]))
            fz.preset('B', 0x8c8, rng.choice([0, 1]))
            fz.preset_f32('B', 0x7b0, rng.uniform(5, 100))
            fz.preset_f32('B', 0x8ec, rng.uniform(1, 20))
            for off, lo, hi in ((0x41c, -50, 200), (0x404, -5, 25), (0x6c98, -10, 100), (0x6f4c, 0, 80), (0x5c, -5, 5), (0x3e0, 0, 360)):
                fz.preset_f32('F', off, rng.uniform(lo, hi))
            fz.preset('F', 0x28, rng.choice([0, 1]))
            fz.preset('F', 0xdbc, rng.choice([0, 1]))
            fz.preset_f32('F', 0x3d8, rng.choice([rng.uniform(-90, 90), rng.uniform(-30, 30)]))
            fz.preset_f32('F', 0x3dc, rng.choice([rng.uniform(-45, 45), rng.uniform(-15, 15)]))
            g = struct.unpack('<Q', struct.pack('<d', rng.uniform(0, 80)))[0]
            case.emu.write_u32(0x142f01918, g & 0xffffffff)
            case.emu.write_u32(0x142f0191c, g >> 32)
            extra[0x142f01918] = g & 0xffffffff
            extra[0x142f0191c] = g >> 32
            flag = rng.choice([0, 1, 1])
            case.emu.write_u32(0x142f01978, flag)
            extra[0x142f01978] = flag
            case.emu.uc.reg_write(UC_X86_REG_RDI, 0)
            case.emu.uc.reg_write(UC_X86_REG_RSI, 1)
        if BLOCK == 'arm':
            def tip_probe(call, rng):
                if rng.random() < 0.7:
                    for k in range(8):
                        call.put_f32(call.ints[3] + 4 * k, rng.uniform(-20, 20))
                call.ret_int(1 if rng.random() < 0.6 else 0)
            case.stub(0x14195ffc0, tip_probe)
            case.emu.uc.reg_write(UC_X86_REG_RBX, B)
            fz.preset('F', 0x28, rng.choice([0, 0, 0, 7]))
            for off, lo, hi in ((0x444c, -30, 0), (0x4450, 0, 60), (0x4454, 0.2, 4), (0x4448, -3, 3), (0x4444, -3, 3), (0x4440, -3, 3)):
                fz.preset_f32('B', off, rng.choice([rng.uniform(lo, hi)] * 6 + ([0.0] if off == 0x4454 else [])))
            fz.preset_f32('F', 0x6528, rng.uniform(-0.5, 1.5))
            fz.preset_f32('F', 0x6548, rng.uniform(-30, 60))
            fz.preset_f32('F', 0x42f50, rng.uniform(0, 3))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
        if BLOCK == 'gear':
            for address in (0x1405dd6e0, 0x1407552a0, 0x1405ddc50):
                case.stub(address, lambda call, rng: None, record=False)
            G = case.region('G', 0x88 * 10)
            EL = case.region('EL', 0x90 * 10)
            PP = case.region('PP', 0x100 * 10)
            T = case.region('T', 0x34c8 * 39)
            for base, off, ptr in ((B, 0x6080, G), (B, 0x6040, T), (F, 0x6958, EL), (F, 0x6960, EL + 0x90 * rng.choice([10, 10, 10, 9]))):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset_f32('B', 0x2830, rng.choice([0.0, 0.3, 1.0]))
            fz.preset_f32('B', 0x2834, rng.uniform(0.1, 2.0))
            fz.preset_f32('F', 0x424, rng.uniform(1, 500))
            for off in (0x29c, 0x2a8, 0x2b4):
                fz.preset_f32('F', off, rng.uniform(-60, 60))
            for k in range(39):
                base = 0x34c8 * k
                live = rng.random() < 0.15
                fz.preset('T', base + 0x5f0, 0x27)
                fz.preset('T', base + 0x54, 0 if live else 1)
                fz.preset('T', base + 0x588, 1)
                fz.preset('T', base + 0x5f4, rng.randrange(0, 10))
            for i in range(10):
                gb = 0x88 * i
                fz.preset('G', gb, rng.choice([0, 1, 2, 3, 4, 5, 6, 7, 8]))
                fz.preset('G', gb + 4, rng.choice([0, 1]))
                for off, lo, hi in ((0x58, 0.0, 3.0), (0x5c, 0.0, 3.0), (0x18, 0.0, 3.0), (0x64, -3, 3), (0x70, -3, 3), (0x7c, -3, 3)):
                    fz.preset_f32('G', gb + off, rng.uniform(lo, hi))
                eb = 0x90 * i
                fz.preset_f32('EL', eb + 0x10, rng.choice([0.0, 0.005, rng.uniform(0, 1), rng.uniform(-0.3, 1.2)]))
                for off, lo, hi in ((0x14, -90, 90), (0x18, -90, 90), (0x2c, -3, 3)):
                    fz.preset_f32('EL', eb + off, rng.uniform(lo, hi))
                pt = PP + 0x100 * i
                fz.preset('EL', eb, pt & 0xffffffff, record=True)
                fz.preset('EL', eb + 4, pt >> 32, record=True)
                for off, lo, hi in ((0x18, -3, 3), (0x20, -3, 3), (0x64, -3, 3), (0x70, -3, 3), (0x7c, -3, 3)):
                    fz.preset_f32('PP', 0x100 * i + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_RSI, 0)
            case.emu.uc.reg_write(UC_X86_REG_XMM7, 0)
        if BLOCK == 'wheels':
            G = case.region('G', 0x88 * 10)
            fz.preset('B', 0x6080, G & 0xffffffff, record=True)
            fz.preset('B', 0x6084, G >> 32, record=True)
            for i in range(10):
                fz.preset('G', 0x88 * i, rng.choice([0, 1, 5]))
                fz.preset('G', 0x88 * i + 4, rng.choice([0, 1, 1]))
            case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        if BLOCK == 'hook':
            OBJ = case.region('OBJ', 0x100)
            case.stub(0x1411b63a0, lambda call, rng: call.ret_int(OBJ if rng.random() < 0.9 else 0))
            for address in (0x1407cd810, 0x1408be280, 0x1408625a0):
                case.stub(address, lambda call, rng: call.ret_f32(rng.choice([rng.uniform(-5, 20), rng.uniform(0, 3), rng.uniform(10.5, 20), rng.uniform(10.5, 20)])))
            case.stub(0x1408ce690, lambda call, rng: call.ret_f64(rng.uniform(-1.5, 1.5)))
            case.stub(0x1411e14d0, lambda call, rng: call.ret_int(rng.choice([0, 1, 1])))
            case.stub(0x1408e25a0, lambda call, rng: None, record=False)
            for off in (0x64, 0x68, 0x6c, 0x80, 0x84, 0x88):
                fz.preset_f32('OBJ', off, rng.uniform(-30, 30) if off >= 0x80 else rng.uniform(-3, 3))
            TABLE = 0x14578b040
            for off in [0x30, 0xc8] + [o + 8 * k for o in (0x8c, 0x90, 0xa4, 0xa8) for k in range(-1, 3)]:
                v = struct.unpack('<I', struct.pack('<f', rng.uniform(-3, 3) if off not in (0xc8,) else rng.uniform(0, 2)))[0]
                case.emu.write_u32(TABLE + off, v)
                extra[TABLE + off] = v
            def put(address, word):
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            put(0x14589a000, rng.choice([0xffffffff, 0, 1, 2, 0xffffffff, 0xffffffff, 0xffffffff]))
            put(0x14589a004, rng.choice([0xffffffff, 0, 1, 2]))
            for address in (0x14589a008, 0x14589a010, 0x14589a018, 0x14589a020):
                bits64 = struct.unpack('<Q', struct.pack('<d', rng.uniform(0, 1.2) if address == 0x14589a008 else rng.uniform(-3, 3)))[0]
                put(address, bits64 & 0xffffffff)
                put(address + 4, bits64 >> 32)
            put(0x145899ffc, struct.unpack('<I', struct.pack('<f', rng.uniform(0, 5)))[0])
            fz.preset('F', 0x28, rng.choice([0, 0, 0, 7]))
            for off, lo, hi in ((0x6528, 0.5, 1.5), (0x6548, -20, 40), (0x654c, -3, 3), (0x6550, -3, 3), (0x6554, -3, 3), (0x6558, -3, 3), (0x655c, -3, 3), (0x6560, -3, 3), (0x288, 500, 3000), (0x368, -30, 30), (0x36c, -30, 30), (0x370, -30, 30)):
                fz.preset_f32('F', off, rng.uniform(lo, hi))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
            for off in (0x378, 0x380, 0x388):
                fz.preset_f64('F', off, rng.uniform(-100, 100))
            for off, lo, hi in ((0x4440, -3, 3), (0x4444, -3, 3), (0x4448, -3, 3), (0x4454, 0.2, 3)):
                fz.preset_f32('B', off, rng.uniform(lo, hi) if off != 0x4454 or rng.random() > 0.1 else 0.0)
            esi_v = rng.choice([0, 1, 1, 1])
            case.emu.uc.reg_write(UC_X86_REG_RSI, esi_v)
            case.emu.uc.reg_write(UC_X86_REG_R14, F + 0x6548)
            case.emu.uc.reg_write(UC_X86_REG_XMM8, struct.unpack('<Q', struct.pack('<d', 0.5))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<I', struct.pack('<f', 0.5))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        if BLOCK == 'tow':
            case.stub(0x140f5c540, lambda call, rng: call.ret_int(rng.randrange(2)))
            case.stub(0x140f42620, lambda call, rng: call.ret_int(rng.randrange(1 << 40)))
            case.stub(0x140f39ae0, lambda call, rng: None)
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
            fz.preset_f32('B', 0x2894, rng.choice([0.0, rng.uniform(0.1, 3), -1.0]))
            fz.preset_f32('B', 0x289c, rng.uniform(-3, 3))
            fz.preset('F', 0xdac, rng.choice([0, 0, 1]))
            fz.preset_f32('F', 0x70, rng.choice([0.0, 0.005, rng.uniform(0.01, 1.3)]))
            fz.preset_f32('F', 0x6590, rng.uniform(0, 5))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
        if BLOCK == 'floats':
            R = case.region('R', 0x720)
            fz.preset('B', 0x6098, R & 0xffffffff, record=True)
            fz.preset('B', 0x609c, R >> 32, record=True)
            fz.preset_f32('F', 0x148, rng.choice([0.0, 0.005, rng.uniform(0.02, 3.0), rng.uniform(0.02, 3.0)]))
            fz.preset_f32('F', 0x6c, rng.uniform(0.3, 1.3))
            for k in range(4):
                base = 0x1c8 * k
                fz.preset('R', base, rng.choice([0, 1, 1]))
                for off, lo, hi in ((8, 0, 2), (0xc, -3, 3), (0x10, -3, 3), (0x14, -3, 3), (0x1b0, -90, 90), (0x1bc, -90, 90)):
                    fz.preset_f32('R', base + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_RSI, 0)
        if BLOCK == 'waves':
            case.stub(0x1408bd9d0, lambda call, rng: call.ret_f32(rng.uniform(-1, 1)))
            fz.preset('F', 0x650c, rng.choice([0, 1, 1, 1]))
            fz.preset_f32('F', 0x652c, rng.uniform(-0.2, 1.2))
            fz.preset_f32('F', 0x6c, rng.uniform(0.3, 1.3))
            for off in range(0x6564, 0x6588, 4):
                fz.preset_f32('F', off, rng.uniform(-3, 3))
            for base in (0x3f40, 0x3f4c, 0x3f58):
                for k in range(3):
                    fz.preset_f32('B', base + 4 * k, rng.uniform(-3, 3))
            for k in range(3):
                fz.preset_f32('B', 0x3f64 + 4 * k, rng.choice([0.0, rng.uniform(0.1, 3), rng.uniform(0.1, 3)]))
                fz.preset_f32('B', 0x3f70 + 4 * k, rng.choice([0.0, rng.uniform(0.1, 3), rng.uniform(0.1, 3)]))
            g = struct.unpack('<Q', struct.pack('<d', rng.uniform(0, 100)))[0]
            case.emu.write_u32(0x142f01918, g & 0xffffffff)
            case.emu.write_u32(0x142f0191c, g >> 32)
            extra[0x142f01918] = g & 0xffffffff
            extra[0x142f0191c] = g >> 32
            case.emu.uc.reg_write(UC_X86_REG_RSI, 0)
        if BLOCK == 'pull':
            for address in (0x1411e4bd0, 0x1409057f0):
                case.stub(address, lambda call, rng: None)
            case.stub(0x1411d9d80, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
            fz.preset('F', 0x28, rng.choice([0, 1, 1, 2]))
            fz.preset('F', 0x3c, rng.choice([0, 1, 1]))
            for off in (0xf0, 0xfc, 0x108, 0xec):
                fz.preset_f32('F', off, rng.uniform(-3, 3))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
            fz.preset_f32('B', 0x2638, rng.uniform(-3, 3))
            fz.preset_f32('B', 0x263c, rng.uniform(-3, 3))
            def putw(address, word):
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            putw(0x145899fd0, rng.choice([0, 1]))
            putw(0x145899fd4, rng.choice([0, 1]))
            for address in (0x145899fc4, 0x145899fc8, 0x145899fcc):
                putw(address, struct.unpack('<I', struct.pack('<f', rng.uniform(-50, 50)))[0])
        if BLOCK == 'gtarget':
            case.stub(0x1411daa80, lambda call, rng: call.ret_f32(rng.uniform(-2, 2)))
            G = case.region('G', 0x88 * 10)
            EL = case.region('EL', 0x90 * 10)
            PP = case.region('PP', 0x100 * 10)
            for base, off, ptr in ((B, 0x6080, G), (F, 0x6958, EL), (F, 0x6960, EL + 0x90 * rng.choice([10, 10, 10, 9]))):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('F', 0x214, rng.choice([0, 0, 1]))
            fz.preset('F', 0x689c, rng.choice([0, 1]))
            fz.preset('F', 0x28, rng.choice([0, 7, 7]))
            for off in (0x218, 0x20c, 0x208):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
            fz.preset_f32('B', 0x1e94, rng.choice([0.0, rng.uniform(1, 40), rng.uniform(1, 40)]))
            fz.preset('B', 0xe78, rng.choice([0, 1]))
            for address in dirty:
                case.emu.write_u32(address, 0)
            dirty.clear()
            def putg(address, word):
                dirty.append(address)
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            for code in rng.sample([0x25, 0x49, 0x1], rng.randrange(0, 3)):
                k = rng.randrange(0, 500)
                putg(0x1460e9708 + 4 * k, code)
                putg(0x1460e9708 + 4 * k + 0xba44, rng.choice([0, 1, 1]))
            for i in range(10):
                gb = 0x88 * i
                fz.preset('G', gb, rng.choice([0, 1, 2, 3, 4, 5, 6, 7, 8]))
                fz.preset('G', gb + 0xc, rng.choice([0, 1]))
                for off, lo, hi in ((0x50, -1, 1.2), (0x54, -1, 1.2), (0x58, 0, 3)):
                    fz.preset_f32('G', gb + off, rng.choice([rng.uniform(lo, hi), 0.0, 0.005]))
                eb = 0x90 * i
                for off, lo, hi in ((0x10, 0, 1.2), (0x18, -90, 90), (0x40, 0, 3), (0x24, -1, 1)):
                    fz.preset_f32('EL', eb + off, rng.uniform(lo, hi))
                pt = PP + 0x100 * i
                fz.preset('EL', eb, pt & 0xffffffff, record=True)
                fz.preset('EL', eb + 4, pt >> 32, record=True)
                for off, lo, hi in ((0x18, -3, 3), (0x20, -3, 3), (0x7c, -3, 3)):
                    fz.preset_f32('PP', 0x100 * i + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_XMM7, struct.unpack('<Q', struct.pack('<d', 0.01))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<I', struct.pack('<f', 0.5))[0])
            case.emu.uc.reg_write(UC_X86_REG_RSI, 0)
            case.emu.uc.reg_write(UC_X86_REG_RDI, 0)
        if BLOCK == 'steer':
            case.stub(0x1417dacf0, lambda call, rng: call.ret_int(rng.choice([0, 0, 0, 0, 0, 1])))
            case.stub(0x1411e3ee0, lambda call, rng: call.ret_int(rng.choice([0, 1])))
            case.stub(0x14123d110, lambda call, rng: call.ret_f32(rng.uniform(1, 80)))
            case.stub(0x1408625a0, lambda call, rng: call.ret_f32(rng.uniform(-1, 1)))
            fz.preset('F', 0x3c, rng.choice([1, 1, 1, 0]))
            fz.preset('F', 0x28, rng.choice([0, 7]))
            fz.preset('F', 0x6824, rng.choice([0, 0, 1]))
            fz.preset_f32('F', 0x23c, rng.choice([0.0, 0.0, 0.5]))
            fz.preset_f32('F', 0x238, rng.choice([0.0, 0.0, 0.5]))
            fz.preset_f32('F', 0x41c, rng.uniform(0, 80))
            for off in (0x240, 0x244, 0x224):
                fz.preset_f32('F', off, rng.choice([rng.uniform(0, 1), 0.95, 1.0]))
            for off in (0x228, 0x22c, 0x220):
                fz.preset('F', off, rng.choice([0, 1]))
            fz.preset_f32('B', 0x2840, rng.choice([0.0, rng.uniform(0.05, 2), rng.uniform(0.05, 2)]))
            fz.preset('B', 0xe94, rng.choice([0, 1]))
            fz.preset('B', 0xe90, rng.choice([0, 1, 1]))
            dirty_w = []
            for address in dirty:
                case.emu.write_u32(address, 0)
            dirty.clear()
            def putg(address, word):
                dirty.append(address)
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            for address in (0x142fe2a48, 0x1460e0a1c, 0x1460e0a6c):
                putg(address, rng.choice([0, 0, 0, 0, 0, 1]))
            for code in rng.sample([6, 7, 1], rng.randrange(0, 3)):
                k = rng.randrange(0, 500)
                putg(0x1460e9708 + 4 * k, code)
                putg(0x1460e9708 + 4 * k + 0xba44, rng.choice([0, 1, 1]))
            case.emu.uc.reg_write(UC_X86_REG_RBX, F + 0x3c)
            case.emu.uc.reg_write(UC_X86_REG_R12, 0x1460e9708)
            case.emu.uc.reg_write(UC_X86_REG_RDX, 0x1460e9ed8)
            case.emu.uc.reg_write(UC_X86_REG_XMM9, 0x7fffffff)
            case.emu.uc.reg_write(UC_X86_REG_XMM7, struct.unpack('<Q', struct.pack('<d', 0.01))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<I', struct.pack('<f', 0.5))[0])
        if BLOCK == 'gstate':
            case.stub(0x1411daa80, lambda call, rng: call.ret_f32(rng.uniform(-1, 1)))
            def world3(call, rng):
                for pointer in (call.ints[2], call.stack[0], call.stack[2]):
                    bits64 = struct.unpack('<Q', struct.pack('<d', rng.uniform(-100, 100)))[0]
                    call.put(pointer, bits64 & 0xffffffff)
                    call.put(pointer + 4, bits64 >> 32)
            CX = case.region('CX', 0x300)
            case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
            for off in range(0x200, 0x278, 8):
                fz.preset_f64('CX', off, rng.uniform(-3.0, 3.0))
            fz.preset_f64('CX', 0xb0, 6378137.0)
            fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
            fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
            fz.preset_f64('CX', 0xd0, rng.uniform(0.0, 0.01))
            def geo3(call, rng):
                for pointer in (call.ints[1], call.ints[2], call.ints[3]):
                    bits64 = struct.unpack('<Q', struct.pack('<d', rng.uniform(-180, 180)))[0]
                    call.put(pointer, bits64 & 0xffffffff)
                    call.put(pointer + 4, bits64 >> 32)
            case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
            G = case.region('G', 0x88 * 10)
            EL = case.region('EL', 0x90 * 10)
            PP = case.region('PP', 0x100 * 10)
            OW = case.region('OW', 0x1000)
            W = case.region('W', 0x36c8 * 48)
            X = case.region('X', 0x2d8 * 48)
            for base, off, ptr in ((B, 0x6080, G), (F, 0x6958, EL), (F, 0x6960, EL + 0x90 * 10), (B, 0x6028, W), (F, 0x6940, X), (F, 0xbde0, OW)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('OW', 0xe7c, rng.randrange(0, 4))
            fz.preset('B', 0xe7c, rng.choice([0, 1, 1]))
            fz.preset('F', 0x24c, rng.choice([0, 1]))
            fz.preset('F', 0x64e4, rng.choice([0, 1]))
            for off in (0x224, 0x240, 0x244):
                fz.preset_f32('F', off, rng.uniform(0, 1))
            for off in range(0xbdd8, 0xbe40, 4):
                if off not in (0xbde0, 0xbde4):
                    fz.preset_f32('F', off, rng.uniform(0, 1))
            for off in (0x280c, 0x2810, 0x2814):
                fz.preset_f32('B', off, rng.uniform(-3, 3))
            for w in range(48):
                fz.preset('W', 0x36c8 * w + 0x678, rng.choice([0, 1, 1]))
            for i in range(10):
                gb = 0x88 * i
                fz.preset('G', gb, rng.choice([0, 1, 2, 3]))
                fz.preset('G', gb + 8, rng.choice([0, 1, 1]))
                fz.preset_f32('G', gb + 0x24, rng.uniform(-1, 1))
                eb = 0x90 * i
                for off, lo, hi in ((0x10, 0, 1.2), (0x14, -90, 90), (0x18, -90, 90)):
                    fz.preset_f32('EL', eb + off, rng.uniform(lo, hi))
                pt = PP + 0x100 * i
                fz.preset('EL', eb, pt & 0xffffffff, record=True)
                fz.preset('EL', eb + 4, pt >> 32, record=True)
                for off, lo, hi in ((0x18, -3, 3), (0x20, -3, 3), (0x64, -2, 2)):
                    fz.preset_f32('PP', 0x100 * i + off, rng.uniform(lo, hi))
            def putg(address, word):
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            g = struct.unpack('<Q', struct.pack('<d', rng.choice([0.5, 1.0, 2.0, 4.0])))[0]
            putg(0x142f01920, g & 0xffffffff)
            putg(0x142f01924, g >> 32)
            putg(0x142f01968, rng.choice([0, 1]))
            case.emu.uc.reg_write(UC_X86_REG_XMM7, struct.unpack('<Q', struct.pack('<d', 0.01))[0])
            case.emu.uc.reg_write(UC_X86_REG_XMM9, 0x7fffffff)
        if BLOCK == 'wcontact':
            case.stub(0x1411878f0, lambda call, rng: None)
            case.stub(0x1411c8690, lambda call, rng: call.ret_int(rng.choice([0, 1, 1, 2])))
            case.stub(0x1411c7a50, lambda call, rng: call.ret_int(rng.choice([0, 1, 2])))
            G = case.region('G', 0x88 * 10)
            EL = case.region('EL', 0x90 * 10)
            PP = case.region('PP', 0x100 * 10)
            for base, off, ptr in ((B, 0x6080, G), (F, 0x6958, EL), (F, 0x6960, EL + 0x90 * rng.choice([10, 10, 10, 9]))):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('B', 0xe90, rng.choice([0, 1, 2, 3]))
            fz.preset('F', 0x22c, rng.choice([0, 1]))
            fz.preset('F', 0x28, rng.choice([0, 7]))
            for off in (0x220, 0x224, 0x64c4, 0x64c8):
                fz.preset_f32('F', off, rng.uniform(0, 1))
            fz.preset_f32('F', 0x348, rng.choice([rng.uniform(-60, 60), rng.uniform(-30, 30)]))
            fz.preset_f32('F', 0x350, rng.choice([rng.uniform(-60, 60), rng.uniform(-30, 30)]))
            fz.preset_f32('B', 0x65a8, rng.choice([rng.uniform(-60, 60), rng.uniform(-30, 30)]))
            fz.preset_f32('F', 0x42f50, rng.uniform(0, 3))
            fz.preset_f32('B', 0x2808, rng.uniform(0, 3))
            for i in range(10):
                gb = 0x88 * i
                fz.preset('G', gb, rng.choice([0, 1, 2, 3]))
                for off, lo, hi in ((0x30, 0.5, 3), (0x34, 0.5, 3), (0x38, -1, 1)):
                    fz.preset_f32('G', gb + off, rng.uniform(lo, hi))
                eb = 0x90 * i
                for off, lo, hi in ((0x10, 0, 1.2), (0x14, -90, 90), (0x18, -90, 90)):
                    fz.preset_f32('EL', eb + off, rng.uniform(lo, hi))
                pt = PP + 0x100 * i
                fz.preset('EL', eb, pt & 0xffffffff, record=True)
                fz.preset('EL', eb + 4, pt >> 32, record=True)
                for off, lo, hi in ((0x18, -3, 3), (0x20, -3, 3), (0x64, -2, 2), (0x70, -2, 2), (0x7c, -3, 3)):
                    fz.preset_f32('PP', 0x100 * i + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_RCX, B)
            case.emu.uc.reg_write(UC_X86_REG_XMM9, 0x7fffffff)
        if BLOCK == 'wprobe':
            case.stub(0x1411c7a50, lambda call, rng: call.ret_int(rng.choice([0, 1, 2])))
            from unicorn import UC_HOOK_CODE
            def stop_at_skip(uc, address, size, user):
                if address == 0x14126ef6b:
                    uc.emu_stop()
            if not getattr(case, 'skip_hook', False):
                case.emu.uc.hook_add(UC_HOOK_CODE, stop_at_skip)
                case.skip_hook = True
            W = case.region('W', 0x36c8 * 48)
            X = case.region('X', 0x2d8 * 48)
            for base, off, ptr in ((B, 0x6028, W), (F, 0x6940, X)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset_f32('F', 0x42ec8, rng.choice([1000.0, 1000.0, 1000.0, -1000.0]))
            fz.preset_f64('F', 0x380, rng.uniform(-10, 30))
            for k in range(3):
                fz.preset_f32('B', 0x64f4 + 4 * k, rng.uniform(-3, 3))
            fz.preset_f32('B', 0x64ac, rng.uniform(0.2, 3))
            fz.preset_f32('B', 0x2898, rng.uniform(0.1, 3))
            fz.preset_f32('F', 0x42f50, rng.uniform(0.1, 3))
            for w in range(48):
                wb = 0x36c8 * w
                fz.preset('W', wb + 0x678, rng.choice([0, 1, 1]))
                n = rng.randrange(1, 4)
                fz.preset('W', wb + 4, n)
                for base in (0x5bc, 0x5e8, 0x614):
                    for k in range(n + 1):
                        fz.preset_f32('W', wb + base + 4 * k, rng.uniform(-5, 5))
            case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
            case.emu.uc.reg_write(UC_X86_REG_RDI, 0)
            case.emu.uc.reg_write(UC_X86_REG_RSI, 1)
        if BLOCK == 'bsurf':
            case.stub(0x1411c7a50, lambda call, rng: call.ret_int(rng.choice([0, 1])))
            D = case.region('D', 0x34c8)
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            case.body = D
            fz.preset_f32('B', 0x64fc, rng.choice([0.0, 0.0, 1.0, 20.0]))
            fz.preset('D', 0x5f4, rng.choice([-1] * 9 + [3]))
            rows = rng.choice([0, 1, 2, 2, 3, 3])
            n = rng.choice([0, 1, 2, 3, 3, 4, 5, 5, 8, 9])
            fz.preset('D', 0x654, rows)
            fz.preset('D', 0x658, n)
            fz.preset_f32('D', 0x664, rng.uniform(-1, 0.3))
            for r in range(4):
                fz.preset_f32('D', 0x58c + 0xd8 * r, rng.uniform(0.2, 2))
            for off in (0x28, 0x24, 0xc):
                fz.preset_f32('D', off, rng.uniform(0.2, 4))
            for off in (0x90, 0x94, 0x98):
                fz.preset_f32('D', 0x588 + off, rng.uniform(-3, 3))
            for off in (0x9c, 0xa0, 0xa4):
                fz.preset_f32('D', 0x588 + off, rng.uniform(-90, 90))
            same = rng.random() < 0.25
            for a in range(3):
                base = 0x660 + 0xd8 * a
                pt = [rng.uniform(-3, 3) for _ in range(3)]
                for m in range(10):
                    q = pt if same else [rng.uniform(-3, 3) for _ in range(3)]
                    for k in range(3):
                        fz.preset_f32('D', base - 4 + 12 * m + 4 * k, q[k])
            for reg, val in ((UC_X86_REG_RSI, D), (UC_X86_REG_RBX, D + 0x588), (UC_X86_REG_R14, D + 0x630),
                             (UC_X86_REG_RDI, 0), (UC_X86_REG_R11, 0)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'bblend':
            def contact(call, rng):
                call.ret_int(rng.choice([0, 1, 1]))
                for k in (3, 4, 5):
                    call.put_f32(call.stack[k], rng.uniform(-2, 2))
                call.put_f32(call.stack[6], rng.choice([-1.0, 0.0, rng.uniform(0, 5)]))
                call.put_f32(call.stack[7], rng.choice([0.0, 0.005, rng.uniform(0, 3)]))
            case.stub(0x1411cb1e0, contact)
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([-0.1, 0.1, 0.3, 0.4, 0.6, rng.uniform(0, 0.6)])))
            D = case.region('D', 0x34c8)
            case.body = D
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            fz.preset('D', 0x654, rng.choice([0, 1, 2, 3, 3, 4]))
            fz.preset('D', 0x658, rng.choice([-1, 0, 1, 3, 4, 7, 8, 12]))
            fz.preset('D', 0x2c, rng.choice([0, 0, 1]))
            for off in range(0x30, 0x54, 4):
                fz.preset_f32('D', off, rng.uniform(0, 1))
            for off in (0x368, 0x370):
                fz.preset_f32('F', off, rng.choice([0.0, rng.uniform(-10, 10), rng.uniform(-3, 3)]))
            for reg, val in ((UC_X86_REG_RSI, D), (UC_X86_REG_RBX, D + 0x588), (UC_X86_REG_RDI, 0),
                             (UC_X86_REG_R11, 0)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'bloop':
            def contact(call, rng):
                call.ret_int(rng.choice([0, 1, 1]))
                for k in (3, 4, 5):
                    call.put_f32(call.stack[k], rng.uniform(-2, 2))
                call.put_f32(call.stack[6], rng.choice([-1.0, 0.0, rng.uniform(0, 5)]))
                call.put_f32(call.stack[7], rng.choice([0.0, 0.005, rng.uniform(0, 3)]))
            case.stub(0x1411cb1e0, contact)
            case.stub(0x1411c7a50, lambda call, rng: call.ret_int(rng.choice([0, 1])))
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([-0.1, 0.1, 0.3, 0.4, 0.6, rng.uniform(0, 0.6)])))
            D = case.region('D', 0x34c8 * 39)
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            for off, ptr in ((0x6040, D),):
                fz.preset('B', off, ptr & 0xffffffff, record=True)
                fz.preset('B', off + 4, ptr >> 32, record=True)
            fz.preset_f32('B', 0x64fc, rng.choice([0.0, 0.0, 1.0]))
            for off in (0x368, 0x370):
                fz.preset_f32('F', off, rng.choice([0.0, rng.uniform(-10, 10), rng.uniform(-3, 3)]))
            for j in range(39):
                b = 0x34c8 * j
                fz.preset('D', b + 0x5f0, rng.choice([0x27, 0x27, 3, 5, 0x26, 0x10]))
                fz.preset('D', b + 0x54, rng.choice([0, 0, 0, 1]))
                fz.preset('D', b + 0x588, rng.choice([0, 1, 1, 1]))
                fz.preset('D', b + 0x5f4, rng.choice([-1, -1, 2]))
                fz.preset('D', b + 0x654, rng.choice([0, 1, 2, 3]))
                fz.preset('D', b + 0x658, rng.choice([0, 1, 2, 3, 4, 5, 8]))
                fz.preset('D', b + 0x2c, rng.choice([0, 0, 1]))
                fz.preset_f32('D', b + 0x664, rng.uniform(-1, 0.3))
                for r in range(4):
                    fz.preset_f32('D', b + 0x58c + 0xd8 * r, rng.uniform(0.2, 2))
                for off in (0x28, 0x24, 0xc):
                    fz.preset_f32('D', b + off, rng.uniform(0.2, 4))
                for off in (0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x4c, 0x50):
                    fz.preset_f32('D', b + off, rng.uniform(0, 1))
                for off in (0x90, 0x94, 0x98):
                    fz.preset_f32('D', b + 0x588 + off, rng.uniform(-3, 3))
                for off in (0x9c, 0xa0, 0xa4):
                    fz.preset_f32('D', b + 0x588 + off, rng.uniform(-90, 90))
                for a in range(3):
                    base = b + 0x660 + 0xd8 * a
                    for m in range(8):
                        for k in range(3):
                            fz.preset_f32('D', base - 4 + 12 * m + 4 * k, rng.uniform(-3, 3))
            for reg, val in ((UC_X86_REG_RSI, 1), (UC_X86_REG_RDI, 0), (UC_X86_REG_R11, 0)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'ground':
            CX = case.region('CX', 0x300)
            case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
            case.stub(0x1408e25a0, lambda call, rng: None, record=False)
            case.stub(0x14119e5b0, lambda call, rng: None)
            def put64f(call, pointer, v):
                bits64 = struct.unpack('<Q', struct.pack('<d', v))[0]
                call.put(pointer, bits64 & 0xffffffff)
                call.put(pointer + 4, bits64 >> 32)
            def geo(call, rng):
                for pointer in (call.ints[1], call.ints[2], call.ints[3]):
                    put64f(call, pointer, rng.uniform(-100, 100))
            def world3(call, rng):
                for pointer in (call.ints[2], call.stack[0], call.stack[2]):
                    put64f(call, pointer, rng.uniform(-100, 100))
            def to_frame(call, rng):
                for pointer in (call.ints[2], call.stack[0], call.stack[2]):
                    call.put_f32(pointer, rng.choice([0.0, rng.uniform(-10, 10), rng.uniform(-0.005, 0.005)]))
            case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
            fz.preset_f64('CX', 0xb0, 6378137.0)
            fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
            fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
            for off in (0x280, 0x288, 0x290, 0x2a0, 0x2a8, 0x2b0, 0x2c0, 0x2c8, 0x2d0):
                fz.preset_f64('CX', off, rng.uniform(-1, 1))
            for off in (0x2e0, 0x2e8, 0x2f0):
                fz.preset_f64('CX', off, rng.uniform(-1e7, 1e7))
            case.stub(0x141296750, to_frame)
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0, 0.4)))
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            fz.preset('B', 0x28dc, rng.choice([0, 1, 1, 1]))
            fz.preset('F', 0x64e4, rng.choice([0, 1, 1]))
            for off in (0x2804, 0x2808):
                fz.preset_f32('B', off, rng.choice([-1.0, 0.0] + [rng.uniform(0.001, 0.01)] * 8))
            for off in (0x2800, 0x2890, 0x2898, 0x28bc, 0x28c0, 0x28c4, 0x280c, 0x2810, 0x2814, 0x64ac):
                fz.preset_f32('B', off, rng.uniform(0.1, 5))
            for off in (0x3d0, 0x3d4, 0x368, 0x36c, 0x370, 0x28c):
                fz.preset_f32('F', off, rng.uniform(-6, 6))
            fz.preset_f32('F', 0x64c8, rng.uniform(-0.01, 0.01))
            fz.preset_f32('B', 0x288c, rng.uniform(5, 500))
            fz.preset_f32('B', 0x7b0, rng.uniform(5, 80))
            fz.preset_f32('F', 0x64cc, rng.uniform(-300, 300))
            fz.preset_f32('F', 0x3e0, rng.uniform(-400, 400))
            fz.preset_f32('F', 0x64dc, rng.choice([0.0, 0.005, 0.015, rng.uniform(0, 1)]))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1, 1))
            fz.preset_f64('F', 0x64e8, rng.uniform(-89, 89))
            fz.preset_f64('F', 0x64f0, rng.uniform(-179, 179))
            fz.preset_f64('F', 0x64f8, rng.uniform(-100, 3000))
            case.emu.uc.reg_write(UC_X86_REG_R11, 0)
        if BLOCK == 'gdrag':
            def wheel_state(call, rng):
                for pointer in (call.ints[3], call.stack[2], call.stack[5]):
                    call.put_f32(pointer, rng.uniform(-1, 1))
            case.stub(0x1411ddfd0, wheel_state)
            case.stub(0x14119e5b0, lambda call, rng: None)
            case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.01, 0.2, 2.0, rng.uniform(0, 1)])))
            G = case.region('G', 0x88 * 10)
            X = case.region('X', 0x90 * 10)
            O = case.region('O', 0x100 * 10)
            for base, off, ptr in ((B, 0x6080, G), (F, 0x6958, X)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('F', 0x6960, (X + 0x90 * 10) & 0xffffffff, record=True)
            fz.preset('F', 0x6964, (X + 0x90 * 10) >> 32, record=True)
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            for i in range(10):
                fz.preset('G', 0x88 * i, rng.choice([0, 1, 2, 3, 3]))
                for off in (0x58, 0x5c, 0x18):
                    fz.preset_f32('G', 0x88 * i + off, rng.uniform(0.05, 1.5))
                fz.preset_f32('X', 0x90 * i + 0x10, rng.choice([-0.5, 0.0, rng.uniform(0.05, 1.0), 1.0]))
                for off in (0x14, 0x18):
                    fz.preset_f32('X', 0x90 * i + off, rng.uniform(-90, 90))
                p = O + 0x100 * i
                fz.preset('X', 0x90 * i, p & 0xffffffff, record=True)
                fz.preset('X', 0x90 * i + 4, p >> 32, record=True)
                for off in (0x18, 0x20, 0x64, 0x70, 0x7c):
                    fz.preset_f32('O', 0x100 * i + off, rng.uniform(-2, 3))
            fz.preset_f32('B', 0x2890, rng.choice([0.0, 0.005, rng.uniform(0.5, 50), rng.uniform(0.5, 50)]))
            fz.preset_f32('B', 0x281c, rng.choice([0.0, 0.5, rng.uniform(0.5, 50)]))
            fz.preset('B', 0x28dc, rng.choice([0, 1, 1]))
            fz.preset('B', 0x28e0, rng.choice([0, 1]))
            fz.preset('F', 0x64d8, rng.choice([0, 1, 1]))
            fz.preset('F', 0xbcd0, 0)
            fz.preset('F', 0xbcc8, 0)
            fz.preset_f32('F', 0x28c, rng.choice([0.0, rng.uniform(0, 50), rng.uniform(0, 5), 49.0]))
            case.emu.uc.reg_write(UC_X86_REG_XMM11, struct.unpack('<Q', struct.pack('<d', 0.01))[0])
            case.emu.uc.reg_write(UC_X86_REG_RDI, 0)
        if BLOCK == 'reset':
            case.stub(0x14120c960, lambda call, rng: call.ret_int(0))
            case.stub(0x1409830b0, lambda call, rng: None)
            case.stub(0x1412763c0, lambda call, rng: None)
            fz.preset('F', 0x675c, rng.choice([0, 0, 1]))
            for off in range(0x294, 0x340, 4):
                fz.preset_f32('F', off, rng.uniform(-100, 100))
        if BLOCK == 'vec':
            IDS = case.region('IDS', 0x40)
            OBJ = case.region('OBJ', 0x200 * 4)
            SL = case.region('SL', 0x40)
            REC = case.region('REC', 0x3800 * 4)
            def collect(call, rng):
                n = rng.choice([0, 1, 2, 3, 4])
                for k in range(n):
                    call.put(IDS + 4 * k, rng.randrange(0, 8))
                for pointer, value in ((call.stack[5], IDS), (call.stack[5] + 8, IDS + 4 * n)):
                    call.put(pointer, value & 0xffffffff)
                    call.put(pointer + 4, value >> 32)
            case.stub(0x1411bbfb0, collect)
            case.stub(0x1407d6e70, lambda call, rng: call.ret_int(OBJ + 0x200 * (call.ints[1] & 3)))
            case.stub(0x1411b9460, lambda call, rng: call.ret_int(SL + 8 * ((call.ints[0] - OBJ) // 0x200)))
            case.stub(0x140f44180, lambda call, rng: call.ret_int(rng.getrandbits(16)))
            for addr in (0x140f39780, 0x1407bfac0, 0x140601360, 0x1411b10a0):
                case.stub(addr, lambda call, rng: None)
            for k in range(4):
                ptr = REC + 0x3800 * k
                fz.preset('SL', 8 * k, ptr & 0xffffffff, record=True)
                fz.preset('SL', 8 * k + 4, ptr >> 32, record=True)
                fz.preset('REC', 0x3800 * k + 0x36e8, rng.choice([1, 2, 2, 2]))
                fz.preset('REC', 0x3800 * k + 0x36ec, rng.choice([1, 2, 2]))
                fz.preset('OBJ', 0x200 * k + 0xfc, rng.getrandbits(32))
            fz.preset('F', 0x28, rng.choice([0, 0, 0, 1]))
            fz.preset('F', 0xb9a8, rng.choice([0, 0, 1]))
            for reg, val in ((UC_X86_REG_RDI, 0), (UC_X86_REG_R13, 1), (UC_X86_REG_RBX, F + 0xbcc8)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'strips':
            VEC = case.region('VEC', 0x80)
            PR = case.region('PR', 0x3770 * 3)
            case.stub(0x1411d9f60, lambda call, rng: call.ret_int(rng.choice([0, 1, 1])))
            case.stub(0x1411b4730, lambda call, rng: call.ret_int(VEC + 0x20 * (call.ints[2] & 3)))
            case.stub(0x1405f3f30, lambda call, rng: call.ret_int(call.ints[1] + 0x10))
            case.stub(0x14120c960, lambda call, rng: call.ret_int(rng.choice([0, 0, 0x100])) if False else call.ret_int(0))
            for addr in (0x140985d90, 0x14121a9b0, 0x1405ddb90):
                case.stub(addr, lambda call, rng: None)
            fz.preset('B', 0x920, rng.choice([0, 1, 2, 3, 3]))
            fz.preset('B', 0x6010, PR & 0xffffffff, record=True)
            fz.preset('B', 0x6014, PR >> 32, record=True)
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            for p in range(3):
                fz.preset('PR', 0x3770 * p + 4, rng.choice([0, 1, 1]))
                fz.preset('PR', 0x3770 * p + 0x8c, rng.choice([0, 1, 2, 3, 5]))
                fz.preset_f32('PR', 0x3770 * p + 0xa0, rng.uniform(-3, 3))
            for k in range(4 * 8):
                fz.preset_f32('VEC', 4 * k, rng.uniform(-5, 5))
            for reg, val in ((UC_X86_REG_RDI, 0), (UC_X86_REG_R13, 1), (UC_X86_REG_R14, 0), (UC_X86_REG_RBX, F + 0xbcc8)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'wstrips':
            W = case.region('W', 0x36c8 * 48)
            XR = case.region('XR', 0x2d8 * 48)
            IDS = case.region('IDS', 0x40 * 48)
            case.stub(0x1411da150, lambda call, rng: call.ret_int(rng.choice([0, 1, 1, 1])))
            case.stub(0x1405f3f30, lambda call, rng: call.ret_int(call.ints[1] + 0x10))
            case.stub(0x14120c960, lambda call, rng: call.ret_int(0))
            for addr in (0x140985d90, 0x14121a9b0, 0x1405ddb90):
                case.stub(addr, lambda call, rng: None)
            for base, off, ptr in ((B, 0x6028, W), (F, 0x6940, XR)):
                name = {F: 'F', B: 'B'}[base]
                fz.preset(name, off, ptr & 0xffffffff, record=True)
                fz.preset(name, off + 4, ptr >> 32, record=True)
            fz.preset('F', 0x20, B & 0xffffffff, record=True)
            fz.preset('F', 0x24, B >> 32, record=True)
            for w in range(48):
                n = rng.choice([0, 1, 2, 3, 5])
                p = IDS + 0x40 * w
                fz.preset('W', 0x36c8 * w + 0x3690, p & 0xffffffff, record=True)
                fz.preset('W', 0x36c8 * w + 0x3694, p >> 32, record=True)
                q = p + 4 * n
                fz.preset('W', 0x36c8 * w + 0x3698, q & 0xffffffff, record=True)
                fz.preset('W', 0x36c8 * w + 0x369c, q >> 32, record=True)
                for k in range(n):
                    fz.preset('IDS', 0x40 * w + 4 * k, rng.randrange(0, 48))
            for reg, val in ((UC_X86_REG_RDI, 0), (UC_X86_REG_R13, 0), (UC_X86_REG_R14, 0), (UC_X86_REG_RBX, F + 0xbcc8)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'tail':
            hit_rate = rng.choice([0.0, 0.0, 0.05, 0.3])
            case.stub(0x1417b2e70, lambda call, rng: call.ret_int(rng.choice([1, 0x201, 0x81]) if rng.random() < hit_rate else rng.choice([0, 0x100, 0x200])))
            case.stub(0x1407debf0, lambda call, rng: None)
            case.stub(0x1424e5fc8, lambda call, rng: call.ret_int(rng.getrandbits(16)))
            for address in dirty:
                case.emu.write_u32(address, 0)
            dirty.clear()
            def putg(address, word):
                dirty.append(address)
                case.emu.write_u32(address, word & 0xffffffff)
                extra[address] = word & 0xffffffff
            putg(0x1424e5fc8, 0x1424e5fc8 & 0xffffffff)
            putg(0x1424e5fcc, 0x1424e5fc8 >> 32)
            putg(0x1460b83b0, 2)
            putg(0x1460b83b4, rng.getrandbits(32))
            fz.preset('F', 0x28, rng.getrandbits(32))
            for reg, val in ((UC_X86_REG_RDI, 0), (UC_X86_REG_RSI, 1)):
                case.emu.uc.reg_write(reg, val)
        if BLOCK == 'angles':
            for off in (0x3f4, 0x3f8, 0x3fc, 0x368, 0x370):
                fz.preset_f32('F', off, rng.choice([rng.uniform(-60, 60), rng.uniform(-0.01, 0.01), 0.0]))
            for off in (0x404, 0x408):
                fz.preset_f32('F', off, rng.uniform(-200, 200))
            case.emu.uc.reg_write(UC_X86_REG_XMM12, 0x3c8efa36)
        if BLOCK == 'velocity':
            for off in (0x368, 0x36c, 0x370):
                fz.preset_f32('F', off, rng.choice([rng.uniform(-60, 60), rng.uniform(-0.5, 0.5), 0.0]))
            for off in (0x430, 0x434, 0x440, 0x444, 0x450, 0x454):
                fz.preset_f32('F', off, rng.uniform(-1.0, 1.0))
        if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate', 'velocity', 'geodetic', 'angles', 'path', 'instruments', 'coeff', 'late', 'arm', 'gear', 'wheels', 'hook', 'tow', 'floats', 'waves', 'pull', 'gtarget', 'steer', 'gstate', 'wcontact', 'wprobe', 'bsurf', 'bblend', 'bloop', 'ground', 'gdrag', 'reset', 'vec', 'strips', 'wstrips', 'tail'):
            S = case.region('S', 0x2000)
            V = case.region('V', 0x4000)
            if BLOCK not in ('motion', 'late', 'arm', 'hook', 'tow', 'floats', 'waves', 'pull', 'gtarget', 'steer', 'gstate', 'wcontact', 'wprobe', 'bsurf', 'bblend', 'bloop', 'ground', 'gdrag', 'reset', 'vec', 'strips', 'wstrips', 'tail'):
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
            if BLOCK in ('strips', 'wstrips', 'tail'):
                fz.preset('S', 0x400 + 0x1750, F & 0xffffffff)
                fz.preset('S', 0x400 + 0x1754, F >> 32)
            if BLOCK == 'ground':
                fz.preset('S', 0x400 - 0x78, rng.choice([0, 1, 1, 1, 1, 1]))
            if BLOCK == 'bblend':
                fz.preset('S', 0x400 + 0x1750, rng.randrange(0, 39))
            if BLOCK in ('bsurf', 'bloop'):
                fz.preset_f32('S', 0x400 - 0x10, rng.uniform(0.1, 3))
                fz.preset_f32('S', 0x400 - 0x8, rng.uniform(0.1, 3))
            if BLOCK in ('gtarget', 'steer', 'gstate', 'wcontact', 'wprobe', 'bsurf'):
                pedal = rng.uniform(-2, 2)
                fz.preset_f32('S', 0x400 + 0x1750, pedal)
                case.emu.uc.reg_write(UC_X86_REG_XMM6, struct.unpack('<I', struct.pack('<f', pedal))[0])
            if BLOCK == 'waves':
                fz.preset('S', 0x400 + 0x1750, F & 0xffffffff)
                fz.preset('S', 0x400 + 0x1754, F >> 32)
            if BLOCK == 'motion':
                for off, lo, hi in ((0x1750, 500.0, 5000.0), (0x1758, 500.0, 5000.0), (-0x78, 300.0, 4000.0)):
                    fz.preset_f32('S', 0x400 + off, rng.uniform(lo, hi))
            case.emu.uc.reg_write(UC_X86_REG_RBP, rbp)
            case.emu.uc.reg_write(UC_X86_REG_XMM15, 0x80000000)
            case.emu.uc.reg_write(UC_X86_REG_XMM11, 0 if BLOCK in ('path', 'instruments') else (0x3f847ae147ae147b if BLOCK == 'gdrag' else 0x3c8efa36))
            case.emu.uc.reg_write(UC_X86_REG_R13, 0xffffffffffffffff)
        case.emu.uc.reg_write(UC_X86_REG_R15, F)
        if BLOCK == 'reset':
            case.emu.uc.reg_write(UC_X86_REG_RCX, F)
        if BLOCK in ('vec', 'strips', 'wstrips', 'tail'):
            case.emu.uc.reg_write(UC_X86_REG_R13, 1)
        if BLOCK in ('strips', 'wstrips', 'tail'):
            case.emu.uc.reg_write(UC_X86_REG_R14, 0)
        case.emu.uc.reg_write(UC_X86_REG_R14, case.body + 0x630 if BLOCK in ('bsurf', 'bblend') else 1 if BLOCK == 'motion' else (F + 0x6548 if BLOCK == 'hook' else (1 if BLOCK == 'floats' else 0)))
        case.emu.uc.reg_write(UC_X86_REG_R12, 0x1460e9708 if BLOCK == 'steer' else 0)
        if BLOCK not in ('integrate', 'geodetic', 'angles', 'path', 'instruments', 'coeff', 'late', 'arm', 'hook'):
            case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        case.emu.uc.reg_write(UC_X86_REG_XMM13, 0)
        case.emu.uc.reg_write(UC_X86_REG_XMM14, struct.unpack('<I', struct.pack('<f', 1.0))[0])
        try:
            case.run(start, until=end, max_instructions=3_000_000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x}' + (f' {rbp:x}' if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate', 'velocity', 'geodetic', 'angles', 'path', 'instruments', 'coeff', 'late', 'arm', 'gear', 'wheels', 'hook', 'tow', 'floats', 'waves', 'pull', 'gtarget', 'steer', 'gstate', 'wcontact', 'wprobe', 'bsurf', 'bblend', 'bloop', 'ground', 'gdrag', 'reset', 'vec', 'strips', 'wstrips', 'tail') else '') + (f' {entry_rsp(0) - 0xc0:x}' if BLOCK == 'integrate' else '') + (f' {case.emu.reg(UC_X86_REG_RSI) & 0xffffffff:x}' if BLOCK == 'arm' else '') + (f' {esi_v:x}' if BLOCK == 'hook' else '') + (f' {int(case.emu.uc.reg_read(UC_X86_REG_RIP) == 0x14126dd98):x}' if BLOCK == 'wprobe' else '') + (f' {case.body:x}' if BLOCK in ('bsurf', 'bblend') else '')
        out = case.dump(header, extra_words=extra)
        if BLOCK in ('element', 'body', 'parts', 'motion', 'integrate', 'velocity', 'geodetic', 'angles', 'path', 'instruments', 'coeff', 'late', 'arm', 'gear', 'wheels', 'hook', 'tow', 'floats', 'waves', 'pull', 'gtarget', 'steer', 'gstate', 'wcontact', 'wprobe', 'bsurf', 'bblend', 'bloop', 'ground', 'gdrag', 'reset', 'vec', 'strips', 'wstrips', 'tail'):
            lines = out.split('\n')
            lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not S <= int(t.split('=')[0], 16) < S + 0x2000)
            out = '\n'.join(lines)
        print(out)
        done += 1


if __name__ == '__main__':
    main()
