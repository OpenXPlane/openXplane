#!/usr/bin/env python3
"""Replay original geometry writer for Wing+18/+1c on explicit grouped state.

Range14117285b..141172bc6 from geometry initializer141170d40. No stubs.
This does not establish upstream connection/grouping/endpoints selection.
Member z arrays are explicit geometry but this range reads their x/y only;
root/tip z coordinates influence the separately supplied group endpoints.
Usage: python tools/gen_wing_dimensions_vectors.py Xplane12/X-Plane.exe > output.tmp
"""
import hashlib
import math
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator, STACK_TOP
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import (
    UC_X86_REG_RDI, UC_X86_REG_RSI, UC_X86_REG_RBP, UC_X86_REG_RSP,
    UC_X86_REG_XMM0, UC_X86_REG_XMM7, UC_X86_REG_XMM13, UC_X86_REG_XMM12, UC_X86_REG_XMM14,
)

SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
BEGIN = 0x14117285b
AREA_END = 0x141172ae0
MEAN_END = 0x141172af2
END = 0x141172bc6


def bits(value):
    return struct.unpack('<I', struct.pack('<f', value))[0]


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def hx(value):
    return f'{bits(value):08x}'


def make_members(count, elements, taper, angle, rng=None):
    origin = [0., 0., 0.] if rng is None else [rng.uniform(-10., 10.) for _ in range(3)]
    spans = [10.]*count if rng is None else [rng.uniform(.1, 20.) for _ in range(count)]
    total = sum(spans)
    root_chord = 6. if rng is None else rng.uniform(.2, 8.)
    radial = 0.
    members = []
    for span in spans:
        n = elements if rng is None else rng.randint(1, 10)
        sweep = 0. if rng is None else rng.uniform(-40., 40.)
        chord = [];coords = [[], [], []]
        for i in range(n+1):
            u = i/n;position = radial+span*u;fraction = position/total
            c = root_chord*(1.-fraction)+root_chord*taper*fraction
            if rng is not None and 0<i<n:
                c *= rng.uniform(.6, 1.4)
            chord.append(f32(c))
            bend = 0. if rng is None else .1*span*math.sin(math.pi*u)
            coords[0].append(f32(origin[0]+position*math.cos(math.radians(angle))+bend))
            coords[1].append(f32(origin[1]+bend*.7))
            coords[2].append(f32(origin[2]+position*math.sin(math.radians(angle))-bend*.5))
        members.append((n, f32(span), f32(sweep), chord, *coords))
        radial += span
    first = members[0];last = members[-1]
    endpoints = [first[4][0], first[5][0], first[6][0], last[4][-1], last[5][-1], last[6][-1]]
    return members, endpoints


def cases():
    for count in [1, 2, 3]:
        for elements in [1, 4, 10]:
            for taper in [0., .25, .5, 1., 1.5]:
                for angle in [-45., 0., 45.]:
                    yield make_members(count, elements, taper, angle)
    rng = random.Random(20261022)
    for _ in range(500):
        yield make_members(rng.randint(1, 3), 0, rng.uniform(0., 1.8), rng.uniform(-80., 80.), rng)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu = Emulator(path)
    objects = [emu.alloc(0x3700) for _ in range(3)]
    pointer_list = emu.alloc(24)
    frame = emu.alloc(2048)+128
    stack = STACK_TOP-0x2000
    selector = emu.read_u32(0x1463a5090)
    print('# reference_sha256', SHA256)
    print(f'# original_range {BEGIN:#x} {END:#x} exclusive; intermediate guards {AREA_END:#x} {MEAN_END:#x}; no stubs')
    print('# supported explicit nonempty1..3member group,counts1..10,finite positive area/span,nonzero endpoints; upstream grouping not claimed')
    print('# group_area supplied by f32 accumulation of ORIGINAL1411a0080 member element areas; ordinal grouping explicit')
    print('# W: members_count endpoints_xyz_xyz[6] group_area | each member: elements span sweep chord[n+1] x[n+1] y[n+1] z[n+1] | outputs18 1c area_sum weighted_radius_sum mean_radius centroid_fraction length3d member_cos[n_members] projection_cos (float32 bits,counts decimal)')
    recorded_trig = []
    def observe_trig(uc,address,size,user):
        if address in [0x141172945,0x141172b87]:
            recorded_trig.append(uc.reg_read(UC_X86_REG_XMM0)&0xffffffff)
    emu.uc.hook_add(UC_HOOK_CODE,observe_trig)
    count = 0
    for members, endpoints in cases():
        for j, member in enumerate(members):
            n, span, sweep, chord, x, y, z = member
            wing = objects[j]
            emu.write(wing, bytes(0x3700))
            emu.write_u32(wing+4, n)
            emu.write_f32(wing+0x10, span)
            emu.write_f32(wing+0x30, sweep)
            for base, array in [(0x70, chord), (0x5bc, x), (0x5e8, y), (0x614, z)]:
                for i, value in enumerate(array):
                    emu.write_f32(wing+base+i*4, value)
            emu.write_u64(pointer_list+j*8, wing)
        # Explicit input area, independently executing a known original pure helper.
        group_area = 0.
        for wing, member in zip(objects, members):
            for element in range(member[0]):
                area = emu.call_float(0x1411a0080, ints=[wing, element])
                group_area = f32(group_area+area)
        root = objects[0]
        for offset, value in zip([0x36b0,0x36b4,0x36b8,0x36bc,0x36c0,0x36c4],endpoints):
            emu.write_f32(root+offset,value)
        for register, value in [(UC_X86_REG_RDI,root),(UC_X86_REG_RSI,pointer_list+len(members)*8),
                                (UC_X86_REG_RBP,frame),(UC_X86_REG_RSP,stack)]:
            emu.uc.reg_write(register,value)
        emu.uc.reg_write(UC_X86_REG_XMM7,struct.unpack('<Q',struct.pack('<d',.5))[0])
        emu.uc.reg_write(UC_X86_REG_XMM13,0)
        emu.write_u64(frame-0x60,pointer_list)
        emu.write_f32(stack+0x78,group_area)
        recorded_trig.clear()
        emu.run_range(BEGIN,AREA_END)
        area_sum = emu.uc.reg_read(UC_X86_REG_XMM14)&0xffffffff
        weighted_sum = emu.uc.reg_read(UC_X86_REG_XMM12)&0xffffffff
        emu.run_range(AREA_END,MEAN_END)
        mean_radius = emu.uc.reg_read(UC_X86_REG_XMM12)&0xffffffff
        emu.run_range(MEAN_END,END)
        fraction = emu.uc.reg_read(UC_X86_REG_XMM12)&0xffffffff
        output = [emu.read_u32(root+0x18),emu.read_u32(root+0x1c),area_sum,weighted_sum,
                  mean_radius,fraction,emu.read_u32(root+0x36a8)]
        assert len(recorded_trig)==len(members)+1
        assert not emu.stubs
        assert emu.read_u32(0x1463a5090)==selector
        tokens = ['W',str(len(members)),*[hx(v) for v in endpoints],hx(group_area),'|']
        for n,span,sweep,chord,x,y,z in members:
            tokens += [str(n),hx(span),hx(sweep),*[hx(v) for v in chord+x+y+z],'|']
        print(*tokens,*[f'{raw:08x}' for raw in output+recorded_trig])
        count += 1
    print('# cases',count)


if __name__ == '__main__':
    main(sys.argv[1])
