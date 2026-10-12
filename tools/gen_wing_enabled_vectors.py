#!/usr/bin/env python3
"""Replay original Wing+678 enable writer,141ab17d0..141ab1816(no stubs).

Inputs are metres after native ACF unit conversion; span/root/tip widened to
f64 and compared strictly with double .001. Metadata identifies scaled-feet
boundary cases. Usage:python tools/gen_wing_enabled_vectors.py Xplane12/X-Plane.exe
"""
import hashlib
import itertools
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator
from unicorn.x86_const import UC_X86_REG_R13,UC_X86_REG_RAX,UC_X86_REG_RSI,UC_X86_REG_XMM9
SHA256='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'


def f32(value):
    return struct.unpack('<f',struct.pack('<f',value))[0]


def bits(value):
    return struct.unpack('<I',struct.pack('<f',value))[0]


def nearby(value):
    raw=bits(value)
    return [struct.unpack('<f',struct.pack('<I',raw+d))[0] for d in [-1,0,1]]


def cases():
    boundary=nearby(.001);values=[-v for v in boundary]+[-0.,0.,*boundary,1.]
    yield from itertools.product(values,repeat=3)
    coefficient=struct.unpack('<f',struct.pack('<I',0x3e9c0ebf))[0]
    feet=[0.,*nearby(.001/coefficient),.01,.1,10.]
    metres=[f32(f32(x)*coefficient) for x in feet]
    yield from itertools.product(metres,repeat=3)
    rng=random.Random(20261024)
    for _ in range(1000):
        yield (rng.uniform(-.003,.003),rng.uniform(-.003,.003),rng.uniform(-.003,.003))


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu=Emulator(path);aircraft=emu.alloc(0x6030);wing=emu.alloc(0x680);emu.write_u64(aircraft+0x6028,wing)
    print('# reference_sha256',SHA256)
    print('# original_range141ab17d0..141ab1816 exclusive;no stubs;R13=Aircraft,RAX=0,RSI=1,XMM9=double.001')
    print('# enabled=f64(span)>.001 && (f64(root)>.001 || f64(tip)>.001);includes adjacent±1mm,signedzero,ACF feet roundedwithf32coefficient0x3e9c0ebf')
    print('# E: span root tip -> enabled (f32bits,flagdecimal)')
    count=0
    for span,root,tip in cases():
        values=[f32(x) for x in [span,root,tip]]
        for off,value in zip([0x10,8,0xc],values):emu.write_f32(wing+off,value)
        for register,value in [(UC_X86_REG_R13,aircraft),(UC_X86_REG_RAX,0),(UC_X86_REG_RSI,1),(UC_X86_REG_XMM9,struct.unpack('<Q',struct.pack('<d',.001))[0])]:emu.uc.reg_write(register,value)
        emu.run_range(0x141ab17d0,0x141ab1816)
        output=emu.read(wing+0x678,1)[0]
        assert output==int(values[0]>.001 and (values[1]>.001 or values[2]>.001))
        assert not emu.stubs
        print('E',*[f'{bits(x):08x}' for x in values],output)
        count+=1
    print('# cases',count)


if __name__=='__main__':
    main(sys.argv[1])
