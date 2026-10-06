#!/usr/bin/env python3
"""Runs an original function in the emulator on lazily random-filled objects and records what it touched.

The objects of the original (the flight object `F`, the aircraft object `B`, record arrays, an output record) are
regions of emulator memory that start zeroed. The first read of a 4-byte word the function makes is intercepted
and the word is filled with a random value of the kind the reading instruction implies (float, integer, double),
unless a policy or a preset says otherwise. The result is an exact, reproducible initial state (the words that
were read) plus the final value of every word the function wrote, so a port that reads the same offsets through
a sparse map can be compared word for word.

    fz = Fuzz(emu, 'Xplane12/X-Plane.exe', [(0x1411bd470, 0x1411c3584)], seed)
    fz.region('F', addr, size); ...
    fz.preset('F', 0x20, pointer_low_word) ...
    fz.run(function_address, ints=[...], floats=[...], stack=[...])
    fz.initial()   -> {region: {offset: word}}
    fz.written()   -> {region: {offset: final word}}
"""
import random
import re
import struct
import subprocess
from pathlib import Path

from unicorn import UC_HOOK_MEM_READ, UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_RIP

FLOAT_OPS = {'movss', 'addss', 'subss', 'mulss', 'divss', 'comiss', 'ucomiss', 'maxss', 'minss', 'cvtss2sd',
             'cvtps2pd', 'sqrtss', 'andps', 'xorps', 'orps', 'movups', 'movaps', 'addps', 'mulps', 'subps'}
DOUBLE_OPS = {'movsd', 'addsd', 'subsd', 'mulsd', 'divsd', 'comisd', 'ucomisd', 'maxsd', 'minsd', 'cvtsd2ss',
              'sqrtsd', 'cvtsd2si', 'cvttsd2si', 'andpd', 'xorpd'}


def disassemble(exe, ranges):
    """{address: mnemonic} for the given [lo, hi) ranges (llvm-objdump)."""
    table = {}
    for lo, hi in ranges:
        out = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={lo:#x}',
                              f'--stop-address={hi:#x}', str(exe)], capture_output=True, text=True).stdout
        for line in out.splitlines():
            m = re.match(r'\s*([0-9a-f]+):\s+(\S+)', line)
            if m:
                table[int(m.group(1), 16)] = m.group(2)
    return table


class Fuzz:
    def __init__(self, emu, exe, ranges, seed=1):
        self.emu = emu
        self.ops = disassemble(exe, ranges)
        self.rng = random.Random(seed)
        self.regions = {}
        self.policy = {}
        self._known = {}
        self._init = {}
        self._written = {}
        self.reads = 0
        emu.uc.hook_add(UC_HOOK_MEM_READ, self._on_read)
        emu.uc.hook_add(UC_HOOK_MEM_WRITE, self._on_write)

    def region(self, name, addr, size):
        self.regions[name] = (addr, size)
        self._known[name] = {}
        self._init[name] = {}
        self._written[name] = set()

    def reset(self):
        for name in self.regions:
            addr, size = self.regions[name]
            self.emu.write(addr, bytes(size))
            self._known[name] = {}
            self._init[name] = {}
            self._written[name] = set()

    def preset(self, name, offset, word, record=True):
        """Sets a word before the run. Recorded in the initial state unless `record` is false."""
        addr, _ = self.regions[name]
        self.emu.write_u32(addr + offset, word)
        self._known[name][offset] = True
        if record:
            self._init[name][offset] = word & 0xffffffff

    def preset_f32(self, name, offset, value):
        self.preset(name, offset, struct.unpack('<I', struct.pack('<f', value))[0])

    def preset_f64(self, name, offset, value):
        low, high = struct.unpack('<II', struct.pack('<d', value))
        self.preset(name, offset, low)
        self.preset(name, offset + 4, high)

    def initial(self):
        return self._init

    def written(self):
        out = {}
        for name, offsets in self._written.items():
            addr, _ = self.regions[name]
            out[name] = {o: self.emu.read_u32(addr + o) for o in sorted(offsets)}
        return out

    # ---- hooks ----------------------------------------------------------------------------------
    def _locate(self, address):
        for name, (addr, size) in self.regions.items():
            if addr <= address < addr + size:
                return name, address - addr
        return None, 0

    def _fill(self, name, offset, size, mnemonic):
        known = self._known[name]
        base = offset & ~3
        end = (offset + size + 3) & ~3
        word_ops = size == 4 and mnemonic in FLOAT_OPS
        for off in range(base, end, 4):
            if known.get(off):
                continue
            custom = self.policy.get((name, off))
            if custom is not None:
                word = custom(self.rng)
            elif mnemonic in DOUBLE_OPS and size == 8:
                if off == base:
                    low, high = struct.unpack('<II', struct.pack('<d', self.rng.uniform(-100, 100)))
                    self._set(name, off, low)
                    self._set(name, off + 4, high)
                continue
            elif size == 8 or mnemonic in ('movq',):
                word = self.rng.randrange(0, 3)
            elif size == 1:
                word = self.rng.randrange(0, 2)
            elif word_ops or size == 16:
                word = struct.unpack('<I', struct.pack('<f', self.rng.uniform(-2.0, 2.0)))[0]
            else:
                word = self.rng.randrange(0, 4)
            self._set(name, off, word)

    def _set(self, name, off, word):
        addr, _ = self.regions[name]
        if self._known[name].get(off):
            return
        self.emu.write_u32(addr + off, word)
        self._known[name][off] = True
        self._init[name][off] = word & 0xffffffff

    def _on_read(self, uc, access, address, size, value, user):
        name, offset = self._locate(address)
        if name is None:
            return
        pc = uc.reg_read(UC_X86_REG_RIP)
        self.reads += 1
        self._fill(name, offset, size, self.ops.get(pc, ''))

    def _on_write(self, uc, access, address, size, value, user):
        name, offset = self._locate(address)
        if name is None:
            return
        for off in range(offset & ~3, (offset + size + 3) & ~3, 4):
            self._written[name].add(off)
            self._known[name].setdefault(off, True)
