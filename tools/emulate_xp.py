#!/usr/bin/env python3
"""Run functions of the reference X-Plane.exe in a CPU emulator (Unicorn) on synthetic inputs.

This executes the original machine code of one function in isolation, with the PE sections mapped at
their virtual addresses and a scratch heap and stack, so a port can be compared against the original
arithmetic without running X-Plane. Callees that need engine state can be replaced by Python stubs.

Windows x64 calling convention: integer arguments in RCX, RDX, R8, R9, floating point arguments in
XMM0-XMM3 (by position), further arguments on the stack after 32 bytes of shadow space, results in
RAX / XMM0. Requires: pip install unicorn numpy.

    from emulate_xp import Emulator
    emu = Emulator('Xplane12/X-Plane.exe')
    value = emu.call_float(0x1406ea0b0, floats=[0.0, 1.0, 10.0, 3.0], stack=[5.0])
"""
import struct
from pathlib import Path

from unicorn import UC_ARCH_X86, UC_MODE_64, UC_HOOK_CODE, UC_HOOK_MEM_UNMAPPED, Uc, UcError
from unicorn.x86_const import (UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RAX, UC_X86_REG_RCX,
                               UC_X86_REG_RDX, UC_X86_REG_RIP, UC_X86_REG_RSP, UC_X86_REG_XMM0,
                               UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3)

PAGE = 0x1000
STACK_TOP = 0x7ff000000000
STACK_SIZE = 0x200000
HEAP_BASE = 0x6f0000000000
HEAP_SIZE = 0x4000000
RET_ADDR = 0x6e0000000000  # a mapped page holding a single RET; calls return here
XMM = [UC_X86_REG_XMM0, UC_X86_REG_XMM1, UC_X86_REG_XMM2, UC_X86_REG_XMM3]
INT = [UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_R8, UC_X86_REG_R9]


def _align_down(v):
    return v & ~(PAGE - 1)


def _align_up(v):
    return (v + PAGE - 1) & ~(PAGE - 1)


class Emulator:
    def __init__(self, exe_path):
        self.data = Path(exe_path).read_bytes()
        d = self.data
        e = struct.unpack_from('<I', d, 0x3c)[0]
        nsec = struct.unpack_from('<H', d, e + 6)[0]
        opt = struct.unpack_from('<H', d, e + 20)[0]
        self.base = struct.unpack_from('<Q', d, e + 24 + 24)[0]
        self.uc = Uc(UC_ARCH_X86, UC_MODE_64)
        o = e + 24 + opt
        self.sections = []
        for _ in range(nsec):
            name = d[o:o + 8].rstrip(b'\0').decode()
            vsize, va, rsize, roff = struct.unpack_from('<IIII', d, o + 8)
            self.sections.append((name, self.base + va, max(vsize, rsize), roff, rsize))
            o += 40
        for name, va, size, roff, rsize in self.sections:
            lo, hi = _align_down(va), _align_up(va + size)
            self._map(lo, hi - lo)
            self.uc.mem_write(va, d[roff:roff + min(rsize, size)])
        self._map(STACK_TOP - STACK_SIZE, STACK_SIZE)
        self._map(HEAP_BASE, HEAP_SIZE)
        self._map(RET_ADDR, PAGE)
        self.uc.mem_write(RET_ADDR, b'\xc3')
        self.heap_top = HEAP_BASE
        self.stubs = {}
        self.trace = []
        self.uc.hook_add(UC_HOOK_CODE, self._on_code)
        self.uc.hook_add(UC_HOOK_CODE, lambda uc, address, size, user: uc.emu_stop(), begin=RET_ADDR, end=RET_ADDR)
        self.uc.hook_add(UC_HOOK_MEM_UNMAPPED, self._on_unmapped)

    def _map(self, addr, size):
        try:
            self.uc.mem_map(addr, size)
        except UcError:
            pass  # overlaps an earlier section page

    def _on_unmapped(self, uc, access, address, size, value, user):
        raise RuntimeError(f'unmapped access at {address:#x} from {uc.reg_read(UC_X86_REG_RIP):#x}')

    def _on_code(self, uc, address, size, user):
        stub = self.stubs.get(address)
        if stub is not None:
            if stub(self) is True:
                return  # the stub declined this call: the function runs natively
            rsp = uc.reg_read(UC_X86_REG_RSP)
            ret = struct.unpack('<Q', uc.mem_read(rsp, 8))[0]
            uc.reg_write(UC_X86_REG_RSP, rsp + 8)
            uc.reg_write(UC_X86_REG_RIP, ret)

    # ---- memory helpers -------------------------------------------------------------------------
    def alloc(self, size, align=16):
        self.heap_top = (self.heap_top + align - 1) & ~(align - 1)
        addr = self.heap_top
        self.heap_top += size
        if self.heap_top > HEAP_BASE + HEAP_SIZE:
            raise MemoryError('emulator heap exhausted')
        return addr

    def write(self, addr, raw):
        self.uc.mem_write(addr, bytes(raw))

    def read(self, addr, size):
        return bytes(self.uc.mem_read(addr, size))

    def write_f32(self, addr, value):
        self.write(addr, struct.pack('<f', value))

    def read_f32(self, addr):
        return struct.unpack('<f', self.read(addr, 4))[0]

    def write_u32(self, addr, value):
        self.write(addr, struct.pack('<I', value & 0xffffffff))

    def write_u64(self, addr, value):
        self.write(addr, struct.pack('<Q', value))

    def read_u32(self, addr):
        return struct.unpack('<I', self.read(addr, 4))[0]

    def read_u64(self, addr):
        return struct.unpack('<Q', self.read(addr, 8))[0]

    def reg(self, which):
        return self.uc.reg_read(which)

    def set_xmm_f32(self, index, value):
        self.uc.reg_write(XMM[index], struct.unpack('<I', struct.pack('<f', value))[0])

    # ---- calling --------------------------------------------------------------------------------
    def call(self, addr, ints=(), floats=(), stack=(), max_instructions=2_000_000, until=None):
        """Calls a function. `ints` go to RCX, RDX, R8, R9; `floats` to XMM0-3 (as float32); `stack`
        holds the 5th and later arguments, each a float32 (float) or a 64-bit integer (int)."""
        uc = self.uc
        sp = STACK_TOP - 0x1000
        sp = (sp - 0x20 - 8 * len(stack)) & ~0xf
        for i, v in enumerate(ints):
            uc.reg_write(INT[i], v & 0xffffffffffffffff)
        for i, v in enumerate(floats):
            self.set_xmm_f32(i, v)
        for i, v in enumerate(stack):
            slot = sp + 0x20 + 8 * i
            if isinstance(v, float):
                self.write(slot, struct.pack('<fI', v, 0))
            else:
                self.write_u64(slot, v)
        sp -= 8
        self.write_u64(sp, RET_ADDR)
        uc.reg_write(UC_X86_REG_RSP, sp)
        uc.reg_write(UC_X86_REG_RIP, addr)
        try:
            uc.emu_start(addr, RET_ADDR if until is None else until, count=max_instructions)
        except UcError as error:
            raise RuntimeError(f'emulation failed in {addr:#x} at {uc.reg_read(UC_X86_REG_RIP):#x}: {error}')
        return uc.reg_read(UC_X86_REG_RAX)

    def xmm0_f32(self):
        return struct.unpack('<f', (self.uc.reg_read(UC_X86_REG_XMM0) & 0xffffffff).to_bytes(4, 'little'))[0]

    def call_float(self, addr, **kwargs):
        self.call(addr, **kwargs)
        return self.xmm0_f32()
