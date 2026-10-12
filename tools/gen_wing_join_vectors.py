#!/usr/bin/env python3
"""Replay both original wing-group endpoint predicates, without stubs.

141a6beb0 tests candidate ROOT;141a6bbf0 tests candidate TIP. Candidate one-element
geometry has explicit root/tip arrays and dihedral. Upstream grouping is not
executed here. Usage: python tools/gen_wing_join_vectors.py Xplane12/X-Plane.exe
"""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator

SHA256='2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'


def f32(value):
    return struct.unpack('<f',struct.pack('<f',value))[0]


def bits(value):
    return struct.unpack('<I',struct.pack('<f',value))[0]


def near(value):
    raw=bits(value)
    return [struct.unpack('<f',struct.pack('<I',raw+d))[0] for d in [-1,0,1]]


def cases():
    # Strict chord-relative distance comparisons and wrapped angle boundaries.
    for chord in [.125,1.,4.]:
        for axis,scale in [(0,.01),(1,.01),(2,.2)]:
            for delta in near(chord*scale):
                for sign in [-1.,1.]:
                    for angle in [-405.,-360.,-180.,-90.,-45.,-0.,0.,45.,90.,180.,360.,405.]:
                        for late in [0,1]:
                            root=[1.,0.,0.];tip=[2.,0.,0.];point=root.copy();point[axis]=f32(point[axis]+sign*delta)
                            yield root,tip,0.,point,chord,angle,late
    band=[-x for x in near(.01)]+[-0.,0.]+near(.01)
    for x0 in band:
        for x1 in band:
            for late in [0,1]:
                yield [x0,0.,0.],[x1,0.,0.],0.,[x0,0.,0.],1.,90.,late
    for value in near(45.):
        for sign in [-1.,1.]:
            for late in [0,1]:
                yield [1.,0.,0.],[2.,0.,0.],f32(value*sign),[1.,0.,0.],1.,0.,late
    rng=random.Random(20261024)
    for i in range(1500):
        # Dyadic endpoint coordinates remove incidental endpoint interpolation
        # roundoff; native helper still executes its original interpolators.
        root=[rng.randrange(-64,65)/32. for _ in range(3)]
        tip=[rng.randrange(-64,65)/32. for _ in range(3)]
        dihedral=f32(rng.uniform(-180.,180.));chord=f32(rng.uniform(.1,4.));base=root if i%2 else tip
        point=[f32(a+rng.uniform(-.015,.015)*chord) for a in base]
        point[2]=f32(base[2]+rng.uniform(-.25,.25)*chord)
        angle=f32(dihedral+rng.uniform(-400.,400.))
        yield root,tip,dihedral,point,chord,angle,rng.randrange(2)


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest()!=SHA256:
        raise ValueError('Reference executable SHA256 mismatch')
    emu=Emulator(path);aircraft=emu.alloc(0x6030);wing=emu.alloc(0x36c8)
    emu.write_u64(aircraft+0x6028,wing);emu.write_u32(wing+4,1)
    print('# reference_sha256',SHA256)
    print('# original_functions root141a6beb0 tip141a6bbf0;no stubs;candidate index0/one-element array;finite bounded angles')
    print('# strictXY abs(f32(delta))<f64(refchord)*.01;strictZ<*.2;sidecross double±.01;wrappedabsangle<=45 otherwiseabs(pointx)>=.01 andlate')
    print('# J: candidate_root3 candidate_tip3 candidate_dihedral reference_point3 reference_chord reference_dihedral late -> accepts_root accepts_tip (f32bits,flagsdecimal)')
    count=0
    for root,tip,dihedral,point,chord,angle,late in cases():
        root=[f32(x) for x in root];tip=[f32(x) for x in tip];point=[f32(x) for x in point];dihedral=f32(dihedral);chord=f32(chord);angle=f32(angle)
        for off,a,b in zip([0x5bc,0x5e8,0x614],root,tip):
            emu.write_f32(wing+off,a);emu.write_f32(wing+off+4,b)
        emu.write_f32(wing+0x20,dihedral)
        output=[emu.call(address,ints=[aircraft],floats=[0.,*point],stack=[float(chord),float(angle),0,0,late]) for address in [0x141a6beb0,0x141a6bbf0]]
        assert not emu.stubs
        assert all(x in [0,1] for x in output)
        print('J',*[f'{bits(x):08x}' for x in [*root,*tip,dihedral,*point,chord,angle]],late,*output)
        count+=1
    print('# cases',count)


if __name__=='__main__':
    main(sys.argv[1])
