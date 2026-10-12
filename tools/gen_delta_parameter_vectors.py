#!/usr/bin/env python3
"""Replay original three-curve delta-wing prefix, with original CRT pow callees.

Usage: python tools/gen_delta_parameter_vectors.py Xplane12/X-Plane.exe > output.tmp
No stubs or replaced instructions; exact original endpoint is required.
"""
import hashlib
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator, STACK_TOP
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_RSP, UC_X86_REG_XMM9, UC_X86_REG_XMM10

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN = 0x1411b7efa
END = 0x1411b7fbd


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def from_bits(raw):
    return struct.unpack('<f', struct.pack('<I', raw))[0]


def cases():
    for value in [45., 75.]:
        raw = bits(value)
        yield from [from_bits(raw-1), value, from_bits(raw+1)]
    yield from [-1000000., -1000., -360., -90., -75., -45., -1., -0., 0., 1., 30.,
                44., 46., 50., 60., 74., 76., 90., 360., 1000., 1000000.]
    rng = random.Random(20261017)
    for index in range(2000):
        yield rng.uniform(45., 75.) if index % 2 else rng.uniform(-1000., 1000.)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    frame = emu.alloc(1024)+128
    selector = emu.read_u32(0x1463a5090)
    print('# reference_sha256', SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; no stubs; CRT_selector {selector} unchanged')
    print('# P: sweep -> slope_178 peak_alpha_158 peak_lift_180 (float32 bits)')
    count = 0
    for sweep in cases():
        emu.uc.reg_write(UC_X86_REG_RBP, frame)
        emu.uc.reg_write(UC_X86_REG_RSP, STACK_TOP-0x1000)
        # Prefix ends after a compare whose flags do not affect its three outputs.
        emu.uc.reg_write(UC_X86_REG_XMM9, 0)
        emu.uc.reg_write(UC_X86_REG_XMM10, 0)
        emu.write_f32(frame-0x58, sweep)
        emu.run_range(BEGIN, END)
        assert not emu.stubs
        assert emu.read_u32(0x1463a5090) == selector
        outputs = [emu.read_u32(frame+offset) for offset in [0x178, 0x158, 0x180]]
        print('P', *(f'{raw:08x}' for raw in [bits(sweep), *outputs]))
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
