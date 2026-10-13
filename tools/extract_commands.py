#!/usr/bin/env python3
"""Enumerate the native command table of the reference X-Plane.exe without running it.

The exact baseline has 3138 consecutive 40-byte records at 0x142ee1400, confirmed by
its native default-key loader. Qwords 0..2 hold internal/name/description pointers;
qword 3 holds a low flag and high default key; qword 4 holds modifier bits
Shift=1, Alt=2, Ctrl=4. Three reserved VR records have low flag 2 rather than 3.
No cmnd_ prefix or zero-modifier filter is used. See research/KEYMAP.md.
Default output stays name, internal, key, description; --modifiers appends a mask.
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


REFERENCE_SHA256 = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'
TABLE_VA, TABLE_COUNT = 0x142ee1400, 3138


def detailed_records(path):
    """Native table, in original order: (name, internal, key, description, modifiers).

    Bounds come from 0x140c6478d..0x140c64827, not an ASCII-pattern guess.
    Fail on another build rather than silently extracting an incomplete catalogue.
    """
    import hashlib
    d, secs = load(path)
    if hashlib.sha256(d).hexdigest() != REFERENCE_SHA256:
        raise ValueError('reference SHA256 mismatch; native table bounds are build-specific')
    lo, _, ro, rs = secs['.data']
    start = ro + TABLE_VA - lo
    if start < ro or start + TABLE_COUNT * 40 > ro + rs:
        raise ValueError('command table is outside file-backed .data')
    out = []
    for index in range(TABLE_COUNT):
        q = struct.unpack_from('<5Q', d, start + index * 40)
        internal, name, desc = (text_at(d, secs, v) for v in q[:3])
        if not internal or not name or '/' not in name or ' ' in name or desc is None:
            raise ValueError(f'invalid native command record {index}')
        if (q[3] & 0xffffffff) not in (2, 3) or q[4] > 7:
            raise ValueError(f'unreviewed flags in native command record {index}')
        out.append((name, internal, q[3] >> 32, desc, q[4]))
    return out


def records(path):
    """Four-column catalogue API retained for existing consumers."""
    return [row[:4] for row in detailed_records(path)]


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('binary')
    ap.add_argument('--prefix', default='')
    ap.add_argument('--modifiers', action='store_true', help='append the modifier mask as column five')
    args = ap.parse_args()
    for name, internal, extra, desc, modifiers in detailed_records(args.binary):
        if name.startswith(args.prefix):
            row = f'{name}\t{internal}\t{extra:#x}\t{desc}'
            print(row + (f'\t{modifiers:#x}' if args.modifiers else ''))


if __name__ == '__main__':
    main()
