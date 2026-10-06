#!/usr/bin/env python3
"""Lists the finite-value guards (fpclassify 0x14230c290) of a function: the checked operand and the repair.
    python3 tools/list_guards.py exe start end"""
import re
import subprocess
import sys

exe, lo, hi = sys.argv[1], sys.argv[2], sys.argv[3]
out = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={lo}', f'--stop-address={hi}', exe],
                     capture_output=True, text=True).stdout
ins = []
for line in out.splitlines():
    m = re.match(r'\s*([0-9a-f]{9}):\s+(.*)', line)
    if m:
        ins.append((m.group(1), re.sub(r'\s+', ' ', re.sub(r'<[^>]*>', '', m.group(2))).strip()))
for i, (a, t) in enumerate(ins):
    if t.startswith('callq') and t.endswith('0x14230c290'):
        operand = ins[i - 1][1]
        target = None
        for j in range(i + 1, i + 10):
            if ins[j][1].startswith('ja '):
                target = ins[j][1].split()[-1]
                repair = ' | '.join(x[1] for x in ins[j + 1:j + 4])
                break
        print(a, 'operand:', operand, ' continue at', target, ' after ja:', repair)
