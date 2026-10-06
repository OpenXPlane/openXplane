#!/usr/bin/env python3
"""Generates test vectors for element_force by running the ORIGINAL get_el_force (0x1411b9840).

The flight object F, the aircraft object B (= [F+0x20]), the wing W and the element arrays X are filled
with random values at the offsets the port reads (everything else stays zero). The profile function
0x141a44350 is replaced by a recording stub (as in gen_wing_element_vectors.py), the input-binding query
0x1407ace10 by a mask stub (as in gen_control_surface_vectors.py), and the alternative regime function
0x1411b8e00 runs for real (the supersonic regime); the foil thickness values at +0x58 of the three airfoil
objects are random. F+0x28 is set so the original skips its
structural-load section.

Line: e retain ice g10 name0 name1 name2 mask thick0 thick1 thick2 | F off=hex ... | B ... | W ... | X ... |
      ncalls { slot x y z retain diag re arg6 alpha mult div dac stalled | ret cl cd cm ratio stall } |
      out1 out2 out3 x_f4 x_11c x_144 x_16c x_1bc_after stall_after

    python3 tools/gen_element_force_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/element_force.txt
"""
import math
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from gen_control_deflection_vectors import TABLE  # noqa: E402
from gen_control_surface_vectors import MASK_AT, QUERY, install_stub  # noqa: E402
from unicorn.x86_const import UC_X86_REG_RCX, UC_X86_REG_RSP, UC_X86_REG_XMM2, UC_X86_REG_XMM3  # noqa: E402

FUNC = 0x1411b9840
EVAL = 0x141a44350
ALT = 0x1411b8e00
GATES = [(0xb, 0x2fc, 0x29c), (0xc, 0x32c, 0x2a0), (0xd, 0x41c, 0x2a4), (0xe, 0x44c, 0x2a8), (0xf, 0x47c, 0x2b4),
         (0x10, 0x35c, 0x2b8), (0x11, 0x38c, 0x2bc), (0x12, 0x3bc, 0x2c0), (0x13, 0x3ec, 0x2c4), (0x14, 0x50c, 0x2c8),
         (0x15, 0x53c, 0x2cc), (0x16, 0x4ac, 0x2ac), (0x17, 0x4dc, 0x2b0)]


def f32bits(x):
    return struct.unpack('<I', struct.pack('<f', float(x)))[0]


def hx(x):
    return f'{f32bits(x):08x}'


def low_f32(value):
    return struct.unpack('<f', (value & 0xffffffff).to_bytes(4, 'little'))[0]


def main():
    emu = Emulator(sys.argv[1])
    install_stub(emu)
    F, W, B, X, G, DBG = (emu.alloc(0x10000), emu.alloc(0x4000), emu.alloc(0x8000), emu.alloc(0x1000),
                          emu.alloc(0x100), emu.alloc(0xbd00))
    OUT = emu.alloc(0x100)
    foils = [emu.alloc(0x1000) for _ in range(3)]
    rng = random.Random(20261012)
    recorded = []
    state = {'alt': False}

    def eval_stub(e):
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

    def alt_stub(e):
        state['alt'] = True

    emu.stubs[EVAL] = eval_stub
    print('# element force vectors from the original 0x1411b9840 (see tools/gen_element_force_vectors.py)')
    produced = 0
    attempts = 0
    while produced < 250 and attempts < 4000:
        attempts += 1
        for obj, size in ((F, 0x10000), (W, 0x4000), (B, 0x8000), (X, 0x1000)):
            emu.write(obj, bytes(size))
        emu.write_u64(F + 0x20, B)
        emu.write_u64(B + 0x61f8, DBG)
        for k, off in enumerate((0x3678, 0x3680, 0x3688)):
            emu.write_u64(W + off, foils[k])
            emu.write_u64(foils[k] + 0xd0, G)
        els = rng.randrange(3, 11)
        e = rng.randrange(0, els)
        regs = {'F': {}, 'B': {}, 'W': {}, 'X': {}}

        def put(obj, name, off, value, integer=False):
            base = {'F': F, 'B': B, 'W': W, 'X': X}[name]
            if integer:
                emu.write_u32(base + off, value)
                regs[name][off] = value & 0xffffffff
            else:
                emu.write_f32(base + off, value)
                regs[name][off] = f32bits(value)

        # --- geometry as in the wing element generator (straight path)
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
        put('', 'W', 0, rng.choice([-1.0, 1.0]))
        put('', 'W', 4, els, True)
        for off in (0x14, 0x18, 0x1c):
            pass
        put('', 'W', 0x14, rng.uniform(1.5, 12))
        put('', 'W', 0x18, rng.uniform(-1, 6))
        put('', 'W', 0x1c, rng.uniform(0, 1))
        put('', 'W', 0x10, rng.uniform(1, 20))
        put('', 'W', 0x20, rng.choice([rng.uniform(0, 90), rng.uniform(0, 45), rng.uniform(-90, 0)]))
        put('', 'W', 0x30, rng.uniform(-20, 20))
        put('', 'W', 0x38, rng.uniform(-30, 30))
        for off, r in zip((0x5c, 0x60, 0x64, 0x68), ratios):
            put('', 'W', off, r)
        for i in (e, e + 1):
            pass
        for off, a, b in ((0x70, c0, c1), (0x5bc, x0, x1), (0x5e8, y0, y1), (0x614, z0, z1)):
            put('', 'W', off + 4 * e, a)
            put('', 'W', off + 4 * (e + 1), b)
        flap, slat = rng.randrange(2), rng.randrange(2)
        put('', 'W', 0x56c + 4 * e, flap, True)
        put('', 'W', 0x594 + 4 * e, slat, True)
        for code, gate, angle_at in GATES:
            put('', 'W', gate + 4 * e, rng.choice([0, 1, 1, 1]), True)
            put('', 'X', angle_at, rng.choice([rng.uniform(-30, 30), rng.uniform(-60, 60), 0.0, rng.uniform(-10, 10)]))
            fa, la, aa, ba = TABLE[code]
            first = rng.randrange(0, els)
            last = first if rng.random() < 0.2 else rng.randrange(first, els)
            put('', 'W', fa, first)
            put('', 'W', la, last)
            put('', 'B', aa, rng.uniform(-1, 1))
            put('', 'B', ba, rng.uniform(-1, 1))
        thick = []
        for k, off in enumerate((0x3678, 0x3680, 0x3688)):
            if False:  # a null airfoil pointer would also reach the profile function stub
                emu.write_u64(W + off, 0)
                thick.append(None)
            else:
                emu.write_u64(W + off, foils[k])
                value = rng.uniform(0.02, 0.25)
                emu.write_f32(foils[k] + 0x58, value)
                thick.append(value)
        for off, n in zip((0x3618, 0x3638, 0x3658), names):
            txt = f'foil{n}'.encode()
            emu.write(W + off, txt + b'\0' * (16 - len(txt)))
            emu.write_u64(W + off + 0x10, len(txt))
            emu.write_u64(W + off + 0x18, 15)
        # --- F
        put('', 'F', 0x5c, rng.choice([rng.uniform(-60, 60), 0.0, rng.uniform(-10, 10)]))
        put('', 'F', 0x6c, rng.uniform(0.3, 1.3))
        put('', 'F', 0x74, rng.uniform(0.5, 3))
        put('', 'F', 0x1a0, rng.uniform(-1, 1))
        put('', 'F', 0x1a4, rng.uniform(-1, 1))
        put('', 'F', 0x408, rng.uniform(-3, 3))
        # the flow-separation weight is zero unless F+0x420 exceeds the limited ratio
        put('', 'F', 0x420, rng.choice([-5.0, -1.0, 0.005, 0.5, 0.9, 1.0, 1.15, 1.5, 2.0, 3.5]))
        put('', 'F', 0x64c0, rng.uniform(-0.5, 0.5))
        put('', 'F', 0xdac, rng.randrange(2), True)
        emu.write_u32(F + 0x28, 1)
        # --- B
        for off, key in ((0x1f00, (0.1, 2)), (0x1f3c, (0.1, 2)), (0x64f4, (1, 12)), (0x64f8, (1, 12)), (0x64fc, (1, 12)),
                         (0x1f04, (-1, 1)), (0x1f40, (-1, 1)), (0x1f78, (-2, 2)), (0x1f7c, (-2, 2)), (0x1f80, (-2, 2)),
                         (0x1fb8, (-2, 2)), (0x1fbc, (-2, 2)), (0x1fc0, (-2, 2))):
            put('', 'B', off, rng.uniform(*key))
        for off in (0x1d20, 0x1d24, 0x1d28):
            put('', 'B', off, rng.randrange(0, 5), True)
        put('', 'B', 0x1f74, rng.randrange(0, 7), True)
        put('', 'B', 0x1eb0, rng.randrange(0, 5), True)
        for k in range(5):
            put('', 'B', 0x1f84 + 4 * k, rng.choice([rng.uniform(-60, 60), rng.uniform(-0.5, 0.5), 0.0]))
            put('', 'B', 0x1fc4 + 4 * k, rng.choice([rng.uniform(-60, 60), rng.uniform(-0.5, 0.5), 0.0]))
        emu.write_f32(G + 0x10, 0.0)
        g10 = rng.uniform(0.3, 2)
        emu.write_f32(G + 0x10, g10)
        # --- X
        put('', 'X', 4 + 4 * e, rng.uniform(2, 120))
        put('', 'X', 0x2c + 4 * e, rng.uniform(-5, 5))
        put('', 'X', 0x54 + 4 * e, rng.uniform(2, 100))
        put('', 'X', 0x7c + 4 * e, rng.uniform(-0.1, 0.1))
        put('', 'X', 0xa4 + 4 * e, rng.uniform(-0.1, 0.1))
        put('', 'X', 0xcc + 4 * e, rng.uniform(-0.1, 0.1))
        put('', 'X', 0x1bc + 4 * e, rng.choice([rng.uniform(-1.5, 1.5), rng.uniform(-0.3, 0.3), 0.0]))
        put('', 'X', 0x1e4 + 4 * e, rng.randrange(2), True)
        put('', 'X', 0x288, rng.uniform(0.5, 2))
        put('', 'X', 0x28c, rng.uniform(0.5, 1.5))
        mask = rng.choice([0, rng.getrandbits(24), rng.getrandbits(24) & rng.getrandbits(24), 0xffffff])
        emu.write_u32(MASK_AT, mask)
        retain = rng.randrange(2)
        ice = rng.choice([0.0, 0.0, rng.uniform(0.01, 0.6)])
        for i in range(3):
            emu.write_f32(OUT + 4 * i, 7.0)
        recorded.clear()
        state['alt'] = False
        emu.call(FUNC, ints=[F, W, X, retain], stack=[e, ice, OUT, OUT + 4, OUT + 8])
        if state['alt']:
            continue
        outs = [emu.read_f32(OUT + 4 * i) for i in range(3)]
        arrays = [emu.read_f32(X + off + 4 * e) for off in (0xf4, 0x11c, 0x144, 0x16c, 0x1bc)]
        stall_after = emu.read_u32(X + 0x1e4 + 4 * e)
        if not all(math.isfinite(v) for v in outs + arrays):
            continue
        tokens = [str(e), str(retain), hx(ice), hx(g10), *map(str, names), f'{mask:06x}',
                  *['-' if v is None else hx(v) for v in thick]]
        for name in ('F', 'B', 'W', 'X'):
            tokens += ['|', *[f'{o:x}={v:08x}' for o, v in sorted(regs[name].items())]]
        tokens += ['|', str(len(recorded))]
        for c, o in recorded:
            tokens += [str(c['slot']), hx(c['x']), hx(c['y']), hx(c['z']), str(c['retain']), str(c['diag']), hx(c['re']),
                       hx(c['arg6']), hx(c['alpha']), hx(c['mult']), hx(c['div']), str(c['dac']), str(c['stalled']), '|',
                       hx(o['ret']), hx(o['cl']), hx(o['cd']), hx(o['cm']), hx(o['ratio']), str(o['stall'])]
        tokens += ['|', *map(hx, outs), *map(hx, arrays), str(stall_after)]
        print(*tokens)
        produced += 1
    print(f'# {produced} cases from {attempts} attempts', file=sys.stderr)


if __name__ == '__main__':
    main()
