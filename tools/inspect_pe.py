#!/usr/bin/env python3
"""Read-only PE inventory for a local reference build. Does not execute the binary."""
import argparse
import hashlib
import json
import re
import struct
from pathlib import Path


def inspect(path):
    data = path.read_bytes()

    def unpack(fmt, offset):
        return struct.unpack_from('<' + fmt, data, offset)

    def cstring(offset):
        end = data.index(b'\0', offset)
        return data[offset:end].decode('utf-8', errors='replace')

    if data[:2] != b'MZ':
        raise ValueError('Not a DOS/PE executable')
    pe, = unpack('I', 0x3c)
    if data[pe:pe + 4] != b'PE\0\0':
        raise ValueError('Invalid PE signature')
    machine, count, timestamp, _, _, optional_size, _ = unpack('HHIIIHH', pe + 4)
    optional = pe + 24
    magic, = unpack('H', optional)
    if magic != 0x20b:
        raise ValueError('This inventory currently supports PE32+ only')
    entry, = unpack('I', optional + 16)
    image_base, = unpack('Q', optional + 24)
    sections = []
    for index in range(count):
        offset = optional + optional_size + index * 40
        name = data[offset:offset + 8].split(b'\0')[0].decode('ascii', errors='replace')
        virtual_size, rva, raw_size, raw_offset = unpack('IIII', offset + 8)
        sections.append(dict(name=name, rva=rva, virtual_size=virtual_size,
                             raw_size=raw_size, raw_offset=raw_offset))

    def file_offset(rva):
        for section in sections:
            delta = rva - section['rva']
            if 0 <= delta < section['raw_size']:
                return section['raw_offset'] + delta
        raise ValueError(f'RVA outside file-backed sections: {rva:#x}')

    imports = []
    directory_count, = unpack('I', optional + 108)
    if directory_count > 1:
        import_rva, import_size = unpack('II', optional + 112 + 8)
        if import_rva:
            offset = file_offset(import_rva)
            for index in range(import_size // 20):
                descriptor = unpack('IIIII', offset + index * 20)
                if not any(descriptor):
                    break
                imports.append(cstring(file_offset(descriptor[3])))

    # Locate standard VS_FIXEDFILEINFO blocks. Label as candidates, not confirmed runtime versions.
    versions = []
    for match in re.finditer(b'\xbd\x04\xef\xfe', data):
        if match.start() + 24 <= len(data):
            _, structure_version, file_ms, file_ls, product_ms, product_ls = unpack('IIIIII', match.start())
            if structure_version == 0x10000:
                def version(ms, ls):
                    return '.'.join(map(str, (ms >> 16, ms & 65535, ls >> 16, ls & 65535)))
                versions.append(dict(file=version(file_ms, file_ls), product=version(product_ms, product_ls)))
    patterns = rb'[ -~]{0,40}(?:X-Plane|blade element|PROPERTIES_BEGIN|\.acf|\.afl)[ -~]{0,100}'
    strings = sorted({m.group().decode('ascii') for m in re.finditer(patterns, data, re.IGNORECASE)})
    return dict(file=str(path), bytes=len(data), sha256=hashlib.sha256(data).hexdigest(),
                machine=hex(machine), coff_timestamp_raw=timestamp, image_base=hex(image_base),
                entry_rva=hex(entry), sections=sections, imports=sorted(set(imports)),
                fixed_file_version_candidates=versions, research_string_samples=strings[:80],
                note='Static inventory only. Does not recover flight algorithms or establish content compatibility.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('binary', type=Path)
    args = parser.parse_args()
    print(json.dumps(inspect(args.binary), indent=2, ensure_ascii=False))
