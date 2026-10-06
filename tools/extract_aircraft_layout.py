#!/usr/bin/env python3
"""Maps byte offsets of the aircraft object (the one behind the singleton getter 0x141a64260) to the
`sim/aircraft/...` datarefs whose scalar getters read them, from the getters' machine code.

    python3 tools/extract_aircraft_layout.py Xplane12/X-Plane.exe research/local/datarefs.tsv > aircraft_layout.tsv
"""
import re
import subprocess
import sys


def main():
    exe, table = sys.argv[1], sys.argv[2]
    for line in open(table):
        name, typ, rw, acc = line.rstrip('\n').split('\t')
        if not name.startswith('sim/aircraft/') or typ not in ('float', 'int', 'double'):
            continue
        m = re.search(r'get_(?:float|int|double)=(0x[0-9a-f]+)', acc)
        if not m:
            continue
        a = int(m.group(1), 16)
        asm = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={a:#x}',
                              f'--stop-address={a + 0x40:#x}', exe], capture_output=True, text=True).stdout
        text = re.sub(r'<[^>]*>', '', asm)
        if '0x141a64260' not in text:
            continue
        after = text.split('0x141a64260', 1)[1]
        mm = re.search(r'(?:movss|movl|movsd|cvtsi2ss|mov\w*)\s+(0x[0-9a-f]+)\(%rax\)', after)
        if mm:
            print(f'{int(mm.group(1), 16):#06x}\t{typ}\t{name}')


if __name__ == '__main__':
    main()
