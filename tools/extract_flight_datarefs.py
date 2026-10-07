#!/usr/bin/env python3
"""Maps the simple datarefs of the reference build to fields of the flight object `F`.

    python3 tools/extract_datarefs.py Xplane12/X-Plane.exe > /tmp/datarefs.tsv
    python3 tools/extract_flight_datarefs.py Xplane12/X-Plane.exe /tmp/datarefs.tsv > research/FLIGHT_DATAREFS.tsv

A getter is recognised when it is a thunk of one of the shapes

  call 0x141a64250 (the flight object of the aircraft index)  ->  load OFFSET(%rax)  [scale]  ret
  mov G(%rip),%rax (a global object)                          ->  load OFFSET(%rax)  [scale]  ret

with the load `movss`/`movsd`/`movl`/`movq` (float, double, int) and an optional `mulss`/`mulsd` by a constant
(the unit conversion). The output is a TSV: name, object (`F` or the global's address), offset, load kind, scale.
"""
import re
import struct
import subprocess
import sys
from pathlib import Path

exe, tsv = sys.argv[1], sys.argv[2]
data = Path(exe).read_bytes()
pe = struct.unpack_from('<I', data, 0x3c)[0]
n = struct.unpack_from('<H', data, pe + 6)[0]
osz = struct.unpack_from('<H', data, pe + 20)[0]
base = struct.unpack_from('<Q', data, pe + 24 + 24)[0]
secs = []
for i in range(n):
    o = pe + 24 + osz + i * 40
    vs, rva, rs, ro = struct.unpack_from('<IIII', data, o + 8)
    secs.append((rva, vs, ro))


def read(va, k):
    r = va - base
    for rva, vs, ro in secs:
        if rva <= r < rva + vs:
            return data[ro + r - rva:ro + r - rva + k]
    return None


def getter(va):
    out = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={va:#x}',
                          f'--stop-address={va + 0x60:#x}', exe], capture_output=True, text=True).stdout
    lines = []
    for line in out.splitlines():
        m = re.match(r'\s*([0-9a-f]+):\s+(\S+)\s*(.*)', line)
        if m:
            lines.append((m.group(2), re.sub(r'<[^>]*>', '', m.group(3)).strip()))
            if m.group(2).startswith('ret'):
                break
    return lines


object_kind = None
for line in Path(tsv).read_text().splitlines():
    name, kinds, rw, rest = line.split('\t')
    m = re.search(r'get_(float|int|double)=(0x[0-9a-f]+)', rest)
    if not m:
        continue
    lines = getter(int(m.group(2), 16))
    ops = [l for l in lines if not l[0].startswith(('sub', 'add', 'push', 'pop', 'nop', 'int3'))]
    shape = None
    src = None
    idx = 0
    if ops and ops[0][0] == 'movl' and 'rip' in ops[0][1]:
        # index load, then the lookup call
        if len(ops) > 2 and ops[1][0] == 'leaq' and ops[2][0].startswith('call') and '0x141a64250' in ops[2][1]:
            src = 'F'
            idx = 3
    elif ops and ops[0][0] == 'movq' and 'rip' in ops[0][1] and '%rax' in ops[0][1]:
        g = re.search(r'# (0x[0-9a-f]+)', ops[0][1])
        src = g.group(1) if g else None
        idx = 1
    if src is None or idx >= len(ops):
        continue
    load = ops[idx]
    lm = re.match(r'(0x[0-9a-f]+)\(%rax\),\s*%(\w+)', load[1])
    if load[0] not in ('movss', 'movsd', 'movl', 'movq') or not lm:
        continue
    offset = int(lm.group(1), 16)
    scale = ''
    rest_ops = ops[idx + 1:]
    for op, args in rest_ops:
        sm = re.search(r'# (0x[0-9a-f]+)', args)
        if op in ('mulss', 'mulsd', 'divss', 'divsd') and sm:
            val = read(int(sm.group(1), 16), 8 if op.endswith('sd') else 4)
            v = struct.unpack('<d' if op.endswith('sd') else '<f', val)[0]
            scale += f'{"*" if op.startswith("mul") else "/"}{v!r}'
        elif op.startswith('ret'):
            break
        elif op in ('cvtss2sd', 'cvtps2pd', 'cvtsd2ss', 'cvtpd2ps', 'movaps', 'cvtdq2ps', 'cvtsi2ss'):
            continue
        else:
            scale = None
            break
    if scale is None:
        continue
    print('\t'.join([name, src, hex(offset), load[0], scale, 'rw' if 'set_' in rest else 'ro']))
