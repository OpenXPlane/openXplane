#!/usr/bin/env python3
"""Disassembles one function of the reference build, with or without exception-table information.

The end is the table's end when the function has an entry; otherwise it is the first `ret` that no earlier
jump in the function skips past.

    python3 tools/dump_func.py Xplane12/X-Plane.exe 0x1411a2bf0
"""
import bisect
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from pe_functions import load  # noqa: E402


def main():
    exe, start = sys.argv[1], int(sys.argv[2], 16)
    table = load(exe)
    i = bisect.bisect_right([b for b, e in table], start - 0x140000000) - 1
    end = None
    if i >= 0 and table[i][0] == start - 0x140000000:
        end = 0x140000000 + table[i][1]
    stop = end or start + 0x600
    asm = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={start:#x}',
                          f'--stop-address={stop:#x}', exe], capture_output=True, text=True).stdout
    farthest = start
    for line in asm.splitlines():
        m = re.match(r'\s*([0-9a-f]{9}):\s+(.*)', line)
        if not m:
            continue
        addr, text = int(m.group(1), 16), re.sub(r'\s+', ' ', re.sub(r'<[^>]*>', '', m.group(2))).strip()
        if end is None and text.startswith('int3'):
            break
        print(f'{addr:x}: {text}')
        j = re.match(r'j\w+ (0x[0-9a-f]+)', text)
        if j and int(j.group(1), 16) > farthest:
            farthest = int(j.group(1), 16)
        if end is None and text.startswith('retq') and addr >= farthest:
            break


if __name__ == '__main__':
    main()
