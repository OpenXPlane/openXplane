#!/usr/bin/env python3
"""Find candidate RIP-relative LEA references to ACF strings in PE executable code.

Instruction byte patterns can match non-instruction data. Confirm every candidate
with a disassembler before interpreting it as behavior. Does not execute input.
"""
import argparse
import bisect
import json
import re
import struct
from pathlib import Path
from inspect_pe import inspect


def find(path, extra_tokens=()):
    inventory = inspect(path)
    data = path.read_bytes()
    sections = inventory['sections']
    base = int(inventory['image_base'], 16)

    def rva(offset):
        for s in sections:
            if s['raw_offset'] <= offset < s['raw_offset'] + s['raw_size']:
                return s['rva'] + offset - s['raw_offset']
        raise ValueError('File offset outside sections')

    targets = {}
    tokens = [b'PROPERTIES_BEGIN', b'PROPERTIES_END', b'_v10_att_file_stl',
              b'_afl_file_1', b'_m_empty', b'_Croot']
    tokens.extend(t.encode('ascii') for t in extra_tokens)
    for token in tokens:
        for match in re.finditer(re.escape(token), data):
            offset = match.start()
            start = data.rfind(b'\0', max(0, offset - 120), offset) + 1
            end = data.find(b'\0', offset)
            raw = data[start:end]
            if len(raw) > 200 or any(c < 32 or c > 126 for c in raw):
                continue
            addr = rva(start)
            targets[addr] = dict(string=raw.decode(), string_rva=hex(addr),
                                 string_va=hex(base + addr), references=[])

    pdata = next((s for s in sections if s['name'] == '.pdata'), None)
    functions = []
    if pdata:
        for offset in range(pdata['raw_offset'], pdata['raw_offset'] + pdata['raw_size'] - 11, 12):
            begin, end, unwind = struct.unpack_from('<III', data, offset)
            if begin and begin < end:
                functions.append((begin, end, unwind))
    functions.sort()
    starts = [f[0] for f in functions]
    for s in sections:
        if s['name'] != '.text':
            continue
        code = data[s['raw_offset']:s['raw_offset'] + s['raw_size']]
        for match in re.finditer(rb'[\x48-\x4f]\x8d[\x05\x0d\x15\x1d\x25\x2d\x35\x3d][\s\S]{4}', code):
            instruction = s['rva'] + match.start()
            displacement = struct.unpack_from('<i', match.group(), 3)[0]
            target = instruction + 7 + displacement
            if target not in targets:
                continue
            record = dict(instruction_rva=hex(instruction), instruction_va=hex(base + instruction), bytes=match.group().hex())
            index = bisect.bisect_right(starts, instruction) - 1
            if index >= 0 and instruction < functions[index][1]:
                begin, end, _ = functions[index]
                record.update(function_begin_va=hex(base + begin), function_end_va=hex(base + end))
            targets[target]['references'].append(record)
    return dict(sha256=inventory['sha256'], targets=list(targets.values()),
                note='Candidate LEA cross references, not recovered function names or algorithms. Verify instruction boundaries with a disassembler.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    parser.add_argument('--token', action='append', default=[], help='Additional ASCII token to investigate')
    args = parser.parse_args()
    print(json.dumps(find(args.binary, args.token), indent=2))
