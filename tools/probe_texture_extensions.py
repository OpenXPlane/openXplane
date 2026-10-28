"""Execute original short-name texture extension replacement blocks."""
import argparse
import hashlib
import json
from pathlib import Path
from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_RSP


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    memory = e.alloc(0x300)
    frame = memory + 0x100
    results = []
    for extension, offset, begin, end in [('.dds', -0x88, 0x1419eb98c, 0x1419eba1f),
                                         ('.bmp', -0x48, 0x1419eba42, 0x1419ebad0),
                                         ('.png', -0x68, 0x1419ebaf3, 0x1419ebb81)]:
        for source in ('tile.dds', 'tile.png', 'tile.bmp', 'abc.tga'):
            address = frame + offset
            e.write(address, source.encode() + b'\0' * (16 - len(source)))
            e.write_u64(address + 16, len(source))
            e.write_u64(address + 24, 15)
            e.uc.reg_write(UC_X86_REG_RBP, frame)
            e.uc.reg_write(UC_X86_REG_RSP, frame - 0x100)
            e.run_range(begin, end, max_instructions=100)
            actual = e.read(address, e.read_u64(address + 16)).decode()
            if actual != source[:-4] + extension:
                raise ValueError('extension rewrite mismatch')
            results.append({'source': source, 'candidate': actual})
    return {'reference_sha256': REFERENCE_SHA256, 'cases': results,
            'limitations': 'Original replacement blocks, inline names longer than four bytes only; no allocation, path normalization, candidate ordering or filesystem access.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
