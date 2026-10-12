#!/usr/bin/env python3
"""Execute two ORIGINAL wing-element instruction blocks and export synthetic bit vectors.

Uses Unicorn via emulate_xp.py; numpy is not required. No callees are stubbed.
Usage: python tools/gen_wing_vectors.py Xplane12/X-Plane.exe > tests/data/wing_factors.txt
"""
import hashlib
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator
from unicorn.x86_const import (UC_X86_REG_RBP, UC_X86_REG_RSI, UC_X86_REG_XMM6,
                               UC_X86_REG_XMM9, UC_X86_REG_XMM11, UC_X86_REG_XMM12,
                               UC_X86_REG_XMM14, UC_X86_REG_XMM15)

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def set_float(emu, reg, value):
    emu.uc.reg_write(reg, bits(value))


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    wing = emu.alloc(256)
    frame = emu.alloc(512) + 128
    emu.uc.reg_write(UC_X86_REG_RSI, wing)
    emu.uc.reg_write(UC_X86_REG_RBP, frame)
    rng = random.Random(20261007)
    print(f'# reference_sha256 {SHA256}')
    print('# F: aspect efficiency -> flow lift_scale alpha_multiplier (float32 bits)')
    print('# D: factor alpha -> faded_multiplier faded_divisor (float32 bits)')
    aspects = [0.01, 0.1, 1.0, 7.32, 50.0, 100.0]
    efficiencies = [0.01, 0.1, 0.5, 1.0, 2.0, 3.0]
    flow_cases = [(a, e) for a in aspects for e in efficiencies]
    flow_cases += [(rng.uniform(0.01, 100), rng.uniform(0.01, 3)) for _ in range(1000)]
    for aspect, efficiency in flow_cases:
        emu.write_f32(wing + 0x14, aspect)
        emu.set_xmm_f32(0, struct.unpack('<f', emu.read(0x142672ac8, 4))[0])
        set_float(emu, UC_X86_REG_XMM6, efficiency)
        emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        emu.run_range(0x1411b6706, 0x1411b6776)
        multiplier = emu.uc.reg_read(UC_X86_REG_XMM6) & 0xffffffff
        scale = emu.read_u32(frame - 0x54)
        # Record flow separately by stopping before the later flow/scale division.
        emu.set_xmm_f32(0, struct.unpack('<f', emu.read(0x142672ac8, 4))[0])
        set_float(emu, UC_X86_REG_XMM6, efficiency)
        emu.run_range(0x1411b6706, 0x1411b672f)
        flow = emu.uc.reg_read(UC_X86_REG_XMM6) & 0xffffffff
        print('F', *(f'{v:08x}' for v in [bits(aspect), bits(efficiency), flow, scale, multiplier]))
    factors = [-1.5, -0.0, 0.0, 0.1, 0.5, 0.8, 1.0, 1.5, 3.0]
    angles = [-180, -90, -70, -55, -20, -0.0, 0.0, 19.99, 20, 20.01, 55, 70, 89.99, 90, 90.01, 180]
    fade_cases = [(x, a) for x in factors for a in angles]
    fade_cases += [(rng.uniform(-2, 4), rng.uniform(-180, 180)) for _ in range(1000)]
    for factor, alpha in fade_cases:
        emu.set_xmm_f32(3, factor)
        set_float(emu, UC_X86_REG_XMM6, factor)
        set_float(emu, UC_X86_REG_XMM9, alpha)
        set_float(emu, UC_X86_REG_XMM11, 1.0)
        emu.run_range(0x1411b68fc, 0x1411b6979)
        multiplier = emu.uc.reg_read(UC_X86_REG_XMM14) & 0xffffffff
        divisor = emu.uc.reg_read(UC_X86_REG_XMM15) & 0xffffffff
        print('D', *(f'{v:08x}' for v in [bits(factor), bits(alpha), multiplier, divisor]))


if __name__ == '__main__':
    main(sys.argv[1])
