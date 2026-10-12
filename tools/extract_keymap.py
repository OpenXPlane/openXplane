#!/usr/bin/env python3
"""Writes the default keyboard map of the reference X-Plane.exe as a TSV (command, key, code, description).

The built-in command table (tools/extract_commands.py, research/COMMANDS.md) stores, in the high dword
of each record's flag word, the default key of the command (0 = none). The codes are the XPLM virtual
key codes: they follow Windows virtual keys (F1-F24 0x70-0x87, digits 0x30-0x39, letters as upper case
ASCII, arrows 0x25-0x28, Esc 0x1B, Enter 0x0D, space 0x20) plus a block of punctuation keys at 0xB0-0xBD.
The default modifiers, if any, are not stored in this record and are not reproduced.

    python3 tools/extract_keymap.py Xplane12/X-Plane.exe > assets/keymap/default_keys.tsv
"""
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from extract_commands import records  # noqa: E402

PUNCTUATION = {0xB0: '=', 0xB1: '-', 0xB2: ']', 0xB3: '[', 0xB4: "'", 0xB5: ';', 0xB6: '\\',
               0xB7: ',', 0xB8: '/', 0xB9: '.', 0xBA: '`', 0xBB: 'Enter', 0xBC: 'NumpadEnter', 0xBD: 'Numpad='}
NAMED = {0x08: 'Backspace', 0x09: 'Tab', 0x0C: 'Clear', 0x0D: 'Return', 0x1B: 'Escape', 0x20: 'Space',
         0x21: 'PageUp', 0x22: 'PageDown', 0x23: 'End', 0x24: 'Home', 0x25: 'Left', 0x26: 'Up',
         0x27: 'Right', 0x28: 'Down', 0x2D: 'Insert', 0x2E: 'Delete', 0x6A: 'Numpad*', 0x6B: 'Numpad+',
         0x6D: 'Numpad-', 0x6E: 'Numpad.', 0x6F: 'Numpad/'}


def key_name(code):
    if code in NAMED:
        return NAMED[code]
    if 0x30 <= code <= 0x39 or 0x41 <= code <= 0x5A:
        return chr(code)
    if 0x60 <= code <= 0x69:
        return f'Numpad{code - 0x60}'
    if 0x70 <= code <= 0x87:
        return f'F{code - 0x6F}'
    return PUNCTUATION.get(code, f'0x{code:02X}')


def main():
    seen = {}
    for name, _internal, code, desc in records(sys.argv[1]):
        if code:
            seen.setdefault(code, []).append(name)
            print(f'{name}\t{key_name(code)}\t0x{code:02x}\t{desc}')
    clashes = {c: n for c, n in seen.items() if len(n) > 1}
    if clashes:
        print(f'# note: codes shared by several commands: {clashes}', file=sys.stderr)


if __name__ == '__main__':
    main()
