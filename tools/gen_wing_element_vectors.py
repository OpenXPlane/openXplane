#!/usr/bin/env python3
"""Generates test vectors for src/wing_element.rs::evaluate by running the ORIGINAL 0x1411b6630.

The function is executed in the emulator on synthetic wing, flow and aircraft objects with the profile
function 0x141a44350 replaced by a stub that records its arguments and returns recorded random results.
The test replays those results and requires the port to pass the same arguments to every call and to
produce the same accumulators and return value, bit for bit. Cases stay on the straight-wing path (sweep
below 38 degrees, delta-wing weight zero).

Line layout (float32 hex, integers decimal):
  e els retain arg6 ice alpha_in ex_cl ex_cd ex_cm | is_right f14 f18 f1c r5c r60 r64 r68
  c0 c1 x0 x1 y0 y1 z0 z1 flap slat n1 n2 n3 | f5c f6c f1a0 f1a4 f408 dac bcc8 bcd0
  b1f00 b1f3c b64f4 b64f8 b64fc g10 | v r11 r21 r163 stall_in |
  ncalls { slot x y z retain diag re arg6 alpha mult div dac stalled | ret cl cd cm ratio stall_out } |
  ret cl cd cm ratio cdi stall_final

    python3 tools/gen_wing_element_vectors.py Xplane12/X-Plane.exe > tests/data/wing_element.txt
Needs: pip install unicorn numpy
"""
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RCX, UC_X86_REG_RSP, UC_X86_REG_XMM2, UC_X86_REG_XMM3  # noqa: E402

FUNC = 0x1411b6630
EVAL = 0x141a44350


def hx(x):
    return f'{struct.unpack("<I", struct.pack("<f", x))[0]:08x}'


def low_f32(value):
    return struct.unpack('<f', (value & 0xffffffff).to_bytes(4, 'little'))[0]


def main():
    emu = Emulator(sys.argv[1])
    A, W, B, R, G, OUT = (emu.alloc(0x10000), emu.alloc(0x4000), emu.alloc(0x8000), emu.alloc(0x1000),
                          emu.alloc(0x100), emu.alloc(0x100))
    foils = [emu.alloc(0x1000) for _ in range(3)]
    emu.write_u64(A + 0x20, B)
    for k, off in enumerate((0x3678, 0x3680, 0x3688)):
        emu.write_u64(W + off, foils[k])
        emu.write_u64(foils[k] + 0xd0, G)
    rng = random.Random(20261006)
    recorded = []

    def stub(e):
        rsp = e.reg(UC_X86_REG_RSP)
        slot = foils.index(e.reg(UC_X86_REG_RCX))
        ptrs = [e.read_u64(rsp + 0x70 + 8 * i) for i in range(5)]
        call = dict(
            slot=slot, x=low_f32(e.reg(UC_X86_REG_XMM2)), y=low_f32(e.reg(UC_X86_REG_XMM3)),
            z=e.read_f32(rsp + 0x28), retain=e.read_u32(rsp + 0x30), diag=e.read_u32(rsp + 0x38),
            re=e.read_f32(rsp + 0x40), arg6=e.read_f32(rsp + 0x48), alpha=e.read_f32(rsp + 0x50),
            mult=e.read_f32(rsp + 0x58), div=e.read_f32(rsp + 0x60), dac=e.read(rsp + 0x68, 1)[0],
            stalled=e.read_u32(ptrs[4]))
        out = dict(ret=rng.uniform(-2, 2), cl=rng.uniform(-2, 2), cd=rng.uniform(0, 1), cm=rng.uniform(-.5, .5),
                   ratio=rng.uniform(-1.5, 1.5), stall=rng.randrange(2))
        for p, k in zip(ptrs[:4], ('cl', 'cd', 'cm', 'ratio')):
            e.write_f32(p, out[k])
        e.write_u32(ptrs[4], out['stall'])
        e.set_xmm_f32(0, out['ret'])
        recorded.append((call, out))

    emu.stubs[EVAL] = stub
    print('# wing element vectors from the original 0x1411b6630 (see tools/gen_wing_element_vectors.py)')
    produced = 0
    while produced < 400:
        els = rng.randrange(3, 11)
        e = rng.randrange(0, els)
        ratios_choices = [(0.1, 0.2, 0.8, 0.9), (0.0, 0.3, 0.7, 1.0), (0.2, 0.2, 0.8, 0.8), (0.1, 0.5, 0.5, 0.9)]
        ratios = rng.choice(ratios_choices)
        names = [rng.randrange(3) for _ in range(3)]
        dist_xy = rng.uniform(0.1, 3.0)
        x0, y0 = rng.uniform(-4, 4), rng.uniform(-1, 1)
        x1, y1 = x0 + dist_xy * rng.uniform(0.5, 1), y0 + dist_xy * rng.uniform(-0.3, 0.3)
        c0, c1 = rng.uniform(0.5, 4), rng.uniform(0.5, 4)
        dist = math.hypot(x1 - x0, y1 - y0)
        z0 = rng.uniform(0, 8)
        z1 = z0 - 0.25 * c0 + 0.25 * c1 + dist * math.tan(math.radians(rng.uniform(-30, 30)))
        vals = dict(
            f14=rng.uniform(1.5, 12), f18=rng.uniform(-1, 6), f1c=rng.uniform(0, 1),
            f5c=rng.choice([rng.uniform(-60, 60), 0.0, rng.uniform(-10, 10)]), f6c=rng.uniform(0.3, 1.3),
            f1a0=rng.uniform(-1, 1), f1a4=rng.uniform(-1, 1), f408=rng.uniform(-3, 3),
            b1f00=rng.uniform(0.1, 2), b1f3c=rng.uniform(0.1, 2), b64f4=rng.uniform(1, 12),
            b64f8=rng.uniform(1, 12), b64fc=rng.uniform(1, 12), g10=rng.uniform(0.3, 2),
            v=rng.uniform(2, 120), r11=rng.uniform(-5, 5), r21=rng.uniform(2, 100), r163=rng.uniform(0.5, 1.5),
            arg6=rng.uniform(-1, 1), ice=rng.choice([0.0, 0.0, rng.uniform(0.01, 0.6)]),
            alpha_in=rng.choice([rng.uniform(-30, 30), rng.uniform(-400, 700), 18.0, 180.0, -180.0]),
            ex=[rng.choice([0.0, rng.uniform(-0.3, 0.3)]) for _ in range(3)])
        flap, slat = rng.randrange(2), rng.randrange(2)
        retain = rng.randrange(2)
        # the diagnostic flags stay off: the logging path needs the engine's output stream
        dac, bcc8, bcd0 = rng.randrange(2), 0, 0
        stall_in = rng.randrange(2)
        for off, val in ((0, rng.choice([-1.0, 1.0])), (0x14, vals['f14']), (0x18, vals['f18']), (0x1c, vals['f1c']),
                         (0x5c, ratios[0]), (0x60, ratios[1]), (0x64, ratios[2]), (0x68, ratios[3])):
            emu.write_f32(W + off, val)
        is_right = emu.read_f32(W)
        emu.write_u32(W + 4, els)
        for i in range(11):
            emu.write_f32(W + 0x70 + 4 * i, rng.uniform(0.5, 4))
            for base in (0x5bc, 0x5e8, 0x614):
                emu.write_f32(W + base + 4 * i, rng.uniform(-3, 8))
            emu.write_u32(W + 0x56c + 4 * i, 0)
            emu.write_u32(W + 0x594 + 4 * i, 0)
        for off, a, b in ((0x70, c0, c1), (0x5bc, x0, x1), (0x5e8, y0, y1), (0x614, z0, z1)):
            emu.write_f32(W + off + 4 * e, a)
            emu.write_f32(W + off + 4 * (e + 1), b)
        emu.write_u32(W + 0x56c + 4 * e, flap)
        emu.write_u32(W + 0x594 + 4 * e, slat)
        for off, n in zip((0x3618, 0x3638, 0x3658), names):
            txt = f'foil{n}'.encode()
            emu.write(W + off, txt + b'\0' * (16 - len(txt)))
            emu.write_u64(W + off + 0x10, len(txt))
            emu.write_u64(W + off + 0x18, 15)
        for off, key in ((0x5c, 'f5c'), (0x6c, 'f6c'), (0x1a0, 'f1a0'), (0x1a4, 'f1a4'), (0x408, 'f408')):
            emu.write_f32(A + off, vals[key])
        emu.write_u32(A + 0xdac, dac)
        emu.write_u32(A + 0xbcc8, bcc8)
        emu.write_u32(A + 0xbcd0, bcd0)
        for off, key in ((0x1f00, 'b1f00'), (0x1f3c, 'b1f3c'), (0x64f4, 'b64f4'), (0x64f8, 'b64f8'), (0x64fc, 'b64fc')):
            emu.write_f32(B + off, vals[key])
        emu.write_f32(G + 0x10, vals['g10'])
        for off, key in ((4 * (1 + e), 'v'), (4 * (11 + e), 'r11'), (4 * (21 + e), 'r21'), (4 * 163, 'r163')):
            emu.write_f32(R + off, vals[key])
        emu.write_u32(R + 4 * (121 + e), stall_in)
        emu.write_f32(R + 4 * (111 + e), 9.0)
        for i in range(4):
            emu.write_f32(OUT + 4 * i, 7.0)
        recorded.clear()
        emu.call(FUNC, ints=[A, W, R, retain],
                 stack=[e, vals['arg6'], vals['ice'], vals['alpha_in'], *vals['ex'], OUT, OUT + 4, OUT + 8, OUT + 12])
        ret = emu.xmm0_f32()
        final = [ret, emu.read_f32(OUT), emu.read_f32(OUT + 4), emu.read_f32(OUT + 8), emu.read_f32(R + 4 * (111 + e)),
                 emu.read_f32(OUT + 12), emu.read_u32(R + 4 * (121 + e))]
        if not all(math.isfinite(v) for v in final[:6]):
            continue
        tokens = [str(e), str(els), str(retain), hx(vals['arg6']), hx(vals['ice']), hx(vals['alpha_in']),
                  *map(hx, vals['ex']), '|', hx(is_right), *map(hx, (vals['f14'], vals['f18'], vals['f1c'], *ratios)),
                  hx(c0), hx(c1), hx(x0), hx(x1), hx(y0), hx(y1), hx(z0), hx(z1), str(flap), str(slat), *map(str, names), '|',
                  *map(hx, (vals['f5c'], vals['f6c'], vals['f1a0'], vals['f1a4'], vals['f408'])), str(dac), str(bcc8), str(bcd0),
                  *map(hx, (vals['b1f00'], vals['b1f3c'], vals['b64f4'], vals['b64f8'], vals['b64fc'], vals['g10'])),
                  *map(hx, (vals['v'], vals['r11'], vals['r21'], vals['r163'])), str(stall_in), '|', str(len(recorded))]
        for c, o in recorded:
            tokens += [str(c['slot']), hx(c['x']), hx(c['y']), hx(c['z']), str(c['retain']), str(c['diag']), hx(c['re']),
                       hx(c['arg6']), hx(c['alpha']), hx(c['mult']), hx(c['div']), str(c['dac']), str(c['stalled']), '|',
                       hx(o['ret']), hx(o['cl']), hx(o['cd']), hx(o['cm']), hx(o['ratio']), str(o['stall'])]
        tokens += ['|', *map(hx, final[:6]), str(final[6])]
        print(*tokens)
        produced += 1
    print(f'# {produced} cases', file=sys.stderr)


if __name__ == '__main__':
    main()
