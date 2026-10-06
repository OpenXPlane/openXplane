#!/usr/bin/env python3
"""Extracts the data-output line table of the reference build: for every line index the 8-cell label string.

The line builder (0x1416b4ed0, engine/dout/dout_lines.cpp) is a switch over the line index. Each case builds a
label string (8 cells of 12 characters; '_' stands for a space; the first six characters of a cell are its upper
label row, the last six the lower row) and calls the line constructor 0x1416b3ce0 with up to 8 (kind, value)
pairs. This reads the jump table, follows each case to its label string and prints index<TAB>label.

    python3 tools/extract_dout.py Xplane12/X-Plane.exe > assets/dout/lines.tsv
"""
import re
import struct
import subprocess
import sys

BASE = 0x140000000
FUNC = (0x1416b4ed0, 0x1416c9d54)
TABLE = BASE + 0x16c9aa0   # jump table of 4-byte RVAs relative to the image base
CASES = 0xad


def main():
    exe = sys.argv[1]
    d = open(exe, 'rb').read()
    pe = struct.unpack_from('<I', d, 0x3c)[0]
    sec = pe + 24 + struct.unpack_from('<H', d, pe + 20)[0]
    secs = [struct.unpack_from('<8sIIII', d, sec + 40 * i) for i in range(struct.unpack_from('<H', d, pe + 6)[0])]

    def rva_to_off(rva):
        for _, vs, va, rs, ro in secs:
            if va <= rva < va + rs:
                return ro + rva - va
        raise KeyError(hex(rva))

    def cstring(va):
        o = rva_to_off(va - BASE)
        e = d.index(b'\0', o)
        return d[o:e].decode('latin1')

    asm = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={FUNC[0]:#x}',
                          f'--stop-address={FUNC[1]:#x}', exe], capture_output=True, text=True).stdout
    ins = []
    for line in asm.splitlines():
        m = re.match(r'\s*([0-9a-f]{9}):\s+(.*)', line)
        if m:
            ins.append((int(m.group(1), 16), re.sub(r'<[^>]*>', '', m.group(2))))
    index = {a: i for i, (a, _) in enumerate(ins)}
    base_off = rva_to_off(TABLE - BASE)
    for case in range(CASES):
        target = BASE + struct.unpack_from('<I', d, base_off + 4 * case)[0]
        if target not in index:
            continue
        label = None
        for a, text in ins[index[target]:index[target] + 400]:
            if 'callq' in text and '0x1416b3ce0' in text:
                break
            m = re.search(r'leaq\s+\S+\(%rip\), %rdx\s+# (0x[0-9a-f]+)', text)
            if m:
                try:
                    s = cstring(int(m.group(1), 16))
                except (KeyError, ValueError):
                    continue
                if len(s) >= 11 and all(32 <= ord(c) < 127 for c in s):
                    label = s
        print(f'{case}\t{label if label is not None else ""}')


if __name__ == '__main__':
    main()
