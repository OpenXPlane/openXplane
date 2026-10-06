#!/usr/bin/env python3
"""Generates test vectors for engine::engine_update by running the ORIGINAL engine update 0x141197b00.

The objects (flight object F, aircraft object B at F+0x20, the engine record, the engine descriptor) are
filled with random values: each dword is a float in 0.2..2 or a small integer, so both float and integer
fields take plausible values. Callees that depend on engine state are replaced by recording stubs (see
engine_harness.py): the atmosphere accessors, the engine flag, the frame time and the input-binding query.
The words the function reads from the objects are found with a memory-read hook; the file lists them.

Line: idx | calls | F off=hex ... | B ... | D ... | E (record before, all words) | E off=hex (record after, changed words)
calls: n { name arg... result } as hex dwords

    python3 tools/gen_engine_update_vectors.py Xplane12/X-Plane.exe > crates/xp-app/tests/data/engine_update.txt
"""
import random
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from emulate_xp import Emulator  # noqa: E402
from unicorn import UC_HOOK_MEM_READ  # noqa: E402
from unicorn.x86_const import UC_X86_REG_R8, UC_X86_REG_R9, UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX, UC_X86_REG_XMM0  # noqa: E402

FUNC = 0x141197b00


def bits_f(x):
    return struct.unpack('<I', struct.pack('<f', x))[0]


def main():
    emu = Emulator(sys.argv[1])
    rng = random.Random(20261016)
    F, B, eng = emu.alloc(0x10000), emu.alloc(0x8000), emu.alloc(0x2cc * 2)
    desc, wings = emu.alloc(0x68 * 4), emu.alloc(0x3770 * 2)
    regions = {'F': (F, 0x10000), 'B': (B, 0x8000), 'D': (desc, 0x68 * 4), 'E': (eng, 0x2cc * 2)}
    touched = {k: set() for k in regions}

    def on_read(uc, access, address, size, value, user):
        for k, (base, length) in regions.items():
            if base <= address < base + length:
                for a in range(address - address % 4, address + size, 4):
                    touched[k].add(a - base)

    for base, length in regions.values():
        emu.uc.hook_add(UC_HOOK_MEM_READ, on_read, None, base, base + length - 1)

    calls = []
    state = {}

    def record(name, ret_kind):
        def stub(e):
            regs = [e.reg(UC_X86_REG_RCX), e.reg(UC_X86_REG_RDX), e.reg(UC_X86_REG_R8), e.reg(UC_X86_REG_R9)]
            xmm = [e.uc.reg_read(UC_X86_REG_XMM0 + i) & 0xffffffff for i in range(4)]
            if ret_kind == 'f':
                value = rng.uniform(0.5, 1.5)
                e.set_xmm_f32(0, value)
                calls.append((name, xmm[1:2] if name == 'atmo_a' else xmm[1:3], bits_f(value)))
            elif ret_kind == 'i':
                value = rng.randrange(2)
                e.uc.reg_write(UC_X86_REG_RAX, value)
                rid, index = regs[2] & 0xffffffff, regs[3] & 0xffffffff
                calls.append((name, [rid, index], value))
            elif ret_kind == 'b':
                value = rng.randrange(2)
                e.uc.reg_write(UC_X86_REG_RAX, value)
                calls.append((name, [], value))
            else:
                value = rng.uniform(0.002, 0.05)
                e.uc.reg_write(UC_X86_REG_XMM0, struct.unpack('<Q', struct.pack('<d', value))[0])
                calls.append((name, [], struct.unpack('<Q', struct.pack('<d', value))[0]))
        return stub

    emu.stubs[0x141ba6750] = record('atmo_a', 'f')
    emu.stubs[0x141ba64e0] = record('atmo_b', 'f')
    emu.stubs[0x1417f12c0] = record('flag', 'b')
    emu.stubs[0x140c448c0] = record('dt', 'd')
    emu.stubs[0x1407ace10] = record('bind', 'i')
    print('# engine update vectors from the original 0x141197b00 (see tools/gen_engine_update_vectors.py)')

    def randomise(addr, size):
        words = []
        for _ in range(size // 4):
            if rng.random() < 0.7:
                words.append(bits_f(rng.uniform(0.2, 2.0)))
            else:
                words.append(rng.choice([0, 1, 1, 2]))
        emu.write(addr, b''.join(struct.pack('<I', w) for w in words))

    produced = attempts = 0
    while produced < 120 and attempts < 3000:
        attempts += 1
        for base, length in regions.values():
            randomise(base, length)
        emu.write_u64(F + 0x20, B)
        emu.write_u64(F + 0x68b0, eng)
        emu.write_u64(B + 0x5ff8, desc)
        emu.write_u64(B + 0x6010, wings)
        emu.write_u32(B + 0x91c, 1)
        emu.write_u32(B + 0x920, 1)
        for obj in (F, B):
            emu.write_u32(obj + 0xbcc8, 0)
            emu.write_u32(obj + 0xbcd0, 0)
        for k in touched:
            touched[k].clear()
        calls.clear()
        snapshot = {k: emu.read(base, length) for k, (base, length) in regions.items()}
        try:
            emu.call(FUNC, ints=[eng, F, 0, 0])
        except Exception:
            continue
        after = emu.read(eng, 0x2cc)
        before = snapshot['E'][:0x2cc]
        changed = [(o, struct.unpack_from('<I', after, o)[0]) for o in range(0, 0x2cc, 4)
                   if before[o:o + 4] != after[o:o + 4]]
        tokens = ['0', '|', str(len(calls))]
        for name, args, ret in calls:
            tokens += [name, str(len(args)), *[f'{a:08x}' for a in args], f'{ret:x}']
        for k in ('F', 'B', 'D'):
            tokens += ['|']
            for off in sorted(touched[k]):
                tokens.append(f'{off:x}={struct.unpack_from("<I", snapshot[k], off)[0]:08x}')
        tokens += ['|', *[f'{struct.unpack_from("<I", before, o)[0]:08x}' for o in range(0, 0x2cc, 4)], '|',
                   *[f'{o:x}={v:08x}' for o, v in changed]]
        print(*tokens)
        produced += 1
    print(f'# {produced} cases from {attempts} attempts', file=sys.stderr)


if __name__ == '__main__':
    main()
