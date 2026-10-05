#!/usr/bin/env python3
"""Enumerate the dataref registration table of the reference X-Plane.exe without running it.

Record layout (128 bytes, 16 qwords) observed at the records of
sim/aircraft/weight/acf_m_empty, sim/aircraft/engine/acf_num_engines and
sim/time/total_running_time_sec, and matching the argument order of XPLMRegisterDataAccessor:

  [0] VA of the NUL-terminated name      [1] low dword: type bits (1 int, 2 float, 4 double,
                                             8 float array, 16 int array, 32 data)
                                             high dword: 1 when writable
  [2] get int      [3] set int      [4] get float    [5] set float
  [6] get double   [7] set double   [8] get int[]    [9] set int[]
  [10] get float[] [11] set float[] [12] get data    [13] set data
  [14], [15] refcons / unused here

Candidates are byte patterns, not decoded runtime structures: a record is accepted only when the
name points into .rdata at a printable string containing '/', the type word is valid and every
accessor slot is zero or points into .text. Output is a TSV: name, type, writable, accessor VAs.
"""
import argparse
import struct
from pathlib import Path

TYPES = {1: 'int', 2: 'float', 4: 'double', 8: 'float[]', 16: 'int[]', 32: 'data'}
SLOTS = ['get_int', 'set_int', 'get_float', 'set_float', 'get_double', 'set_double',
         'get_int_array', 'set_int_array', 'get_float_array', 'set_float_array',
         'get_data', 'set_data']


def load(path):
    d = Path(path).read_bytes()
    e = struct.unpack_from('<I', d, 0x3c)[0]
    nsec = struct.unpack_from('<H', d, e + 6)[0]
    opt = struct.unpack_from('<H', d, e + 20)[0]
    base = struct.unpack_from('<Q', d, e + 24 + 24)[0]
    secs = {}
    o = e + 24 + opt
    for _ in range(nsec):
        name = d[o:o + 8].rstrip(b'\0').decode()
        vs, va, rs, ro = struct.unpack_from('<IIII', d, o + 8)
        secs[name] = (base + va, base + va + max(vs, rs), ro, rs)
        o += 40
    return d, secs


def in_sec(secs, name, v):
    lo, hi, _, _ = secs[name]
    return lo <= v < hi


def file_off(secs, name, v):
    lo, _, ro, rs = secs[name]
    off = v - lo
    return ro + off if off < rs else None


def cstring(d, secs, v):
    f = file_off(secs, '.rdata', v)
    if f is None:
        return None
    end = d.find(b'\0', f, f + 200)
    if end <= f:
        return None
    s = d[f:end]
    if all(32 < c < 127 for c in s) and b'/' in s:
        return s.decode()
    return None


def records(path):
    d, secs = load(path)
    _, _, ro, rs = secs['.data']
    out = []
    for f in range(ro, ro + rs - 128 + 1, 8):
        q = struct.unpack_from('<16Q', d, f)
        typ, writable = q[1] & 0xffffffff, q[1] >> 32
        if typ not in TYPES and not (0 < typ < 64) or writable > 1 or typ == 0:
            continue
        if not in_sec(secs, '.rdata', q[0]):
            continue
        if any(q[i] and not in_sec(secs, '.text', q[i]) for i in range(2, 14)):
            continue
        name = cstring(d, secs, q[0])
        if name is None:
            continue
        out.append((name, typ, writable, q[2:14]))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('binary')
    ap.add_argument('--prefix', default='')
    args = ap.parse_args()
    for name, typ, writable, acc in records(args.binary):
        if not name.startswith(args.prefix):
            continue
        label = '|'.join(t for bit, t in TYPES.items() if typ & bit) or hex(typ)
        slots = ','.join(f'{SLOTS[i]}={acc[i]:#x}' for i in range(12) if acc[i])
        print(f'{name}\t{label}\t{"rw" if writable else "ro"}\t{slots}')


if __name__ == '__main__':
    main()
