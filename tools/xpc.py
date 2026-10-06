#!/usr/bin/env python3
"""Prints the constants at the given addresses of the reference executable (float32 bits/value and
float64 bits/value): python3 tools/xpc.py Xplane12/X-Plane.exe 0x14250e248 0x14250e330 ..."""
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402

emu = Emulator(sys.argv[1])
for a in sys.argv[2:]:
    addr = int(a, 16)
    raw = bytes(emu.uc.mem_read(addr, 8))
    f32 = struct.unpack('<f', raw[:4])[0]
    f64 = struct.unpack('<d', raw)[0]
    print(f'{addr:#x}: f32 {raw[:4][::-1].hex()} = {f32!r}   f64 {raw[::-1].hex()} = {f64!r}   i32 {struct.unpack("<i", raw[:4])[0]}')
