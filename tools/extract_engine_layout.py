#!/usr/bin/env python3
"""Lists the byte offsets inside the engine record (stride 0x2cc, array pointer at +0x68b0 of the aircraft
object) of every `sim/.../engine/...` dataref whose array getter reads one, from the getters' machine code.

    python3 tools/extract_engine_layout.py Xplane12/X-Plane.exe research/local/datarefs.tsv
(the dataref table comes from tools/extract_datarefs.py)
"""
import re
import subprocess
import sys


def main():
    exe, table = sys.argv[1], sys.argv[2]
    rows = []
    for line in open(table):
        name, typ, rw, acc = line.rstrip('\n').split('\t')
        if '/engine/' not in name:
            continue
        m = re.search(r'get_(?:float|int)_array=(0x[0-9a-f]+)', acc)
        if not m:
            continue
        a = int(m.group(1), 16)
        asm = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={a:#x}',
                              f'--stop-address={a + 0x90:#x}', exe], capture_output=True, text=True).stdout
        if 'imulq\t$0x2cc' not in asm:
            continue
        mm = re.search(r'movl\s+(-?0x[0-9a-f]+)\(%rcx,%rsi\)', asm) or re.search(r'movl\s+(-?0x[0-9a-f]+)\(%rsi,%rcx\)', asm)
        if mm:
            rows.append((int(mm.group(1), 16) + 0x2cc, typ, name))
    for off, typ, name in sorted(rows):
        print(f'0x{off:03x}\t{typ}\t{name}')


if __name__ == '__main__':
    main()
