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

    def stub(self, address, handler):
        def run(e):
            call = Call(self, address)
            handler(call, self.rng)
            e.uc.reg_write(UC_X86_REG_RAX, call.rax)
            e.uc.reg_write(UC_X86_REG_XMM0, call.xmm0)
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
