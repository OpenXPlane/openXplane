"""Check the reference OBJ loader's loop-field assignment on supplied numbers."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_XMM0


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    frame, record = e.alloc(0x100), e.alloc(0x40)
    e.uc.reg_write(UC_X86_REG_RBP, frame + 0x20)
    e.write_u64(frame + 0x10, record)
    cases = [0.0, 1.0, 8.0, -2.5, 123.125]
    for value in cases:
        def number(emu):
            emu.uc.reg_write(UC_X86_REG_XMM0, struct.unpack('<Q', struct.pack('<d', value))[0])
        e.stubs[0x140873240] = number
        e.call(0x1408784b6, until=0x14087b7e1, max_instructions=30)
        if e.read_f32(record + 0x14) != value:
            raise ValueError('loop field mismatch')
    return {'reference_sha256': REFERENCE_SHA256, 'cases': cases,
            'range': ['0x1408784b6', '0x14087b7e1'],
            'limitations': 'Substituted numeric reader; verifies storage only, not the original loop evaluator or malformed-key handling.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
