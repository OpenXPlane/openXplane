#!/usr/bin/env python3
"""Compare embedded chords with the original default-key copy block (local EXE only).

Executes 0x140c647a0..0x140c647b4 for each of the 3138 original records,
with the original table mapped and an explicit preallocated destination. No calls/stubs.
Command-name resolution, subsequent zero-key removal and event dispatch are not replayed.
"""
import argparse
import struct
from pathlib import Path
from emulate_xp import Emulator
from extract_commands import TABLE_VA, detailed_records
from unicorn.x86_const import UC_X86_REG_R13, UC_X86_REG_RDI, UC_X86_REG_RBX


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('binary')
    ap.add_argument('--table', type=Path, default=Path(__file__).resolve().parent.parent / 'assets/keymap/default_keys.tsv')
    args = ap.parse_args()
    records = detailed_records(args.binary)  # validates identity and native bounds
    expected = {}
    for line in args.table.read_text().splitlines():
        name, _key, code, _desc, mask = line.split('\t')
        if name in expected:
            raise ValueError(f'duplicate embedded command {name}')
        expected[name] = (int(code, 16), int(mask, 16))
    emu = Emulator(args.binary)
    owner, destination = emu.alloc(32), emu.alloc(16 * len(records))
    emu.write_u64(owner + 8, destination)
    bound = 0
    for index, (name, _internal, _code, _description, _mask) in enumerate(records):
        emu.uc.reg_write(UC_X86_REG_R13, owner)
        emu.uc.reg_write(UC_X86_REG_RDI, index * 16)
        emu.uc.reg_write(UC_X86_REG_RBX, TABLE_VA + index * 40 + 28)
        emu.run_range(0x140c647a0, 0x140c647b4)
        actual = struct.unpack('<II', emu.read(destination + index * 16, 8))
        if actual[0]:
            if expected.pop(name, None) != actual:
                raise AssertionError(f'embedded chord differs from original copy output for {name}')
            bound += 1
        elif name in expected:
            raise AssertionError(f'embedded binding for original unbound command {name}')
    assert not expected, f'unmatched embedded bindings: {list(expected)}'
    assert not emu.stubs
    assert bound == 182
    print(f'Original key-copy block: {len(records)} records replayed, {bound} embedded chords match exactly; no stubs.')
    print('Scope: key code and modifier copying only; command lookup, conflict dispatch, UI handlers and OS events unverified.')


if __name__ == '__main__':
    main()
