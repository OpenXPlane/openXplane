#!/usr/bin/env python3
"""Replay full original delta coefficient target range with explicit synthetic state.

Usage: python tools/gen_delta_target_vectors.py Xplane12/X-Plane.exe > output.tmp
No profile stubs, no replaced instructions. Diagnostic flags zero; finite bounded
inputs, v/span positive. Noise table is synthetic controlled input, not inferred
runtime initialization. Thickness is the original midpoint foil value.
"""
import hashlib
import math
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator, STACK_TOP
from gen_noise2_vectors import SHA256, TABLE, table_bytes
from unicorn.x86_const import (
    UC_X86_REG_RBP, UC_X86_REG_RSP, UC_X86_REG_RSI, UC_X86_REG_RDI,
    UC_X86_REG_R12, UC_X86_REG_R13, UC_X86_REG_R14, UC_X86_REG_R15,
    UC_X86_REG_XMM0, UC_X86_REG_XMM9, UC_X86_REG_XMM10,
    UC_X86_REG_XMM11, UC_X86_REG_XMM12,
)

BEGIN = 0x1411b7efa
END = 0x1411b8779
TIME = 0x142f01918
RADIANS = struct.unpack('<f', bytes.fromhex('36fa8e3c'))[0]


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def from_bits(raw):
    return struct.unpack('<f', struct.pack('<I', raw))[0]


def nearby(value):
    raw = bits(value)
    return [from_bits(raw-1), f32(value), from_bits(raw+1)]


def peak_alpha(sweep):
    t = max(0., min(1., f32(f32(f32(1./30.)*f32(sweep-45.))+0.)))
    power = f32(math.pow(t, .75))
    return f32(f32(15.*f32(power-0.))+25.)


def cases():
    # Supported values keep normalization/seed conversions finite and within i32.
    for sweep in [40., 45., 50., 60., 75., 80.]:
        angles = [-180., -0., 0., 180.]
        for boundary in [peak_alpha(sweep), 60., 90.]:
            angles += nearby(boundary)
            angles += [-x for x in nearby(boundary)]
        for alpha in angles:
            for time in [0., .125, 1.234567890123]:
                yield (alpha, sweep, 7.32, .8, .12, .05, .015, -.025, time, -3., 4., 10., 12.)
    rng = random.Random(20261019)
    for _ in range(1500):
        yield (rng.uniform(-180., 180.), rng.uniform(35., 85.), rng.uniform(.2, 15.),
               rng.uniform(.2, 1.5), rng.uniform(.01, .3), *[rng.uniform(-.3, .3) for _ in range(3)],
               rng.uniform(-20., 20.), rng.uniform(-10., 10.), rng.uniform(-20., 20.),
               rng.uniform(1., 40.), rng.uniform(-20., 20.))


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    wing = emu.alloc(0x4000)
    flow = emu.alloc(0xc000)
    aircraft = emu.alloc(0x7000)
    state = emu.alloc(0x400)
    frame = emu.alloc(2048)+128
    output = emu.alloc(16)
    foils = [emu.alloc(0x100) for _ in range(3)]
    index = 2
    emu.write_u64(flow+0x20, aircraft)
    emu.write_u32(wing+4, 8)
    emu.write_f32(wing+0x60, .25)
    emu.write_f32(wing+0x64, .75)
    for i, foil in enumerate(foils):
        emu.write_u64(wing+0x3678+i*8, foil)
    table = table_bytes()
    emu.write(TABLE, table)
    selector = emu.read_u32(0x1463a5090)
    print('# reference_sha256', SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; no stubs; diagnostics zero; CRT_selector {selector} unchanged')
    print('# table_recipe f32(((u32(index)*1664525+1013904223).wrapping_u32()>>8)/16777216.0)')
    print('# table_sha256_le_f32', hashlib.sha256(table).hexdigest())
    print('# supported finite alpha[-180,180],sweep[35,85],v/span>0,bounded coordinates/seeds; time explicit f64')
    print('# T: alpha sweep v k thickness extra_cl extra_cd extra_cm time_f64bits x z span zref cosf_alpha sinf_abs_alpha -> cl cd cm ratio (others f32bits)')
    count = 0
    for original in cases():
        # Preserve the explicit double time; narrow every other input as loaded by original code.
        values = [value if i == 8 else f32(value) for i, value in enumerate(original)]
        alpha, sweep, v, k, thickness, ex_cl, ex_cd, ex_cm, time, x, z, span, zref = values
        for register, value in [(UC_X86_REG_RBP, frame), (UC_X86_REG_RSP, STACK_TOP-0x2000),
                                (UC_X86_REG_RSI, wing), (UC_X86_REG_RDI, flow),
                                (UC_X86_REG_R12, state), (UC_X86_REG_R13, index),
                                (UC_X86_REG_R14, output), (UC_X86_REG_R15, output+4)]:
            emu.uc.reg_write(register, value)
        for register, value in [(UC_X86_REG_XMM9, alpha), (UC_X86_REG_XMM10, 0.), (UC_X86_REG_XMM11, 1.)]:
            emu.uc.reg_write(register, bits(value))
        emu.uc.reg_write(UC_X86_REG_XMM12, struct.unpack('<Q', struct.pack('<d', 1.))[0])
        emu.write_u64(frame+0x188, output+8)
        for offset, value in [(-0x58, sweep), (-0x5c, v), (-0x54, k), (0x130, .7),
                              (0x160, ex_cl), (0x168, ex_cd), (0x170, ex_cm)]:
            emu.write_f32(frame+offset, value)
        for foil in foils:
            emu.write_f32(foil+0x58, thickness)
        emu.write_f32(wing+0x5bc+index*4, x)
        emu.write_f32(wing+0x614+index*4, z)
        emu.write_f32(aircraft+0x64f4, span)
        emu.write_f32(aircraft+0x64fc, zref)
        emu.write(TIME, struct.pack('<d', time))
        emu.run_range(BEGIN, END)
        results = [emu.read_u32(frame+offset) for offset in [0x140, 0x168, 0x170]]
        results.append(emu.uc.reg_read(UC_X86_REG_XMM9) & 0xffffffff)
        # Independent ORIGINAL trig replay, to distinguish platform CRT differences.
        endpoints = []
        for address, angle in [(0x14230b380, alpha), (0x14230be80, abs(alpha))]:
            emu.call(address, floats=[f32(angle*RADIANS)])
            endpoints.append(emu.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff)
        assert not emu.stubs
        assert emu.read_u32(0x1463a5090) == selector
        fields = [f'{struct.unpack("<Q", struct.pack("<d", value))[0]:016x}' if i == 8 else f'{bits(value):08x}'
                  for i, value in enumerate(values)]
        print('T', *fields, *(f'{word:08x}' for word in endpoints+results))
        count += 1
    assert emu.read(TABLE, len(table)) == table
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
