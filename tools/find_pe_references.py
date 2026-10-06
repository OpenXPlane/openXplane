#!/usr/bin/env python3
"""Find candidate references to a VA without executing the PE.

RIP candidates assume the disp32 ends an instruction. They miss instructions
with trailing immediates and can match unrelated bytes. Confirm boundaries and
operand meanings with a disassembler. Qword candidates are not decoded records.
"""
import argparse
import bisect
import json
import struct
from pathlib import Path
from inspect_pe import inspect


def find(path, target, mode):
    inventory = inspect(path)
    data = path.read_bytes()
    base = int(inventory['image_base'], 16)
    sections = inventory['sections']
    functions = []
    for section in sections:
        if section['name'] == '.pdata':
            for offset in range(section['raw_offset'], section['raw_offset'] + section['raw_size'] - 11, 12):
                begin, end, _ = struct.unpack_from('<III', data, offset)
                if begin and begin < end:
                    functions.append((base + begin, base + end))
    functions.sort()
    starts = [f[0] for f in functions]
    records = []

    def record(section, offset, kind):
        address = base + section['rva'] + offset - section['raw_offset']
        result = dict(kind=kind, file_offset=hex(offset), field_va=hex(address),
                      section=section['name'])
        if kind == 'rip_disp32_candidate':
            result['context_bytes'] = data[max(section['raw_offset'], offset - 8):offset + 4].hex()
            index = bisect.bisect_right(starts, address) - 1
            if index >= 0 and functions[index][0] <= address < functions[index][1]:
                result['pdata_range'] = [hex(v) for v in functions[index]]
        records.append(result)

    for section in sections:
        start = section['raw_offset']
        end = min(start + section['raw_size'], len(data))
        if mode in ('rip', 'all') and section['name'] == '.text':
            origin = base + section['rva'] - start
            for offset in range(start, end - 3):
                displacement = struct.unpack_from('<i', data, offset)[0]
                if origin + offset + 4 + displacement == target:
                    record(section, offset, 'rip_disp32_candidate')
        if mode in ('qword', 'all'):
            needle = struct.pack('<Q', target)
            offset = data.find(needle, start, end)
            while offset >= 0:
                record(section, offset, 'qword_candidate')
                offset = data.find(needle, offset + 1, end)
    return dict(sha256=inventory['sha256'], target_va=hex(target), mode=mode,
                references=records,
                note='Byte-pattern candidates only. field_va is not an instruction start. '
                     'pdata_range is present only when the field lies inside that range; '
                     'leaf functions may have no pdata entry.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('--va', required=True, type=lambda text: int(text, 0))
    parser.add_argument('--mode', choices=['rip', 'qword', 'all'], default='all')
    args = parser.parse_args()
    print(json.dumps(find(args.binary, args.va, args.mode), indent=2))
