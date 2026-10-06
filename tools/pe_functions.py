#!/usr/bin/env python3
"""Looks up the function (from the .pdata exception table) containing each address.

    python3 tools/pe_functions.py Xplane12/X-Plane.exe 0x1416b5252 0x1416b3ce0
"""
import bisect
import struct
import sys


def load(path):
    d = open(path, 'rb').read()
    pe = struct.unpack_from('<I', d, 0x3c)[0]
    n = struct.unpack_from('<H', d, pe + 6)[0]
    opt = struct.unpack_from('<H', d, pe + 20)[0]
    sec = pe + 24 + opt
    for i in range(n):
        name, vs, va, rs, ro = struct.unpack_from('<8sIIII', d, sec + 40 * i)
        if name.startswith(b'.pdata'):
            raw = d[ro:ro + rs]
            return [struct.unpack_from('<II', raw, o) for o in range(0, len(raw) - 11, 12) if raw[o:o + 4] != b'\0\0\0\0']
    raise SystemExit('no .pdata')


def main():
    table = load(sys.argv[1])
    begins = [b for b, e in table]
    for arg in sys.argv[2:]:
        rva = int(arg, 16) - 0x140000000
        i = bisect.bisect_right(begins, rva) - 1
        b, e = table[i]
        print(f'{arg}: function {0x140000000 + b:#x}..{0x140000000 + e:#x} ({e - b:#x} bytes)' if b <= rva < e else f'{arg}: no function')


if __name__ == '__main__':
    main()
