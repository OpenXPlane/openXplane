#!/usr/bin/env python3
"""Prints a function of the reference build with its diagnostic code removed, so the arithmetic can be read.

Removed: clusters around the C++ stream and string calls used for logging, finite-value guards (`fpclassify`
call followed by a jump over a repair block), and the named debug-check calls.

    python3 tools/clean_asm.py Xplane12/X-Plane.exe 0x1411bd470 0x1411c3584 > clean.asm
"""
import re
import subprocess
import sys

LOG_CALLS = ('0x1405e18d0', '0x1422c675c', '0x1422c6748', '0x1408943e0', '0x140a31900', '0x1405dcad0')
CHECK_CALLS = ('0x1411764a0', '0x1408e25a0', '0x141a67610')
FPCLASSIFY = '0x14230c290'


def main():
    exe, start, stop = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3], 16)
    asm = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={start:#x}',
                          f'--stop-address={stop:#x}', exe], capture_output=True, text=True).stdout
    ins = []
    for line in asm.splitlines():
        m = re.match(r'\s*([0-9a-f]{9}):\s+(.*)', line)
        if m:
            ins.append((int(m.group(1), 16), re.sub(r'\s+', ' ', re.sub(r'<[^>]*>', '', m.group(2))).strip()))
    index = {a: i for i, (a, _) in enumerate(ins)}
    skip = set()
    for i, (a, t) in enumerate(ins):
        if t.startswith('callq') and t.endswith(FPCLASSIFY):
            for j in range(i + 1, i + 9):
                if ins[j][1].startswith('ja ') and int(ins[j][1].split()[-1], 16) in index:
                    skip.update(range(i, index[int(ins[j][1].split()[-1], 16)]))
                    break
        if t.startswith('callq') and t.endswith(CHECK_CALLS):
            k = i
            while k > 0 and k > i - 14 and not ins[k - 1][1].startswith('callq'):
                k -= 1
            skip.update(range(k, i + 1))
    logs = [i for i, (a, t) in enumerate(ins) if t.startswith('callq') and (t.endswith(LOG_CALLS) or re.search(r'callq \*', t))]
    clusters, cur = [], [logs[0]] if logs else []
    for i in logs[1:]:
        if i - cur[-1] <= 14:
            cur.append(i)
        else:
            clusters.append(cur)
            cur = [i]
    if cur:
        clusters.append(cur)
    for c in clusters:
        s = c[0]
        while s > 0 and c[0] - s < 6 and not ins[s - 1][1].startswith(('j', 'callq', 'cmp')):
            s -= 1
        skip.update(range(s, c[-1] + 1))
    dbg = re.compile(r'^cmpl \$0x0, 0xbc[cd][08]\(')
    for i, (a, t) in enumerate(ins):
        if i in skip:
            continue
        if re.search(r'^leaq .*# 0x142f05e60', t):
            continue
        if dbg.search(t):
            skip_jump = True
            continue
        if re.match(r'^j(ne|e) 0x[0-9a-f]+$', t) and i > 0 and dbg.search(ins[i - 1][1]):
            continue
        print(f'{a:x}: {t}')

if __name__ == '__main__':
    main()
