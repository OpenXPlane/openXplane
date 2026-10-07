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
        cell = case.region('GC', 16)
        fz.preset_f32('GC', 0, rng.uniform(-3, 3))
        fz.preset_f32('GC', 4, rng.uniform(-3, 3))
        fz.preset_f32('GR', 8, rng.choice([1.0, 4.0, 20.0]))
        fz.preset('GR', 0x48, cell & 0xffffffff)
        fz.preset('GR', 0x4c, cell >> 32)
    fz.region('GL', GRID_GLOBAL, 8)
    fz.preset('GL', 0, grid & 0xffffffff)
    fz.preset('GL', 4, grid >> 32)


def stub_terrain_helpers(case, caller=(0x141960dc0, 0x141961200)):
    """Replaces the helpers of the probe that are not ported: the grid height and the triangle normal."""
    case.stub(0x14194dc20, lambda call, rng: call.ret_f32(rng.uniform(-3, 3)), caller=caller)

    def normal(call, rng):
        for k in range(3):
            call.put_f32(call.ints[3] + 4 * k, rng.uniform(-1, 1))
    case.stub(0x1406ed6a0, normal, caller=caller)


SURFACES = 0x14611ac88


def setup_segment_mesh(case, rng, mesh_name='MS', mesh_off=0, triangles=(10, 26)):
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
            lo, hi = (-8.0, 25.0) if i % 3 == 1 else (-12.0, 12.0)
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
