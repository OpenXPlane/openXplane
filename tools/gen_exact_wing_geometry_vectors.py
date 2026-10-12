#!/usr/bin/env python3
"""Replay complete original quarter-chord sweep and sweep/AR/taper helpers.

No callees are stubbed, no instruction replacements, CRT selector unchanged.
Usage: python tools/gen_exact_wing_geometry_vectors.py Xplane12/X-Plane.exe > output.tmp
"""
import hashlib
import math
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator
SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'

def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]
from unicorn.x86_const import UC_X86_REG_XMM0

SWEEP = 0x1411a0230
CORRECTION = 0x1411a00f0
CRT_SELECTOR = 0x1463a5090
OFFSETS = (0x5BC, 0x5E8, 0x614, 0x5C0, 0x5EC, 0x618, 0x70, 0x74, 0x18, 0x1C)


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def near(value):
    """Adjacent binary32 inputs, used around target-angle boundaries."""
    value = f32(value)
    raw = bits(value)
    return [struct.unpack('<f', struct.pack('<I', raw + delta))[0] for delta in (-1, 0, 1)]


def cases():
    rng = random.Random(20261012)
    # Constant equal chords ensure quarter-chord terms cancel. The z inputs are
    # neighboring binary32 values around every indicated angle, not claims that
    # original output equals a mathematically exact degree value.
    for degrees in [-89., -75., -50., -40., -15., -0.01, 0.01, 15., 40., 50., 75., 89.]:
        for z in near(10.0 * math.tan(degrees / 57.2957763671875)):
            for aspect in [0., 1., 4., 8., 16., 25.]:
                for taper in [-1., -0., 0., .25, .5, 1., 1.5]:
                    yield (0., 0., 0., 10., 0., z, 1., 1., aspect, taper)
    # Zero transverse radius and signed-zero coordinates/chords.
    for zero in [-0., 0.]:
        for delta_z in [-10., -0., 0., 10.]:
            for chords in [(0., 0.), (-0., 0.), (1., 1.), (1., 3.), (-1., 1.)]:
                yield (zero, zero, zero, zero, zero, delta_z, *chords, 8., .5)
    # Translation, unequal quarter chords, small/large transverse scales.
    for scale in [.0001, .01, 1., 100., 10000.]:
        for sign in [-1., 1.]:
            yield (4., -3., 2., 4.+scale, -3.+scale, 2.+sign*scale, .3, 4., 7.32, .5)
    for _ in range(2500):
        start = [rng.uniform(-50., 50.) for _ in range(3)]
        end = [rng.uniform(-50., 50.) for _ in range(3)]
        chords = [rng.uniform(-1., 10.) for _ in range(2)]
        yield (*start, *end, *chords, rng.uniform(0., 25.), rng.uniform(-1., 2.))


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    wing = emu.alloc(0x1000)
    selector = emu.read_u32(CRT_SELECTOR)
    print('# reference_sha256', SHA256)
    print(f'# original_helpers {SWEEP:#x} {CORRECTION:#x}; CRT_selector {selector}; no stubs')
    print('# G: start_xyz[3] end_xyz[3] chord0 chord1 raw_18 raw_1c -> sweep_degrees correction (float32 bits)')
    count = 0
    for values in cases():
        values = [f32(value) for value in values]
        for offset, value in zip(OFFSETS, values):
            emu.write_f32(wing + offset, value)
        outputs = []
        for address in [SWEEP, CORRECTION]:
            emu.call(address, ints=[wing, 0])
            outputs.append(emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff)
        assert not emu.stubs
        assert emu.read_u32(CRT_SELECTOR) == selector
        print('G', *(f'{raw:08x}' for raw in [*[bits(value) for value in values], *outputs]))
        count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
