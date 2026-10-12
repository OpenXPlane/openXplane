#!/usr/bin/env python3
"""Replay original six-octave 2D noise with explicitly supplied synthetic table.

Usage: python tools/gen_noise2_vectors.py Xplane12/X-Plane.exe > output.tmp
Table is deterministic test input, not claimed original runtime initialization.
"""
import hashlib
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator
from unicorn.x86_const import UC_X86_REG_XMM0

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
FUNCTION = 0x1411e1940
TABLE = 0x14578f1f0
TABLE_LEN = 0x40000


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def from_bits(raw):
    return struct.unpack('<f', struct.pack('<I', raw))[0]


def table_bytes():
    # uint32 wrapping affine hash; the upper24 bits convert exactly to binary32.
    values = [(((index*1664525+1013904223) & 0xffffffff) >> 8) / 16777216.0
              for index in range(TABLE_LEN)]
    return struct.pack(f'<{TABLE_LEN}f', *values)


def cases():
    seeds = [0, 1, -1, 100, -100, 2147483647, -2147483648]
    points = [-1., -0., 0., .5, 1., 2.]
    for x in points:
        for y in points:
            for seed in seeds:
                yield x, y, seed
    for boundary in [1., 512., 262143., 262144.]:
        raw = bits(boundary)
        adjacent = [from_bits(raw-1), boundary, from_bits(raw+1)]
        for x in adjacent:
            for y in adjacent:
                for seed in seeds:
                    yield x, y, seed
                    yield -x, -y, seed
    rng = random.Random(20261018)
    for _ in range(1000):
        yield rng.uniform(-4000., 4000.), rng.uniform(-4000., 4000.), rng.randrange(-2147483648, 2147483648)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    raw_table = table_bytes()
    emu.write(TABLE, raw_table)
    print('# reference_sha256', SHA256)
    print(f'# original_function {FUNCTION:#x}; table_address {TABLE:#x}; table_len {TABLE_LEN}; no stubs')
    print('# table_recipe f32(((u32(index)*1664525+1013904223).wrapping_u32()>>8)/16777216.0)')
    print('# table_sha256_le_f32', hashlib.sha256(raw_table).hexdigest())
    print('# frequencies x/y simultaneous: 1,2,4,8,16,32; double weights 1,.5,.25,.125,.0625,.03125')
    print('# N: x y seed_i32_bits -> noise_f32_bits')
    count = 0
    for x, y, seed in cases():
        emu.call(FUNCTION, floats=[x, y], ints=[0, 0, seed])
        assert not emu.stubs
        result = emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff
        print('N', *(f'{raw:08x}' for raw in [bits(x), bits(y), seed & 0xffffffff, result]))
        count += 1
    assert emu.read(TABLE, len(raw_table)) == raw_table
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
