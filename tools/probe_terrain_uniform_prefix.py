"""Compare raw composite uniform-prefix packing with original instructions."""
import argparse
import hashlib
import json
import random
from pathlib import Path

from emulate_xp import Emulator, UC_X86_REG_RAX
from probe_terrain_shader_registration import REFERENCE_SHA256


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    material, output = e.alloc(0x300), e.alloc(0x1c0)
    e.stubs[0x140002e50] = lambda emu: emu.uc.reg_write(UC_X86_REG_RAX, 0)
    rng = random.Random(0x1202)
    cases = 0
    for mode in [5, 6]:
        for index in range(256):
            words = [rng.getrandbits(32) for _ in range(16)]
            if index == 0:
                words[:8] = [0, 0x80000000, 0x7fc12345, 0xff812345,
                             0x3f800000, 0x80000000, 0x7f800000, 0xff800000]
            e.write(material, bytes(0x300))
            e.write(output, bytes(0x1c0))  # Caller zero-initializes the full block.
            e.write_u32(material + 0x44, mode)
            for i, word in enumerate(words):
                e.write_u32(material + 0x58 + 4 * i, word)
            e.call(0x1417705c0, ints=[material, output], until=0x1417709b2,
                   max_instructions=500)
            actual = [e.read_u32(output + 4 * i) for i in range(16)]
            if mode == 5:
                expected = words[:]
                expected[1] ^= 0x80000000
                expected[5] ^= 0x80000000
            else:
                expected = words[:8] + [0] * 8
                expected[5] ^= 0x80000000
            if actual != expected:
                raise ValueError(f'mode {mode} case {index}: uniform-prefix mismatch')
            cases += 1
    return {'reference_sha256': REFERENCE_SHA256, 'begin_va': '0x1417705c0',
            'exclusive_stop_va': '0x1417709b2', 'cases': cases,
            'stub_addresses': ['0x140002e50'],
            'limitations': 'Synthetic material, zeroed output and substituted texture-presence query. Checks only first 64 bytes and stops before other uniforms, upload and GPU execution.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
