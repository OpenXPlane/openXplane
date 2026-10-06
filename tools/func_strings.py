#!/usr/bin/env python3
"""Lists the printable strings a function refers to through rip-relative leaq: tools/func_strings.py exe start end"""
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

exe, lo, hi = sys.argv[1], int(sys.argv[2], 16), int(sys.argv[3], 16)
out = subprocess.run(['llvm-objdump', '-d', '--no-show-raw-insn', f'--start-address={lo:#x}', f'--stop-address={hi:#x}', exe],
                     capture_output=True, text=True).stdout
emu = Emulator(exe)
seen = {}
for line in out.splitlines():
    m = re.search(r'leaq\s.*# (0x[0-9a-f]+)', line)
    if not m:
        continue
    a = int(m.group(1), 16)
    try:
        raw = bytes(emu.uc.mem_read(a, 64))
    except Exception:
        continue
    s = raw.split(b'\0')[0]
    if len(s) >= 4 and all(32 <= c < 127 for c in s) and a not in seen:
        seen[a] = s.decode()
for a, s in seen.items():
    print(hex(a), s)
