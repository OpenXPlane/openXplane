#!/usr/bin/env python3
"""Runs one original function on lazily random-filled objects at absolute emulator addresses and prints a case:

  H <kind> <entry> <args...>        header (free text for the Rust side)
  W addr=word ...                   initial words (absolute hex addresses) the function read or the case preset
  C addr rcx rdx r8 r9 x0 x1 x2 x3 s0 s1 s2 s3 | rax xmm0 | e addr=word ...     one line per stubbed call, in order
  O addr=word ...                   final words of everything the function (or a stub) wrote

Stubs are registered with `case.stub(address, handler)`; a handler receives a `Call` (arguments, `put` for memory
effects, `ret_int`/`ret_f32`/`ret_f64`) and the random generator. Words written by a stub through `put` are logged
as effects and replayed by the Rust side.
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator, STACK_TOP  # noqa: E402
from xp_fuzz import Fuzz  # noqa: E402
from unicorn.x86_const import (  # noqa: E402
    UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_RSP, UC_X86_REG_XMM0,
    UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3)

XMM = [UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3]


def f32bits(v):
    return struct.unpack('<I', struct.pack('<f', v))[0]


class Call:
    def __init__(self, case, address):
        e = case.emu
        self.case = case
        self.address = address
        self.ints = [e.reg(r) for r in (UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9)]
        self.xmm = [e.uc.reg_read(r) & 0xffffffff for r in XMM]
        rsp = e.reg(UC_X86_REG_RSP)
        self.stack = [e.read_u64(rsp + 0x28 + 8 * i) for i in range(12)]
        self.effects = []
        self.rax = 0
        self.xmm0 = 0

    def put(self, address, word):
        self.case.emu.write_u32(address, word)
        self.case.fz.note_write(address)
        self.effects.append((address, word & 0xffffffff))

    def put_f32(self, address, value):
        self.put(address, f32bits(value))

    def ret_int(self, v):
        self.rax = v & 0xffffffffffffffff

    def ret_f32(self, v):
        self.xmm0 = f32bits(v)

    def ret_f64(self, v):
        self.xmm0 = struct.unpack('<Q', struct.pack('<d', v))[0]


class VmCase:
    def __init__(self, exe, seed, ranges):
        self.emu = Emulator(exe)
        self.fz = Fuzz(self.emu, exe, ranges, seed)
        self.rng = self.fz.rng
        self.mark = self.emu.heap_top
        self.calls = []
        self.regions = {}

    def reset(self):
        self.emu.heap_top = self.mark
        self.fz.policy = {}
        self.fz.regions.clear()
        self.fz._known.clear()
        self.fz._init.clear()
        self.fz._written.clear()
        self.calls = []
        self.regions = {}
        # the static input tables of the binding query: random-filled on first read like any other input
        self.fz.region('IT2', 0x142515560, 4 * 0x302)
        self.fz.region('IT3', 0x142515364, 40 * 13 + 4)

    def region(self, name, size):
        addr = self.emu.alloc(size)
        self.fz.region(name, addr, size)
        self.emu.write(addr, bytes(size))
        self.regions[name] = addr
        return addr

    def stub(self, address, handler, record=True, caller=None):
        """Replaces a function; with `caller=(lo, hi)` only calls whose return address lies in the range."""
        def run(e):
            if caller is not None:
                rsp = e.uc.reg_read(UC_X86_REG_RSP)
                ret = struct.unpack('<Q', e.uc.mem_read(rsp, 8))[0]
                if not caller[0] <= ret < caller[1]:
                    return True
            call = Call(self, address)
            handler(call, self.rng)
            e.uc.reg_write(UC_X86_REG_RAX, call.rax)
            e.uc.reg_write(UC_X86_REG_XMM0, call.xmm0)
            if record:
                self.calls.append(call)
        self.emu.stubs[address] = run

    def run(self, entry, **kwargs):
        return self.emu.call(entry, **kwargs)

    def dump(self, header, extra_words=None):
        out = [f'H {header}']
        words = {}
        for name, init in self.fz.initial().items():
            base = self.fz.regions[name][0]
            for off, w in init.items():
                words[base + off] = w
        words.update(extra_words or {})
        out.append('W ' + ' '.join(f'{a:x}={w:08x}' for a, w in sorted(words.items())))
        for c in self.calls:
            line = ['C', f'{c.address:x}'] + [f'{v:x}' for v in c.ints] + [f'{v:08x}' for v in c.xmm] \
                + [f'{v:x}' for v in c.stack[:4]] + ['|', f'{c.rax:x}', f'{c.xmm0:x}', '|']
            line += [f'{a:x}={w:08x}' for a, w in c.effects]
            out.append(' '.join(line))
        written = self.fz.written()
        out.append('O ' + ' '.join(f'{self.fz.regions[n][0] + o:x}={w:08x}' for n in written for o, w in written[n].items()))
        return '\n'.join(out)


def entry_rsp(stack_args=0):
    """The stack pointer at the entry of a function called through Emulator.call with `stack_args` stack arguments."""
    return ((STACK_TOP - 0x1000 - 0x20 - 8 * stack_args) & ~0xf) - 8


GRID_GLOBAL = 0x1461179f8


def setup_terrain(case, rng, mesh_name='F', mesh_off=0x42e40, triangles=6):
    """Presets a terrain mesh object inside region `mesh_name` (triangles, hole range, top height) and, at random, the
    global grid object that 0x141945c90 returns. The probe 0x141960dc0 and its helpers then run on real memory."""
    fz = case.fz
    count = rng.randrange(0, triangles + 1) if rng.random() < 0.15 else rng.randrange(3, triangles + 3)
    tri = case.region('M', 36 * max(count, 1) + 64)
    for k in range(count):
        for i in range(9):
            lo, hi = (-8.0, 25.0) if i % 3 == 1 else (-12.0, 12.0)
            fz.preset_f32('M', 36 * k + 4 * i, rng.uniform(lo, hi))
    fz.preset(mesh_name, mesh_off + 0x28, tri & 0xffffffff)
    fz.preset(mesh_name, mesh_off + 0x2c, tri >> 32)
    g0 = 0 if rng.random() < 0.7 else rng.randrange(0, count + 1)
    g1 = count if rng.random() < 0.7 else rng.randrange(g0, count + 1)
    h0 = rng.randrange(0, count + 1)
    h1 = h0 if rng.random() < 0.6 else rng.randrange(h0, count + 1)
    for off, v in ((0x74, h0), (0x78, h1), (0x7c, g0), (0x80, g1)):
        fz.preset(mesh_name, mesh_off + off, v)
    fz.preset_f32(mesh_name, mesh_off + 0x84, rng.choice([100.0, 100.0, rng.uniform(-5, 30)]))
    grid = 0
    if rng.random() < 0.6:
        grid = case.region('GR', 0x80)
        cell = case.region('GC', 0x20)
        holder = case.region('GH', 0x90)
        n = rng.choice([2, 4, 8])
        data = case.region('GD', 2 * n * n + 4)
        fz.preset_f32('GC', 0, rng.uniform(-3, 3))
        fz.preset_f32('GC', 4, rng.uniform(-3, 3))
        fz.preset_f32('GR', 8, rng.choice([1.0, 4.0, 20.0]))
        fz.preset('GR', 0, n)
        fz.preset_f32('GR', 0x30, rng.uniform(-8, 8))
        fz.preset_f32('GR', 0x38, rng.uniform(-8, 8))
        fz.preset('GR', 0x48, cell & 0xffffffff)
        fz.preset('GR', 0x4c, cell >> 32)
        fz.preset('GC', 0x18, holder & 0xffffffff)
        fz.preset('GC', 0x1c, holder >> 32)
        fz.preset('GH', 0x80, data & 0xffffffff)
        fz.preset('GH', 0x84, data >> 32)
        for k in range(0, 2 * n * n, 4):
            lo = rng.choice([rng.randrange(-32768, 32768), -32768]) & 0xffff
            hi = rng.choice([rng.randrange(-32768, 32768), -32768]) & 0xffff
            fz.preset('GD', k, lo | hi << 16)
    fz.region('GL', GRID_GLOBAL, 8)
    fz.preset('GL', 0, grid & 0xffffffff)
    fz.preset('GL', 4, grid >> 32)


def stub_terrain_helpers(case, caller=(0x141960dc0, 0x141961200)):
    """Replaces the helpers of the probe that are not ported: the grid height and the triangle normal."""
    def normal(call, rng):
        for k in range(3):
            call.put_f32(call.ints[3] + 4 * k, rng.uniform(-1, 1))
    case.stub(0x1406ed6a0, normal, caller=caller)


SURFACES = 0x14611ac88


def setup_segment_mesh(case, rng, mesh_name='MS', mesh_off=0, triangles=(10, 26), heights=(-8.0, 25.0), extent=12.0):
    """Presets a mesh object for 0x14195ffc0 in region `mesh_name`: triangles, flag words, the five range bounds,
    moving cells with their spheres, the surface records and the noise table."""
    fz = case.fz
    M = mesh_name
    O = mesh_off
    fz.region('NZ', 0x14578f1f0, 0x100000)
    count = rng.randrange(*triangles)
    tri = case.region('TR', 36 * count)
    for k in range(count):
        for i in range(9):
            lo, hi = heights if i % 3 == 1 else (-extent, extent)
            fz.preset_f32('TR', 36 * k + 4 * i, rng.uniform(lo, hi))
    flags = case.region('FL', 4 * count)
    for k in range(count):
        fz.preset('FL', 4 * k, rng.randrange(0, 4) | rng.choice([0, 0, 0x8000]) | rng.randrange(0, 4) << 16)
    bounds = sorted(rng.randrange(0, count + 1) for _ in range(5))
    if rng.random() < 0.5:
        bounds[0] = 0
    for off, v in zip((0x70, 0x74, 0x78, 0x7c, 0x80), bounds):
        fz.preset(M, O + off, v)
    fz.preset(M, O + 0x28, tri & 0xffffffff)
    fz.preset(M, O + 0x2c, tri >> 32)
    fz.preset(M, O + 0x40, flags & 0xffffffff)
    fz.preset(M, O + 0x44, flags >> 32)
    fz.preset_f32(M, O + 0x88, rng.choice([100.0, 100.0, rng.uniform(-5, 30)]))
    cells = rng.randrange(0, 4)
    spheres = case.region('SPH', 16 * max(cells, 1))
    records = case.region('CEL', 0x50 * max(cells, 1))
    for c in range(cells):
        for i, (lo, hi) in enumerate(((-8, 8), (-8, 8), (-8, 8), (2, 15))):
            fz.preset_f32('SPH', 16 * c + 4 * i, rng.uniform(lo, hi))
        for off in range(0, 0x48, 4):
            fz.preset_f32('CEL', 0x50 * c + off, rng.uniform(-1.5, 1.5))
        a = rng.randrange(0, count)
        fz.preset('CEL', 0x50 * c + 0x48, a)
        fz.preset('CEL', 0x50 * c + 0x4c, rng.randrange(a, count + 1))
    fz.preset(M, O + 0x90, spheres & 0xffffffff)
    fz.preset(M, O + 0x94, spheres >> 32)
    fz.preset(M, O + 0xa8, records & 0xffffffff)
    fz.preset(M, O + 0xac, records >> 32)
    end = records + 0x50 * cells
    fz.preset(M, O + 0xb0, end & 0xffffffff)
    fz.preset(M, O + 0xb4, end >> 32)
    table = case.region('SR', 36 * 4)
    for r in range(4):
        fz.preset_f32('SR', 36 * r, rng.uniform(0.01, 0.3))
        fz.preset_f32('SR', 36 * r + 4, rng.choice([0.0, rng.uniform(0.1, 3)]))
    fz.region('GS', SURFACES, 8)
    fz.preset('GS', 0, table & 0xffffffff)
    fz.preset('GS', 4, table >> 32)



def setup_balance(case, rng, F, B):
    """Presets everything 0x1411ddfd0 reads (the flight object F, the aircraft B and the objects they point to) and
    the replayed callees (registry lookups and the binding query); the reference counting and the label strings
    are unrecorded no-ops."""
    def bits(v):
        return struct.unpack('<I', struct.pack('<f', v))[0]

    def f32(v):
        return struct.unpack('<f', struct.pack('<f', v))[0]

    fz = case.fz
    for address in (0x140d72460, 0x140f59550, 0x1405f3830, 0x1422e7c2c):
        case.stub(address, lambda call, rng: None, record=False)
    FLOW = case.region('FLOW', 0x6600)
    L = case.region('L', 0xbd00)
    W = case.region('W', 0x36c8 * 2)
    ITEMS = case.region('ITM', 0x68 * 4)
    ATT = case.region('ATT', 0x3770 * 4)
    KEYS = case.region('KEYS', 16 * 4)
    CNT = case.region('CNT', 16 * 4)
    RECS = case.region('RECS', 0x90 * 4)
    OBJ = case.region('OBJ', 0x3e40 * 2)
    ENT = case.region('ENT', 0x200)
    SH = case.region('SH', 0x3e40)
    fz.region('KG', 0x142f03778, 0x10)

    def put(name, off, v):
        fz.preset(name, off, v & 0xffffffff)
        fz.preset(name, off + 4, v >> 32)
    for off in (0xbcc8, 0xbcd0):
        fz.preset('F', off, 0)
    put('F', 0x20, B)
    put('F', 0xb588, FLOW)
    for off in (0x64f4, 0x64f8, 0x64fc):
        fz.preset_f32('FLOW', off, rng.uniform(-60, 60))
    for off in (0xb79c, 0xb798, 0xb7a0, 0xb7a4, 0xb838, 0x28c, 0x6534, 0x284, 0x118, 0x11c):
        fz.preset_f32('F', off, rng.choice([0.0, rng.uniform(0, 100)]))
    for k in range(9):
        fz.preset_f32('F', 0x260 + 4 * k, rng.uniform(0, 100))
        fz.preset_f32('F', 0xbbb4 + 4 * k, rng.choice([0.0, rng.uniform(0, 50)]))
    for off in (0x288c, 0x28ac, 0x28b0, 0x28b4, 0x28c8, 0x28cc, 0x28d0, 0x2914, 0x2918, 0x2920, 0x2928, 0x2268, 0x226c):
        fz.preset_f32('B', off, rng.uniform(-3, 100) if off == 0x288c else rng.uniform(-3, 3))
    fz.preset('B', 0x28d4, rng.choice([0, 0, 1]))
    fz.preset('B', 0x28d8, rng.choice([0, 0, 1]))
    for k in range(9):
        fz.preset_f32('B', 0x412c + 4 * k, rng.choice([0.0, rng.uniform(0, 100)]))
        fz.preset('B', 0x4108 + 4 * k, rng.choice([0, 1, 2]))
        for j in range(3):
            fz.preset_f32('B', 0x4150 + 12 * k + 4 * j, rng.uniform(-5, 5))
    # tanks (positions through the part transform)
    scale = rng.uniform(0.5, 3)
    fz.preset_f32('B', 0x2888, scale)
    put('B', 0x61f8, L)
    put('B', 0x6028, W)
    for k in range(9):
        size = rng.choice([0.0, rng.uniform(1, 200)])
        fz.preset_f32('B', 0x3f7c + 4 * k, size)
        capacity = max(f32(size) * f32(scale), 0.01)
        fz.preset_f32('L', 0xbbb4 + 4 * k, rng.choice([0.0, capacity, rng.uniform(0, 1.2) * capacity]))
        fz.preset('B', 0x40e4 + 4 * k, rng.choice([-1, -1, 0, 1]) & 0xffffffff)
        for j in range(3):
            fz.preset_f32('B', 0x400c + 12 * k + 4 * j, rng.uniform(-5, 5))
            fz.preset_f32('B', 0x4078 + 12 * k + 4 * j, rng.uniform(-5, 5))
    for off, lo, hi in ((0x30, -90, 90), (0x20, -90, 90), (0x38, -90, 90), (0x28, -90, 90), (0x40, -3, 3), (0x54, -3, 3), (0x10, -3, 3), (0, -2, 2)):
        for part in range(2):
            fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(lo, hi))
    for part in range(2):
        fz.preset('W', 0x36c8 * part + 0x3c, rng.randrange(-3, 4) & 0xffffffff)
        for off in (0x6f0, 0x6f4, 0x6f8):
            fz.preset_f32('W', 0x36c8 * part + off, rng.uniform(-5, 5))
    for off, lo, hi in ((0x6070, -2, 2), (0x6074, -2, 2), (0x6078, 0.5, 4)):
        fz.preset_f32('B', off, rng.choice([rng.uniform(lo, hi)] + ([0.0] if off == 0x6070 else [])))
    # removed items
    fz.preset('B', 0x91c, rng.randrange(0, 4))
    put('B', 0x5ff8, ITEMS)
    put('B', 0x6010, ATT)
    for k in range(4):
        fz.preset_f32('ITM', 0x68 * k + 0x38, rng.uniform(0, 30))
        for j in range(3):
            fz.preset_f32('ATT', 0x3770 * k + 0x790 + 4 * j, rng.uniform(-5, 5))
    # attached objects
    entries = rng.randrange(0, 4)
    put('F', 0x69b8, KEYS)
    put('F', 0x69c0, KEYS + 16 * entries)
    put('F', 0x69a0, CNT)
    put('B', 0x60c8, RECS)
    for off, v in ((0, 0xffffffff), (4, 0), (8, 0xffffffff), (12, 0xffffffff)):
        fz.preset('KG', off, v)
    for k in range(4):
        same = rng.random() < 0.3
        fz.preset('KEYS', 16 * k, 0xffffffff if same else rng.randrange(0, 1 << 30))
        fz.preset('KEYS', 16 * k + 4, 0)
        fz.preset('KEYS', 16 * k + 8, 0xffffffff if same else rng.randrange(0, 1 << 30))
        fz.preset('KEYS', 16 * k + 12, 0xffffffff if same else 0)
        fz.preset('CNT', 16 * k, rng.randrange(0, 20))
        for j in (0x6c, 0x74, 0x7c):
            fz.preset_f32('RECS', 0x90 * k + j, rng.uniform(-5, 5))
        if rng.random() < 0.7:
            base = 0x3e40 * (k % 2)
            put('RECS', 0x90 * k + 0x30, ENT)
            put('RECS', 0x90 * k + 0x88, OBJ + base)
            fz.preset_f32('OBJ', base + 0x37d8, rng.uniform(0, 50))
            fz.preset('OBJ', base + 0x36e8, rng.choice([0, 1, 4, 5, 6, 7, 8, 9]))
            fz.preset_f32('OBJ', base + 0xa8, rng.uniform(0, 3))
            for off in (0x3e08, 0x3e0c, 0x3e10):
                fz.preset_f32('OBJ', base + off, rng.uniform(-5, 5))
            for off, v in ((0x37b0, 5), (0x37b4, 0), (0x37b8, 15), (0x37bc, 0)):
                fz.preset('OBJ', base + off, v)
        else:
            put('RECS', 0x90 * k + 0x30, 0)
            put('RECS', 0x90 * k + 0x88, 0)
    put('ENT', 0x178, SH)
    fz.preset_f32('ENT', 0xbc, rng.uniform(0, 50))
    for off in (0x788, 0x78c, 0x790):
        fz.preset_f32('SH', off, rng.uniform(-5, 5))
    for off, v in ((0x37b0, 5), (0x37b4, 0), (0x37b8, 15), (0x37bc, 0)):
        fz.preset('SH', off, v)
    case.stub(0x140f5c540, lambda call, rng: call.ret_int(rng.choice([0, 1, 1])))
    case.stub(0x140f42620, lambda call, rng: call.ret_int(ENT))


def setup_strut_world(case, rng, F, B, mesh_triangles=(8, 20), mesh_extent=12.0, mesh_heights=(-3.0, 3.0)):
    """Presets what 0x1411c7a50 needs around a flight object whose blocks already set the wheel vector, the
    matrices and the position: the terrain mesh, the weight and balance, the aircraft tables, the surface table
    with the wetness, the contact and recorder vectors and the replayed callees."""
    import math
    fz = case.fz

    def put(name, off, v):
        fz.preset(name, off, v & 0xffffffff)
        fz.preset(name, off + 4, v >> 32)
    setup_segment_mesh(case, rng, 'F', 0x42e40, triangles=mesh_triangles, heights=mesh_heights, extent=mesh_extent)
    setup_balance(case, rng, F, B)
    fz.region('TB', 0x1461243b0, 8)
    put('TB', 0, F)
    fz.region('TA', 0x146124398, 8)
    put('TA', 0, B)
    VEC = case.region('VEC', 0x18 * 6)
    n = rng.randrange(0, 3)
    put('F', 0x430d8, VEC)
    put('F', 0x430e0, VEC + 0x18 * n)
    put('F', 0x430e8, VEC + 0x18 * (n + rng.choice([0, 1, 2])))
    fz.region('RV', 0x146125768, 0x20)
    RCD = case.region('RCD', 0x50 * 4)
    m = rng.randrange(0, 3)
    put('RV', 0, RCD)
    put('RV', 8, RCD + 0x50 * m)
    put('RV', 16, RCD + 0x50 * (m + rng.choice([0, 1])))
    fz.region('RID', 0x142f2e3dc, 4)
    fz.preset('RID', 0, rng.choice([0, 1]))
    fz.region('SND', 0x142f01920, 8)
    fz.preset_f64('SND', 0, rng.choice([0.0, 2.0]))
    fz.region('WET', 0x142fe8518, 16)
    for off in (0, 4, 8, 12):
        fz.preset_f32('WET', off, rng.choice([0.0, rng.uniform(0, 1.2)]))
    ST = case.region('ST', 36 * 4)
    fz.region('SU', 0x14611ac80, 0x10)
    put('SU', 8, ST)
    for k in range(4):
        for off in (0x18, 0x1c, 0x20):
            fz.preset_f32('ST', 36 * k + off, rng.uniform(0.2, 2))
    for sn, cs in ((0x430, 0x434), (0x440, 0x444), (0x450, 0x454)):
        angle = rng.uniform(-0.6, 0.6)
        fz.preset_f32('F', sn, math.sin(angle))
        fz.preset_f32('F', cs, math.cos(angle))
    for off in (0x3cc, 0x3d0, 0x3d4):
        fz.preset_f32('F', off, rng.uniform(-1, 1))
    for off in (0x368, 0x36c, 0x370, 0x35c, 0x360, 0x364):
        fz.preset_f32('F', off, rng.uniform(-3, 3))
    for off in (0x378, 0x380, 0x388):
        fz.preset_f64('F', off, rng.uniform(-3, 3))
    for off in (0x3e0, 0x3dc, 0x3d8):
        fz.preset_f32('F', off, rng.uniform(-30, 30))
    fz.preset_f32('F', 0x288, rng.uniform(50, 1000))
    for off, lo, hi in ((0x2898, 0.5, 20), (0x27c0, 0.2, 2), (0x27c4, 0.2, 2), (0x64d8, 0.5, 5), (0x2828, 10, 500)):
        fz.preset_f32('B', off, rng.uniform(lo, hi))
    case.stub(0x140c51370, lambda call, rng: call.ret_int(0))
    case.stub(0x141a21200, lambda call, rng: None)

    def normal(call, rng):
        v = [rng.uniform(-1, 1) for _ in range(3)]
        length = sum(c * c for c in v) ** 0.5 or 1.0
        for k in range(3):
            call.put_f32(call.ints[3] + 4 * k, v[k] / length)
    case.stub(0x1406ed6a0, normal, caller=(0x14195ffc0, 0x141960dc0))
    NEW = case.region('NEW', 0x18 * 8)

    def grow(call, rng):
        vec, end, rec = call.ints[0], call.ints[1], call.ints[2]
        old_begin = case.emu.read_u64(vec)
        count = (end - old_begin) // 0x18
        for k in range(count * 6):
            call.put(NEW + 4 * k, case.emu.read_u32(old_begin + 4 * k))
        for k in range(6):
            call.put(NEW + 0x18 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
        for off, v in ((0, NEW), (8, NEW + 0x18 * (count + 1)), (16, NEW + 0x18 * (count + 4))):
            call.put(vec + off, v & 0xffffffff)
            call.put(vec + off + 4, v >> 32)
    case.stub(0x1405e73d0, grow)
    NEWR = case.region('NEWR', 0x50 * 8)

    def grow_rec(call, rng):
        vec, rec = call.ints[0], call.ints[2]
        old_begin = case.emu.read_u64(vec)
        end = case.emu.read_u64(vec + 8)
        count = (end - old_begin) // 0x50
        for k in range(count * 20):
            call.put(NEWR + 4 * k, case.emu.read_u32(old_begin + 4 * k))
        for k in range(20):
            call.put(NEWR + 0x50 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
        for off, v in ((0, NEWR), (8, NEWR + 0x50 * (count + 1)), (16, NEWR + 0x50 * (count + 4))):
            call.put(vec + off, v & 0xffffffff)
            call.put(vec + off + 4, v >> 32)
    case.stub(0x140f0f660, grow_rec)


def setup_wheel_world(case, rng, F, B, gears=3, wheel_records=None):
    """Presets what the native wheel model 0x1411c8690 reads on top of setup_strut_world: the planet, the globals
    of the surface, the gear and wheel records (`gears` of them; `wheel_records` is a list of
    (name, offset) places of the wheel records, the default creates a region), the objects those point to and the
    input-binding query for the idents 0x88 + gear. Returns (G, WRS) with the gear region and the wheel record
    addresses."""
    fz = case.fz

    def put(name, off, v):
        fz.preset(name, off, v & 0xffffffff)
        fz.preset(name, off + 4, v >> 32)
    for off in (0xbcc8, 0xbcd0):
        fz.preset('F', off, 0)
    fz.preset('F', 0x28, 0)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.05, 0.2, rng.uniform(0, 0.5)])))
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
    case.stub(0x140c51370, lambda call, rng: call.ret_int(0))
    CX = case.region('CX', 0x300)
    case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
    case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
    fz.preset_f64('CX', 0xb0, 6378137.0)
    fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
    fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
    fz.preset_f64('CX', 0xd0, rng.uniform(0.0, 0.01))
    for off in range(0x200, 0x278, 8):
        fz.preset_f64('CX', off, rng.uniform(-3.0, 3.0))
    G = case.region('G', 0x88 * gears)
    put('B', 0x6080, G)
    for k in range(gears):
        fz.preset('G', 0x88 * k, rng.choice([0, 1, 2, 3, 3, 4, 4, 5, 6, 7, 8]))
        fz.preset('G', 0x88 * k + 4, rng.choice([0, 1]))
        fz.preset('G', 0x88 * k + 0xc, rng.choice([0, 0, 1]))
        fz.preset_f32('G', 0x88 * k + 0x24, rng.uniform(-20, 20))
        fz.preset_f32('G', 0x88 * k + 0x28, rng.uniform(1000, 20000))
        fz.preset_f32('G', 0x88 * k + 0x2c, rng.uniform(0.05, 0.4))
        fz.preset_f32('G', 0x88 * k + 0x30, rng.uniform(1000, 30000))
        fz.preset_f32('G', 0x88 * k + 0x34, rng.choice([rng.uniform(0.2, 0.7), 0.0]))
        fz.preset_f32('G', 0x88 * k + 0x38, rng.uniform(0, 3000))
        fz.preset_f32('G', 0x88 * k + 0x3c, rng.uniform(0, 100))
        fz.preset_f32('G', 0x88 * k + 0x50, rng.choice([0.0, rng.uniform(0, 2), 0.005]))
        fz.preset_f32('G', 0x88 * k + 0x54, rng.choice([0.0, rng.uniform(0, 2), 0.005]))
        fz.preset_f32('G', 0x88 * k + 0x58, rng.uniform(0.1, 0.8))
        fz.preset_f32('G', 0x88 * k + 0x60, rng.uniform(0.1, 3))
        fz.preset_f32('G', 0x88 * k + 0x7c, rng.choice([0.0, 0.3, -0.3]))
    if wheel_records is None:
        WRS = [case.region('WR%d' % k, 0x90) for k in range(gears)]
        names = ['WR%d' % k for k in range(gears)]
        offs = [0] * gears
    else:
        names = [n for n, _ in wheel_records]
        offs = [o for _, o in wheel_records]
        WRS = [case.fz.regions[n][0] + o for n, o in wheel_records]
    for k in range(gears):
        Q = case.region('Q%d' % k, 0x80)
        BW = case.region('BW%d' % k, 0x2900)
        n, o = names[k], offs[k]
        put(n, o, Q)
        put(n, o + 8, BW)
        for off, lo, hi in ((0x10, 0.0, 1.2), (0x14, -30, 30), (0x18, -30, 30), (0x24, -30, 30), (0x28, -30, 30),
                            (0x40, -3, 3), (0x44, -3, 3), (0x48, -50, 50), (0x50, 0.0, 1.0), (0x54, 0.0, 1.2),
                            (0x58, 0, 2), (0x5c, 0, 5), (0x60, -1, 3), (0x64, 0, 1), (0x68, 0, 10)):
            fz.preset_f32(n, o + off, rng.uniform(lo, hi))
        fz.preset(n, o + 0x4c, rng.choice([0, 0, 1]))
        fz.preset_f64(n, o + 0x80, rng.uniform(-8, 8))
        fz.preset_f64(n, o + 0x88, rng.uniform(-8, 8))
        fz.preset(n, o + 0x7c, rng.choice([0, 1]))
        for off, lo, hi in ((0x18, 0.3, 1.5), (0x20, 0.0, 0.5), (0x64, -3, 3), (0x70, -3, 3), (0x7c, -3, 3)):
            fz.preset_f32('Q%d' % k, off, rng.uniform(lo, hi))
        for off, lo, hi in ((0x2838, 5, 100), (0x2898, 0.5, 20), (0x282c, 0.5, 5)):
            fz.preset_f32('BW%d' % k, off, rng.uniform(lo, hi))
    for off, lo, hi in ((0x27cc, 0.1, 0.6), (0x283c, 0.1, 3), (0x65a8, -20, 20)):
        fz.preset_f32('B', off, rng.uniform(lo, hi))
    fz.preset_f32('B', 0x27bc, rng.choice([0.0, 0.0, rng.uniform(0.1, 30)]))
    fz.preset_f32('B', 0x1e90, rng.choice([0.0, rng.uniform(0.5, 5)]))
    fz.preset('B', 0xe98, rng.choice([0, 1, 1]))
    for off, lo, hi in ((0x42f50, 0.2, 1.5), (0x358, -30, 30), (0x408, -30, 30), (0x41c, 0, 5),
                        (0x240, -1, 1), (0x244, -1, 1)):
        fz.preset_f32('F', off, rng.uniform(lo, hi))
    for off, choices in ((0x200, [0, 1]), (0x204, [0, 1]), (0x214, [0, 0, 1])):
        fz.preset('F', off, rng.choice(choices))
    fz.preset_f32('F', 0x210, rng.choice([0.0, 0.3, 0.8]))
    fz.preset_f32('F', 0x218, rng.choice([0.0, 0.0, 0.5]))
    fz.region('TD', 0x142f01910, 8)
    fz.preset_f64('TD', 0, rng.uniform(0, 1000))
    ENG = case.region('ENG', 0x68)
    OBJ = case.region('OBJ', 0x40)
    ENGL = case.region('ENGL', 0x2cc)
    put('B', 0x5ff8, ENG)
    fz.preset('ENG', 0, rng.choice([0, 1, 2, 3, 5]))
    put('B', 0x1d18, rng.choice([0, OBJ]))
    put('F', 0x68b0, ENGL)
    fz.preset_f32('ENGL', 0x218, rng.choice([0.0, 0.3, 1.0, -0.5]))
    for off in (0x55c, 0x564, 0x5ac, 0x5b0, 0x5b4, 0x5b8, 0x5bc, 0x5c0, 0x5c4, 0x5f0, 0x5f4):
        fz.preset('F', off, rng.choice([0, 0, 1, 2, 5]))
    for k in range(6):
        fz.preset_f32('F', 0xd18 + 4 * k, rng.choice([-1.0, 0.0, rng.uniform(-0.5, 1.5), 1.0]))
        fz.preset_f32('F', 0xd30 + 4 * k, rng.uniform(-2, 2))
    fz.preset('B', 0x918, rng.choice([0, 1, 1]))
    fz.preset('B', 0x91c, rng.choice([0, 1, 1, 2]))
    fz.preset('B', 0xad8, rng.choice([-1, 0, 3, 12, 13, 40]) & 0xffffffff)
    fz.preset('B', 0xc90, rng.randrange(0, 8))
    fz.preset_f32('B', 0x1b7c, rng.choice([0.0, 0.5, 1.0, 0.2]))
    for gear in range(gears):
        ident = 0x88 + gear
        fz.preset('F', 0xdc0 + 4 * ident, rng.choice([0, 0, 0, 6, 1]))
        byte_at = 0x55f0 + ident
        fz.preset('F', byte_at & ~3, rng.choice([0, 0, 1]) << (8 * (byte_at & 3)), record=True)
        mask_at = 0x2998 + ident
        fz.preset('B', mask_at & ~3, rng.choice([0, 1, 2, 3, 4, 8, 0x10, 0x20, 0x3f, 0x25]) << (8 * (mask_at & 3)), record=True)
        fz.preset_f32('B', 0x2c9c + 4 * ident, rng.uniform(-2, 2))
        fz.preset('IT2', 4 * ident, case.emu.read_u32(0x142515560 + 4 * ident))
    for k in range(13):
        fz.preset('IT3', 40 * k, case.emu.read_u32(0x142515364 + 40 * k))
    fz.region('GT', 0x142f01918, 8)
    fz.preset_f64('GT', 0, rng.choice([0.0, 0.5, 0.9, -1.0]))
    case.stub(0x14082be10, lambda call, rng: call.ret_int(rng.choice([0, 1])))
    return G, WRS


def setup_blend_world(case, rng, F, B):
    """Presets what the native blend contact 0x1411cb1e0 reads around the flight object (on top of
    setup_strut_world): the planet, the geoid, the engine flag, the debug-line vector, the grid global and the
    replayed callees."""
    fz = case.fz

    def put(name, off, v):
        fz.preset(name, off, v & 0xffffffff)
        fz.preset(name, off + 4, v >> 32)

    def normal(call, rng):
        v = [rng.uniform(-1, 1) for _ in range(3)]
        length = sum(c * c for c in v) ** 0.5 or 1.0
        for k in range(3):
            call.put_f32(call.ints[3] + 4 * k, v[k] / length)
    case.stub(0x1406ed6a0, normal)
    for off in (0xbcc8, 0xbcd0):
        fz.preset('F', off, 0)
    fz.preset('F', 0x28, 0)
    case.stub(0x140c448c0, lambda call, rng: call.ret_f64(rng.choice([0.05, 0.2, rng.uniform(0, 0.5)])))
    case.stub(0x1417f12c0, lambda call, rng: call.ret_int(rng.choice([0, 0, 1])))
    CX = case.region('CX', 0x300)
    case.stub(0x14193ae40, lambda call, rng: call.ret_int(CX))
    case.stub(0x1419f8ff0, lambda call, rng: call.ret_f64(rng.uniform(-60, 60)))
    fz.preset_f64('CX', 0xb0, 6378137.0)
    fz.preset_f64('CX', 0xb8, rng.uniform(6350000.0, 6378000.0))
    fz.preset_f64('CX', 0xc8, rng.uniform(0.0, 0.01))
    fz.preset_f64('CX', 0xd0, rng.uniform(0.0, 0.01))
    for base in (0x200, 0x280):
        for r in range(4):
            for c in range(4):
                v = (1.0 if r == c and r < 3 else 0.0) + (rng.uniform(-0.1, 0.1) if r < 3 and c < 3 else (rng.uniform(-5, 5) if r == 3 and c < 3 else 0.0))
                if r == 3 and c == 3:
                    v = 1.0
                fz.preset_f64('CX', base + 0x20 * r + 8 * c, v)
    fz.preset_f64('F', 0x390, rng.uniform(-80, 80))
    fz.preset_f64('F', 0x398, rng.uniform(-170, 170))
    fz.region('GRID', 0x1461179f8, 8)
    fz.preset('GRID', 0, 0)
    fz.preset('GRID', 4, 0)
    fz.region('IDX', 0x1426552ac, 4)
    fz.preset('IDX', 0, 0)
    for off in (0x64f4, 0x64f8, 0x64fc):
        fz.preset_f32('B', off, rng.uniform(-30, 30))
    fz.preset_f32('F', 0x3e0, rng.uniform(-180, 180))
    fz.region('LV', 0x146125780, 0x20)
    LREC = case.region('LREC', 0x28 * 6)
    m = rng.randrange(0, 3)
    put('LV', 0, LREC)
    put('LV', 8, LREC + 0x28 * m)
    put('LV', 16, LREC + 0x28 * (m + rng.choice([0, 1])))
    NEWL = case.region('NEWL', 0x28 * 8)

    def grow(call, rng):
        vec, rec = call.ints[0], call.ints[2]
        old_begin = case.emu.read_u64(vec)
        end = case.emu.read_u64(vec + 8)
        count = (end - old_begin) // 0x28
        for k in range(count * 10):
            call.put(NEWL + 4 * k, case.emu.read_u32(old_begin + 4 * k))
        for k in range(10):
            call.put(NEWL + 0x28 * count + 4 * k, case.emu.read_u32(rec + 4 * k))
        for off, v in ((0, NEWL), (8, NEWL + 0x28 * (count + 1)), (16, NEWL + 0x28 * (count + 4))):
            call.put(vec + off, v & 0xffffffff)
            call.put(vec + off + 4, v >> 32)
    case.stub(0x141167000, grow)


def setup_blend_body(case, rng, name, base, skip=(0x654, 0x658)):
    """Presets one body record (at `base` in region `name`) for the blend contact: the smoothed values, the euler
    offsets and angles of its gear record (`+0x588`) and the vertices of the array at `+0x630`."""
    fz = case.fz
    fz.preset(name, base + 0x2c, rng.choice([0, 1]))
    fz.preset_f32(name, base + 0x38, rng.uniform(0, 3))
    fz.preset_f32(name, base + 0x44, rng.uniform(-1, 1))
    for off in (0x588 + 0x90, 0x588 + 0x94, 0x588 + 0x98):
        fz.preset_f32(name, base + off, rng.uniform(-1, 1))
    for off in (0x588 + 0x9c, 0x588 + 0xa0, 0x588 + 0xa4):
        fz.preset_f32(name, base + off, rng.uniform(-40, 40))
    for k in range(0x630, 0x630 + 12 * 140, 4):
        if k - 0x630 in skip:
            continue
        fz.preset_f32(name, base + k, rng.uniform(-1.5, 1.5))
