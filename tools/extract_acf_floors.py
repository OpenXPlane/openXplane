#!/usr/bin/env python3
"""Finds the "value below a limit becomes a default" tails of the ACF loaders.

    python3 tools/extract_acf_floors.py Xplane12/X-Plane.exe > crates/xp-acf/data/object_floors.tsv

After a reader call that stores a float at `OFF(%reg)` the loaders sometimes compare the stored value with a
constant and overwrite it with a default, in one of two shapes:

    comiss OFF(%reg), %xmmL ; jbe skip ; movl $DEFAULT, OFF(%reg)                 (single precision limit)
    movss OFF(%reg), %xmmA ; cvtps2pd %xmmA, %xmmA ; comisd %xmmA, %xmmL ;
        jbe skip ; movl $DEFAULT, OFF(%reg)                                         (double precision limit)

The limit register is resolved from the last `movss`/`movsd CONST(%rip), %xmmL` or `xorps` before the
comparison. The whole loader area (0x141a90000 .. 0x141ab0000) is scanned because the property readers of the
bodies and wings are separate functions. Output columns: function-area start, offset, base register, width
(`f` or `d`), limit bits, default word. The stored value is replaced when `limit > value`; an unordered
comparison keeps the value.
"""
import re
import struct
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from extract_acf_schema import off, pe  # noqa: E402

LOW, HIGH = 0x141a90000, 0x141ab0000


def main():
    exe = sys.argv[1]
    d, secs = pe(exe)
    raw = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={LOW:#x}',
                          f'--stop-address={HIGH:#x}', exe], capture_output=True, text=True).stdout
    insns = []
    for line in raw.splitlines():
        m = re.match(r'\s*([0-9a-f]+):\s*(.*)', line)
        if m:
            body = re.sub(r'<[^>]*>', '', m.group(2))
            tgt = re.search(r'# (0x[0-9a-f]+)', body)
            text = re.sub(r'\s+', ' ', re.sub(r'\s*#.*', '', body)).strip()
            insns.append((int(m.group(1), 16), text, int(tgt.group(1), 16) if tgt else None))
    xmm = {}
    seen = set()
    for i, (addr, t, tgt) in enumerate(insns):
        m = re.match(r'xorps (%xmm\d+), (%xmm\d+)$', t)
        if m and m.group(1) == m.group(2):
            xmm[m.group(1)] = ('f', 0)
        m = re.match(r'movs([sd]) 0x[0-9a-f]+\(%rip\), (%xmm\d+)$', t)
        if m and tgt:
            f = off(secs, tgt)
            if f is not None:
                width = 'f' if m.group(1) == 's' else 'd'
                xmm[m.group(2)] = (width, struct.unpack_from('<I' if width == 'f' else '<Q', d, f)[0])
        m = re.match(r'movaps (%xmm\d+), (%xmm\d+)$', t)
        if m and m.group(1) in xmm:
            xmm[m.group(2)] = xmm[m.group(1)]
        window = [x[1] for x in insns[i:i + 8]]
        # single precision: comiss OFF(%reg), %xmmL ; jbe ; movl $IMM, OFF(%reg)
        m = re.match(r'comiss (-?0x[0-9a-f]+|)\((%\w+)\), (%xmm\d+)$', t)
        if m and i + 2 < len(insns):
            s = re.match(r'movl \$(0x[0-9a-f]+), (-?0x[0-9a-f]+|)\((%\w+)\)$', insns[i + 2][1])
            if insns[i + 1][1].startswith('jbe') and s and s.group(2) == m.group(1) and s.group(3) == m.group(2):
                limit = xmm.get(m.group(3))
                if limit and limit[0] == 'f':
                    key = (int(m.group(1) or '0', 16), m.group(2), 'f', limit[1], int(s.group(1), 16))
                    if key not in seen:
                        seen.add(key)
                        print(f'{LOW:#x}\t{key[0]:#x}\t{key[1]}\tf\t{key[3]:08x}\t{key[4]:08x}')
        # double precision: movss OFF(%reg), %xmmA ; cvtps2pd %xmmA, %xmmA ; comisd %xmmA, %xmmL ; jbe ; movl
        m = re.match(r'movss (-?0x[0-9a-f]+|)\((%\w+)\), (%xmm\d+)$', t)
        if m:
            a = m.group(3)
            for k in range(1, 5):
                if i + k + 2 >= len(insns):
                    break
                c = re.match(r'comisd (%xmm\d+), (%xmm\d+)$', insns[i + k][1])
                if c and c.group(1) == a:
                    s = re.match(r'movl \$(0x[0-9a-f]+), (-?0x[0-9a-f]+|)\((%\w+)\)$', insns[i + k + 2][1])
                    if insns[i + k + 1][1].startswith('jbe') and s and s.group(2) == m.group(1) and \
                            s.group(3) == m.group(2):
                        limit = xmm.get(c.group(2))
                        if limit and limit[0] == 'd':
                            key = (int(m.group(1) or '0', 16), m.group(2), 'd', limit[1], int(s.group(1), 16))
                            if key not in seen:
                                seen.add(key)
                                print(f'{LOW:#x}\t{key[0]:#x}\t{key[1]}\td\t{key[3]:016x}\t{key[4]:08x}')
                    break


if __name__ == '__main__':
    main()
