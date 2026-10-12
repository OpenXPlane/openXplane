#!/usr/bin/env python3
"""Replay the original delta-wing final blending block, without calls or stubs.

Usage: python tools/gen_delta_blend_vectors.py Xplane12/X-Plane.exe > output.tmp
Boundary 0x1411b8779..0x1411b885a (exclusive endpoint) of 0x1411b6630.
"""
import hashlib
import itertools
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator, STACK_TOP
from unicorn.x86_const import (
    UC_X86_REG_RBP, UC_X86_REG_RSP, UC_X86_REG_R12, UC_X86_REG_R13,
    UC_X86_REG_R14, UC_X86_REG_R15, UC_X86_REG_RSI,
    UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3,
    UC_X86_REG_XMM4, UC_X86_REG_XMM8, UC_X86_REG_XMM9, UC_X86_REG_XMM11,
    UC_X86_REG_XMM12, UC_X86_REG_XMM14, UC_X86_REG_XMM15,
)

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN = 0x1411b8779
END = 0x1411b885a


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def from_bits(value):
    return struct.unpack('<f', struct.pack('<I', value))[0]


def cases():
    # Signed zero in all coefficients, including same and reversed bounds.
    for a, b, weight in itertools.product([-0.0, 0.0], [-0.0, 0.0], [-1.0, -0.0, 0.0, 1.0, 2.0]):
        yield [a] * 4, [b] * 4, weight
    for base, target in [(-2.0, 3.0), (3.0, -2.0), (1.0, 1.0), (-1.0, -1.0)]:
        for weight in [-1.0, -0.0, 0.0, .25, .5, 1.0, 2.0]:
            yield [base] * 4, [target] * 4, weight
    # Every side of the exact stall thresholds, with interpolation and clamping.
    ratios = [from_bits(raw) for raw in [0x3f7fffff, 0x3f800000, 0x3f800001,
                                         0xbf7fffff, 0xbf800000, 0xbf800001]]
    for base, target, weight in itertools.product(ratios, ratios, [-1.0, 0.0, .5, 1.0, 2.0]):
        yield [.3, .04, -.02, base], [1.1, .6, .1, target], weight
    rng = random.Random(20261016)
    for _ in range(2000):
        yield [rng.uniform(-5., 5.) for _ in range(4)], [rng.uniform(-5., 5.) for _ in range(4)], rng.uniform(-1., 2.)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    frame = emu.alloc(1024) + 128
    output = emu.alloc(16)
    wing = emu.alloc(0x400)
    index = 2
    print('# reference_sha256', SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; no calls/stubs; finite inputs only')
    print('# B: base_cl base_cd base_cm base_ratio target_cl target_cd target_cm target_ratio weight -> cl cd cm ratio stalled (hex bits/u32)')
    count = 0
    for base, target, weight in cases():
        for register, value in [(UC_X86_REG_RBP, frame), (UC_X86_REG_RSP, STACK_TOP-0x1000),
                                (UC_X86_REG_R12, wing), (UC_X86_REG_R13, index),
                                (UC_X86_REG_R14, output), (UC_X86_REG_R15, output+4),
                                (UC_X86_REG_RSI, output+8)]:
            emu.uc.reg_write(register, value)
        for register, value in [(UC_X86_REG_XMM0, base[0]), (UC_X86_REG_XMM1, target[0]),
                                (UC_X86_REG_XMM3, base[0]), (UC_X86_REG_XMM4, target[0]),
                                (UC_X86_REG_XMM12, base[1]), (UC_X86_REG_XMM15, base[2]),
                                (UC_X86_REG_XMM8, base[3]), (UC_X86_REG_XMM9, target[3]),
                                (UC_X86_REG_XMM2, weight), (UC_X86_REG_XMM11, 1.0)]:
            emu.uc.reg_write(register, bits(value))
        # andps uses all 128 bits but only the low float is subsequently consumed.
        emu.uc.reg_write(UC_X86_REG_XMM14, 0x7fffffff)
        emu.write_f32(frame+0x168, target[1])
        emu.write_f32(frame+0x170, target[2])
        emu.run_range(BEGIN, END)
        assert not emu.stubs
        inputs = [bits(value) for value in [*base, *target, weight]]
        results = [emu.read_u32(output+offset) for offset in [0, 4, 8]]
        results += [emu.read_u32(wing+0x1bc+index*4), emu.read_u32(wing+0x1e4+index*4)]
        print('B', *(f'{value:08x}' for value in inputs+results))
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
