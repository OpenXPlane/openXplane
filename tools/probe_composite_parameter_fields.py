"""Observe composite numeric field storage after directive recognition."""
import argparse
import hashlib
import json
from pathlib import Path
from emulate_xp import Emulator, UC_X86_REG_RAX
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_RSI


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    frame, record, reader, token = e.alloc(0x200), e.alloc(0x80), e.alloc(0x60), e.alloc(1)
    observations = []
    for optional in (False, True):
        supplied = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0] + ([16.0] if optional else [])
        pending = iter(supplied)
        reads = []

        def number(emu):
            v = next(pending)
            reads.append(v)
            emu.set_xmm_f32(0, v)

        def whitespace(emu):
            emu.uc.reg_write(UC_X86_REG_RAX, 0)

        e.stubs.update({0x1406ca600: number, 0x14061d8d0: whitespace})
        e.write(record, b'\xff' * 0x80)
        e.write(token, b'7')
        e.write_u64(reader + 0x38, token)
        e.write_u64(reader + 0x40, token + int(optional))
        e.write_u64(frame + 0x77, record)
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.uc.reg_write(UC_X86_REG_RSI, reader)
        e.call(0x140c2e780, until=0x140c2f1c0, max_instructions=200)
        actual = [e.read_f32(record + 0x20 + 4 * i) for i in range(7)]
        expected = supplied if optional else supplied + [0.0]
        if actual != expected or reads != supplied:
            raise ValueError('composite field mismatch')
        observations.append({'optional_seventh': optional, 'stored_fields': actual})
    return {'reference_sha256': REFERENCE_SHA256, 'begin_va': '0x140c2e780',
            'exclusive_stop_va': '0x140c2f1c0', 'observations': observations,
            'stub_addresses': ['0x1406ca600', '0x14061d8d0'],
            'limitations': 'Starts after token recognition; number/whitespace queries substituted and cursor synthetic. Verifies field order and absent optional value only; not complete TER parsing or shader uniform mapping.'}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('executable')
    print(json.dumps(probe(p.parse_args().executable), indent=2))
