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
from xp_vmcase import VmCase  # noqa: E402
from unicorn.x86_const import (UC_X86_REG_R12, UC_X86_REG_R14, UC_X86_REG_R15, UC_X86_REG_RBP, UC_X86_REG_XMM11,  # noqa: E402
                               UC_X86_REG_XMM12, UC_X86_REG_XMM13, UC_X86_REG_XMM14, UC_X86_REG_XMM15)

EXE, BLOCK, TRIALS, SEED = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
BLOCKS = {'aspect': (0x141265f7d, 0x14126644a), 'thrust': (0x14126644a, 0x141266b52), 'element': (0x141266b52, 0x141267978)}
SIM_TIME = 0x142f01918
RANGES = [(0x141265000, 0x141275000), (0x1411d0000, 0x1411e0000), (0x141210000, 0x141220000), (0x141290000, 0x1412a0000),
          (0x140860000, 0x140870000), (0x1406e0000, 0x1406f0000), (0x1411a0000, 0x1411d0000), (0x140910000, 0x140911000),
          (0x141220000, 0x141230000), (0x140810000, 0x140820000), (0x1407d0000, 0x1407e0000), (0x1408b0000, 0x1408c0000),
          (0x140900000, 0x140910000), (0x140620000, 0x140630000), (0x140f20000, 0x140f30000), (0x141170000, 0x141180000)]
RECORD_VECTOR = 0x146125768
RECORDING_ID = 0x142f2e3dc


def main():
    start, end = BLOCKS[BLOCK]
    case = VmCase(EXE, SEED, RANGES)
    case.stub(0x1407ace10, lambda call, rng: call.ret_int(rng.randrange(2) if rng.random() < 0.2 else 0))
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.randrange(2)))
    case.stub(0x141260090, lambda call, rng: None)

    def airflow(call, rng):
        call.put_f32(call.ints[2], rng.uniform(-50, 50))
        call.put_f32(call.stack[0], rng.uniform(-50, 50))
        call.put_f32(call.stack[2], rng.uniform(-50, 50))

    def element_force(call, rng):
        for pointer in (call.stack[2], call.stack[3], call.stack[4]):
            call.put_f32(pointer, rng.uniform(-1, 1))

    case.stub(0x14121b580, airflow)
    case.stub(0x1411b9840, element_force)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.uniform(0.005, 0.06)))
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
        if BLOCK == 'element':
            S = case.region('S', 0x2000)
            V = case.region('V', 0x4000)
            fz.preset('F', 0x28, rng.choice([7, 12345]))
            for w in range(48):
                fz.preset('W', 0x36c8 * w + 0x58, rng.choice([0, 1]))
            case.emu.write_u32(RECORDING_ID, 12345)
            for address, value in ((RECORD_VECTOR + 8, V), (RECORD_VECTOR + 16, V + 0x4000)):
                case.emu.write_u32(address, value & 0xffffffff)
                case.emu.write_u32(address + 4, value >> 32)
            extra = {RECORDING_ID: 12345}
            for address, value in ((RECORD_VECTOR + 8, V), (RECORD_VECTOR + 16, V + 0x4000)):
                extra[address] = value & 0xffffffff
                extra[address + 4] = value >> 32
            rbp = S + 0x400
            case.emu.uc.reg_write(UC_X86_REG_RBP, rbp)
            case.emu.uc.reg_write(UC_X86_REG_XMM15, 0x80000000)
            case.emu.uc.reg_write(UC_X86_REG_XMM11, 0x3c8efa36)
        case.emu.uc.reg_write(UC_X86_REG_R15, F)
        case.emu.uc.reg_write(UC_X86_REG_R14, 0)
        case.emu.uc.reg_write(UC_X86_REG_R12, 0)
        case.emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        case.emu.uc.reg_write(UC_X86_REG_XMM13, 0)
        case.emu.uc.reg_write(UC_X86_REG_XMM14, struct.unpack('<I', struct.pack('<f', 1.0))[0])
        try:
            case.run(start, until=end, max_instructions=3_000_000)
        except RuntimeError as err:
            sys.stderr.write(f'trial failed: {err}\n')
            continue
        header = f'{F:x}' + (f' {rbp:x}' if BLOCK == 'element' else '')
        out = case.dump(header, extra_words=extra)
        if BLOCK == 'element':
            lines = out.split('\n')
            lines[-1] = 'O ' + ' '.join(t for t in lines[-1].split()[1:] if not S <= int(t.split('=')[0], 16) < S + 0x2000)
            out = '\n'.join(lines)
        print(out)
        done += 1


if __name__ == '__main__':
    main()
