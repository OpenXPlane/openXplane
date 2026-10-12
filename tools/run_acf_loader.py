#!/usr/bin/env python3
"""Runs the original ACF loader (0x141a972a0) in the emulator on a .acf file.

    python3 tools/run_acf_loader.py Xplane12/X-Plane.exe "Xplane12/Cessna 172 SP/Cessna_172SP.acf" > out.txt

The loader asks a property context for each key (0x140c99910: binary search in a sorted vector of 32-byte entries
{key ptr, key len, value ptr, value len} under the current path prefix); the context is built here from the file
so the original reading code (number parser, string reader, scopes) runs unchanged. The aircraft object and the
sub-objects it fills are zeroed regions, so a property absent from the file leaves its word zero (the original's
constructor defaults are not applied here). The output is `REGION OFFSET=WORD` lines (`B` for the aircraft
object, `R<pointer-offset>` for each sub-object region); with `XP_READS=file` every reader call is recorded as
`kind key region offset coeff-bits final-word`.

As a module: `run(exe_path, entries)` with `entries` a dict of key bytes to value bytes returns
`(words, reads, error)`; `words` maps (region, offset) to the final word.
"""
import os
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn import UC_HOOK_CODE  # noqa: E402
from unicorn.x86_const import (UC_X86_REG_R8, UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX,  # noqa: E402
                               UC_X86_REG_XMM3, UC_X86_REG_RIP)

POINTERS = {0x5ff8: 0x68 * 8, 0x6010: 0x3770 * 8, 0x6028: 0x36c8 * 48, 0x6040: 0x34c8 * 39, 0x6080: 0x88 * 10,
            0x60f8: 0x548 * 64}
B_SIZE = 0x6700
READERS = {0x140c99f40: 'float', 0x140c99dc0: 'coeff', 0x140c9a020: 'int', 0x140c9a240: 'string'}


def parse_acf(path):
    entries = {}
    for line in Path(path).read_text(errors='replace').splitlines():
        if line.startswith('P '):
            key, _, value = line[2:].partition(' ')
            entries[key.encode()] = value.encode()
    return entries


def build_context(emu, entries):
    """The property context: mode word 0 (reading), sorted entry vector at +0x20..+0x28, empty prefix at +0x70."""
    ctx = emu.alloc(0x400)
    emu.write(ctx, bytes(0x400))
    n = len(entries)
    vec = emu.alloc(32 * n + 32)
    for i, key in enumerate(sorted(entries)):
        value = entries[key]
        kp = emu.alloc(len(key) + 1)
        emu.write(kp, key + b'\0')
        vp = emu.alloc(len(value) + 1)
        emu.write(vp, value + b'\0')
        emu.write(vec + 32 * i, struct.pack('<QQQQ', kp, len(key), vp, len(value)))
    emu.write_u64(ctx + 0x20, vec)
    emu.write_u64(ctx + 0x28, vec + 32 * n)
    emu.write_u64(ctx + 0x80, 0)
    emu.write_u64(ctx + 0x88, 0xf)
    stack = emu.alloc(32 * 64)
    emu.write(stack, bytes(32 * 64))
    emu.write_u64(ctx + 0x90, stack)
    emu.write_u64(ctx + 0x98, stack)
    emu.write_u64(ctx + 0xa0, stack + 32 * 64)
    marks = emu.alloc(16 * 64)
    emu.write(marks, bytes(16 * 64))
    emu.write_u64(ctx + 0x30, marks)
    emu.write_u64(ctx + 0x38, marks)
    emu.write_u64(ctx + 0x40, marks + 16 * 64)
    return ctx


def run(exe, entries, trace_reads=False, emu=None, trace_writes=None):
    emu = emu or Emulator(exe)
    emu.stubs[0x1422e8180] = lambda e: None  # the stack probe reads the TEB

    def new(e):  # operator new
        size = e.reg(UC_X86_REG_RCX)
        e.uc.reg_write(UC_X86_REG_RAX, e.alloc(max(size, 16) + 16, 16))
    emu.stubs[0x1422e7bf0] = new
    emu.stubs[0x1422e7c2c] = lambda e: None  # operator delete
    mark = emu.heap_top
    ctx = build_context(emu, entries)
    b = emu.alloc(B_SIZE)
    emu.write(b, bytes(B_SIZE))
    regions = {}
    for off, size in POINTERS.items():
        r = emu.alloc(size + 0x1000)
        emu.write(r, bytes(size + 0x1000))
        emu.write_u64(b + off, r)
        regions[off] = (r, size + 0x1000)
    reads = []
    handles = []
    if trace_reads:
        def on_reader(uc, address, size, user):
            c, dest, r8 = uc.reg_read(UC_X86_REG_RCX), uc.reg_read(UC_X86_REG_RDX), uc.reg_read(UC_X86_REG_R8)
            ptr, ln = struct.unpack('<QQ', bytes(uc.mem_read(r8, 16)))
            key = bytes(uc.mem_read(ptr, ln)).decode()
            plen = struct.unpack('<Q', bytes(uc.mem_read(c + 0x80, 8)))[0]
            cap = struct.unpack('<Q', bytes(uc.mem_read(c + 0x88, 8)))[0]
            pp = c + 0x70 if cap <= 0xf else struct.unpack('<Q', bytes(uc.mem_read(c + 0x70, 8)))[0]
            prefix = bytes(uc.mem_read(pp, plen)).decode()
            coeff = uc.reg_read(UC_X86_REG_XMM3) & 0xffffffff if address == 0x140c99dc0 else 0
            reads.append((READERS[address], prefix + key, dest, coeff))
        for a in READERS:
            handles.append(emu.uc.hook_add(UC_HOOK_CODE, on_reader, begin=a, end=a))
    if trace_writes is not None:
        from unicorn import UC_HOOK_MEM_WRITE
        lo = min(r for r, _ in regions.values())
        hi = max(r + s for r, s in regions.values())

        def on_write(uc, access, address, size, value, user):
            trace_writes[address] = uc.reg_read(UC_X86_REG_RIP)
        handles.append(emu.uc.hook_add(UC_HOOK_MEM_WRITE, on_write, begin=min(lo, b), end=max(hi, b + B_SIZE)))
        trace_writes['base'] = (b, regions)
    error = None
    try:
        emu.call(0x141a972a0, ints=[b, ctx], max_instructions=60_000_000)
    except RuntimeError as err:
        error = str(err)
    for h in handles:
        emu.uc.hook_del(h)
    areas = [('B', b, B_SIZE)] + [(f'R{off:x}', r, size) for off, (r, size) in regions.items()]
    words = {}
    for name, base, size in areas:
        raw = emu.read(base, size)
        for o in range(0, size, 4):
            w = struct.unpack_from('<I', raw, o)[0]
            if w:
                words[(name, o)] = w
    located = []
    for kind, key, dest, coeff in reads:
        where = ('?', dest)
        for name, base, size in areas:
            if base <= dest < base + size:
                where = (name, dest - base)
        located.append((kind, key, where[0], where[1], coeff))
    emu.heap_top = mark
    return words, located, error


def main():
    exe, acf = sys.argv[1], sys.argv[2]
    words, reads, error = run(exe, parse_acf(acf), trace_reads=bool(os.environ.get('XP_READS')))
    if error:
        sys.stderr.write(f'loader stopped: {error}\n')
    if os.environ.get('XP_READS'):
        with open(os.environ['XP_READS'], 'w') as f:
            for kind, key, name, off, coeff in reads:
                final = words.get((name, off), 0) if name != '?' else 0
                f.write(f'{kind}\t{key}\t{name}\t{off:x}\t{coeff:x}\t{final:08x}\n')
    for (name, off), w in sorted(words.items()):
        print(f'{name} {off:x}={w:08x}')


if __name__ == '__main__':
    main()
