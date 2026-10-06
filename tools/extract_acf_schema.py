#!/usr/bin/env python3
"""Extract the ACF property loading schema from the reference X-Plane.exe without running it.

The loader functions read one property per block of straight-line code:

  lea  <key string>(%rip), %rax|%rdx        ; the property key, a string in .rdata
  ...  lea OFF(%reg), %rdx  |  addq $OFF, %rdx   ; destination field (offset in the object)
  ...  movss <const>(%rip), %xmmN ; movaps %xmmN, %xmm3       ; optional unit coefficient
  call <reader>                            ; reads the value of that key and stores it

For each block this prints: key, loader function, destination base, offset, reader function and the
float coefficient in xmm3 when one was loaded. xmm registers are tracked linearly through a function,
so a coefficient loaded once and reused by later blocks is carried forward; branches can break that,
so the coefficient is a heuristic that must be confirmed against the code, as done in
research/ACF_LOADING.md. Requires llvm-objdump on PATH.
"""
import argparse
import re
import struct
import subprocess
from collections import Counter
from pathlib import Path

LOADERS = [(0x141a972a0, 0x141aaa465), (0x141aaabe0, 0x141aab363),
           (0x141aad150, 0x141aad639), (0x141aad640, 0x141aaea87)]
READERS = {0x140c99dc0: 'float*coeff', 0x140c99f40: 'float', 0x140c9a020: 'int', 0x140c9a240: 'string'}


def pe(path):
    d = Path(path).read_bytes()
    e = struct.unpack_from('<I', d, 0x3c)[0]
    nsec = struct.unpack_from('<H', d, e + 6)[0]
    opt = struct.unpack_from('<H', d, e + 20)[0]
    base = struct.unpack_from('<Q', d, e + 24 + 24)[0]
    secs = []
    o = e + 24 + opt
    for _ in range(nsec):
        name = d[o:o + 8].rstrip(b'\0').decode()
        vs, va, rs, ro = struct.unpack_from('<IIII', d, o + 8)
        secs.append((name, base + va, base + va + max(vs, rs), ro, rs))
        o += 40
    return d, secs


def off(secs, v):
    for _, lo, hi, ro, rs in secs:
        if lo <= v < hi and v - lo < rs:
            return ro + v - lo


def key_at(d, secs, v):
    f = off(secs, v)
    if f is None:
        return None
    end = d.find(b'\0', f, f + 100)
    raw = d[f:end] if end > f else b''
    if raw and all(32 < c < 127 for c in raw) and raw[:1] in (b'_', b'/'):
        return raw.decode()


def disasm(path, lo, hi):
    out = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={lo:#x}',
                          f'--stop-address={hi:#x}', str(path)], capture_output=True, text=True).stdout
    for line in out.splitlines():
        m = re.match(r'^\s*([0-9a-f]+):\s+(\w+)\s*(.*)$', line)
        if m:
            yield int(m.group(1), 16), m.group(2), re.sub(r'<[^>]*>', '', m.group(3)).strip()


def schema(path):
    d, secs = pe(path)
    rows = []
    for lo, hi in LOADERS:
        xmm, key, dest, coeff, keyregs = {}, None, None, None, {}
        for va, op, args in disasm(path, lo, hi):
            tgt = re.search(r'#\s*(0x[0-9a-f]+)', args)
            args = args.split('#')[0].strip()
            m = re.match(r'(0x[0-9a-f]+|-?\d+)?\(%rip\), (%\w+)$', args)
            if op == 'leaq' and tgt and m:
                k = key_at(d, secs, int(tgt.group(1), 16))
                if k:
                    keyregs[m.group(2)] = k
                    key, dest, coeff = k, None, None
                    continue
            # a key kept in a register and stored to the string_view slot later
            m = re.match(r'(%\w+), -0x30\(%rbp\)$', args)
            if op == 'movq' and m and m.group(1) in keyregs:
                key, dest, coeff = keyregs[m.group(1)], None, None
                continue
            if key is None:
                if op == 'movss' and tgt and re.search(r'%xmm\d+$', args):
                    f = off(secs, int(tgt.group(1), 16))
                    if f is not None:
                        xmm[args.split('%')[-1]] = struct.unpack_from('<f', d, f)[0]
                continue
            m = re.match(r'(-?0x[0-9a-f]+|-?\d+)\((%\w+)\), %rdx$', args)
            if op == 'leaq' and m and m.group(2) not in ('%rbp', '%rsp'):
                dest = (m.group(2), int(m.group(1), 16))
            m = re.match(r'\$(0x[0-9a-f]+), %rdx$', args)
            if op == 'addq' and m:
                dest = ('ctx', int(m.group(1), 16))
            m = re.match(r'(%r(?:cx|di|si|[0-9]+)), %rdx$', args)
            if op == 'movq' and m:
                dest = (m.group(1), 0)
            if op == 'movss' and tgt and re.search(r'%xmm\d+$', args):
                f = off(secs, int(tgt.group(1), 16))
                if f is not None:
                    xmm[args.split('%')[-1]] = struct.unpack_from('<f', d, f)[0]
            m = re.match(r'%(xmm\d+), %xmm3$', args)
            if op == 'movaps' and m and m.group(1) in xmm:
                coeff = xmm[m.group(1)]
            if op == 'callq':
                t = re.match(r'(0x[0-9a-f]+)', args)
                t = int(t.group(1), 16) if t else None
                if t in READERS and dest:
                    rows.append((key, f'{lo:#x}', dest[0], dest[1], READERS[t], coeff if t == 0x140c99dc0 else None))
                    key = None
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('binary')
    ap.add_argument('--key', default='', help='only keys starting with this prefix')
    args = ap.parse_args()
    for key, fn, base, offset, reader, coeff in schema(args.binary):
        if key.startswith(args.key):
            c = '' if coeff is None else repr(coeff)
            print(f'{key}\t{fn}\t{base}\t{offset:#x}\t{reader}\t{c}')


if __name__ == '__main__':
    main()
