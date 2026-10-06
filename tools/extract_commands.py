#!/usr/bin/env python3
"""Enumerate the command table of the reference X-Plane.exe without running it.

Record layout (40 bytes, 5 qwords), observed at sim/flight_controls/flaps_up and neighbours:

  [0] VA of the internal name, always "cmnd_..."   [1] VA of the command name ("sim/...")
  [2] VA of the description                        [3] low dword: 3 in every observed record;
                                                       high dword: nonzero in some records
  [4] zero

The meaning of qword 3 is not established; it is printed as-is. Candidates are byte patterns:
all three pointers must reach printable strings in .rdata and the command name must contain '/'.
Output is a TSV: command name, internal name, qword3, description.
"""
import argparse
import struct
from pathlib import Path


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


def text_at(d, secs, v, limit=300):
    lo, hi, ro, rs = secs['.rdata']
    if not lo <= v < hi or v - lo >= rs:
        return None
    f = ro + v - lo
    end = d.find(b'\0', f, f + limit)
    if end <= f:
        return None
    raw = d[f:end]
    if all(32 <= c < 127 for c in raw):
        return raw.decode()
    return None


def records(path):
    d, secs = load(path)
    _, _, ro, rs = secs['.data']
    out = []
    for f in range(ro, ro + rs - 40 + 1, 8):
        q = struct.unpack_from('<5Q', d, f)
        if q[4] != 0 or (q[3] & 0xffffffff) != 3:
            continue
        internal = text_at(d, secs, q[0])
        if not internal or not internal.startswith('cmnd_'):
            continue
        name = text_at(d, secs, q[1])
        if not name or '/' not in name or ' ' in name:
            continue
        desc = text_at(d, secs, q[2])
        if desc is None:
            continue
        out.append((name, internal, q[3] >> 32, desc))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('binary')
    ap.add_argument('--prefix', default='')
    args = ap.parse_args()
    for name, internal, extra, desc in records(args.binary):
        if name.startswith(args.prefix):
            print(f'{name}\t{internal}\t{extra:#x}\t{desc}')


if __name__ == '__main__':
    main()
