#!/usr/bin/env python3
"""Starting point for running the engine update 0x141197b00 of the reference build in the emulator.

What it needs (found by trial, see research/ENGINE.md): the flight object F with the engine array at
F+0x68b0 and the aircraft object B at F+0x20 holding the engine descriptors (+0x5ff8, stride 0x68) and
wings (+0x6010, stride 0x3770); the counts B+0x91c and B+0x920 small; the diagnostic flags at +0xbcc8 and
+0xbcd0 zero; and stubs for the callees that depend on engine state: the atmosphere accessors 0x141ba6750
and 0x141ba64e0 (they read a runtime table), the engine flag test 0x1417f12c0 and the frame time 0x140c448c0
(it reads thread-local storage). With those the function runs to its end on random floats.

    python3 tools/engine_harness.py Xplane12/X-Plane.exe
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_XMM0  # noqa: E402

FUNC = 0x141197b00


def build(emu, rng):
    F, B = emu.alloc(0x10000), emu.alloc(0x8000)
    eng, desc, wings = emu.alloc(0x2cc * 16), emu.alloc(0x68 * 16), emu.alloc(0x3770 * 4)
    for obj, size in ((F, 0x10000), (B, 0x8000), (eng, 0x2cc * 16), (desc, 0x68 * 16), (wings, 0x3770 * 4)):
        emu.write(obj, b''.join(struct.pack('<f', rng.uniform(0.2, 2.0)) for _ in range(size // 4)))
    emu.write_u64(F + 0x20, B)
    emu.write_u64(F + 0x68b0, eng)
    emu.write_u64(B + 0x5ff8, desc)
    emu.write_u64(B + 0x6010, wings)
    emu.write_u32(B + 0x91c, 1)
    emu.write_u32(B + 0x920, 1)
    for i in range(16):
        emu.write_u32(desc + 0x68 * i, 1)
    for obj in (F, B):
        emu.write_u32(obj + 0xbcc8, 0)
        emu.write_u32(obj + 0xbcd0, 0)
    return F, B, eng


def install_stubs(emu, log):
    def returning(name, value=None, integer=None):
        def stub(e):
            log.append((name, e.reg(UC_X86_REG_RCX)))
            if value is not None:
                e.set_xmm_f32(0, value)
            if integer is not None:
                e.uc.reg_write(UC_X86_REG_RAX, integer)
        return stub

    emu.stubs[0x141ba6750] = returning('atmosphere_1', 101325.0)
    emu.stubs[0x141ba64e0] = returning('atmosphere_2', 1.0)
    emu.stubs[0x1417f12c0] = returning('engine_flag', integer=0)

    def frame_time(e):
        log.append(('frame_time', 0))
        e.uc.reg_write(UC_X86_REG_XMM0, struct.unpack('<Q', struct.pack('<d', 0.005))[0])

    emu.stubs[0x140c448c0] = frame_time


def main():
    emu = Emulator(sys.argv[1])
    log = []
    install_stubs(emu, log)
    F, B, eng = build(emu, random.Random(1))
    emu.call(FUNC, ints=[eng, F, 0, 0])
    print('returned', emu.xmm0_f32(), 'stub calls', [n for n, _ in log])


if __name__ == '__main__':
    main()
