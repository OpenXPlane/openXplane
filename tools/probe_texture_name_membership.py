"""Execute the reference exact-name membership tail with a supplied search result."""
import argparse
import hashlib
import json
from pathlib import Path
from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RCX


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    owner, record, query, index = e.alloc(0x80), e.alloc(0x28), e.alloc(0x20), e.alloc(4)
    e.write_u64(owner + 0x68, record)
    e.write_u64(owner + 0x70, record + 0x28)

    def string(address, value):
        raw = value.encode('ascii')
        if len(raw) > 15:
            raise ValueError('probe uses inline strings only')
        e.write(address, raw + b'\0' * (16 - len(raw)))
        e.write_u64(address + 0x10, len(raw))
        e.write_u64(address + 0x18, 15)

    def search(emu):
        emu.write_u64(emu.reg(UC_X86_REG_RCX), record)

    e.stubs[0x140cec650] = search
    results = []
    for candidate, requested, expected in [('tile.png', 'tile.png', True),
                                         ('tile.png', 'tile.dds', False),
                                         ('tile.png', 'Tile.png', False),
                                         ('tile.png', 'tile.pngx', False)]:
        string(record, candidate)
        string(query, requested)
        e.write_u32(index, 0xffffffff)
        actual = bool(e.call(0x140d062c0, ints=[owner, query, index], max_instructions=10000) & 255)
        if actual != expected or (actual and e.read_u32(index) != 0):
            raise ValueError('name comparison mismatch')
        results.append({'candidate': candidate, 'requested': requested, 'matches': actual})
    return {'reference_sha256': REFERENCE_SHA256, 'cases': results,
            'limitations': 'Search result substituted; inline ASCII strings only. Original comparison executes, but no filesystem access, complete search or fallback selection is tested.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
