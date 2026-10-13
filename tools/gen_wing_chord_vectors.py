#!/usr/bin/env python3
"""Execute original wing-loader chord conversion loop; no stubs or CRT calls."""
import hashlib
import random
import struct
import sys
from pathlib import Path
from emulate_xp import Emulator
from unicorn.x86_const import UC_X86_REG_RDI, UC_X86_REG_RSI, UC_X86_REG_XMM6
SHA = '2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936'


def main(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != SHA:
        raise ValueError('reference SHA256 mismatch')
    emu = Emulator(path)
    wing = emu.alloc(0x100)
    rng = random.Random(20261013)
    print('# reference_sha256', SHA)
    print('# original_range 0x141aae9ee..0x141aaea17 exclusive; no stubs; XMM6=0x3e9c0ebf')
    print('# count; eleven input words; eleven output words; finite and signed-zero inputs')
    count = 0
    for n in [-2, -1, *range(11)]:
        for variant in range(24):
            inputs = ([0, 0x80000000, 0x3f800000, 0xbf800000, 1, 0x80000001, 0x7f7fffff, 0xff7fffff, 0x00800000, 0x80800000, 0x3e9c0ebf]
                      if variant == 0 else [struct.unpack('<I', struct.pack('<f', rng.uniform(-1000, 1000)))[0] for _ in range(11)])
            emu.write(wing, bytes(0x100))
            emu.write(wing + 4, struct.pack('<i', n))
            emu.write(wing + 0x70, struct.pack('<11I', *inputs))
            for reg, value in [(UC_X86_REG_RDI, wing), (UC_X86_REG_RSI, wing + 0x70), (UC_X86_REG_XMM6, 0x3e9c0ebf)]:
                emu.uc.reg_write(reg, value)
            emu.run_range(0x141aae9ee, 0x141aaea17)
            outputs = struct.unpack('<11I', emu.read(wing + 0x70, 44))
            assert not emu.stubs
            print(n, *[f'{v:08x}' for v in inputs], *[f'{v:08x}' for v in outputs])
            count += 1
    print('# cases', count)


if __name__ == '__main__':
    main(sys.argv[1])
