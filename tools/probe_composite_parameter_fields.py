"""Observe composite numeric field storage after directive recognition."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from emulate_xp import Emulator, UC_X86_REG_RAX
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_RSI, UC_X86_REG_XMM10


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    frame, record, reader, token = e.alloc(0x200), e.alloc(0x80), e.alloc(0x60), e.alloc(1)
    e.write(frame + 0xe8, b'\xff' * 0x3c)
    e.uc.reg_write(UC_X86_REG_RBP, frame)
    e.call(0x1419a7616, until=0x1419a765a, max_instructions=30)
    reset = [e.read_f32(frame + 0xe8 + 4 * i) for i in range(15)]
    expected_reset = [1.0, 1.0, 0.1, 0.1, 0.0625, 8.0, 0.5, 0.5,
                      0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
    expected_reset = [struct.unpack('<f', struct.pack('<f', v))[0] for v in expected_reset]
    if reset != expected_reset:
        raise ValueError(f'COMPOSITE_TEX reset mismatch: {reset}')
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
    ter_observations = []
    for optional in (False, True):
        supplied = [0.25, 0.5, 1.0, 2.0, 4.0, 8.0] + ([16.0] if optional else [])
        pending = iter(supplied)
        reads = []

        def ter_number(emu):
            value = next(pending)
            reads.append(value)
            emu.set_xmm_f32(0, value)

        def has_optional(emu):
            emu.uc.reg_write(UC_X86_REG_RAX, int(optional))

        e.stubs.update({0x1406ca600: ter_number, 0x14072f8c0: has_optional})
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.write(frame + 0xf0, b'\xff' * 28)
        e.call(0x1419a9f82, until=0x1419ab53b, max_instructions=100)
        actual = [e.read_f32(frame + 0xf0 + 4 * i) for i in range(7)]
        if actual != supplied + ([] if optional else [0.0]) or reads != supplied:
            raise ValueError('TER composite field mismatch')
        ter_observations.append({'optional_seventh': optional, 'stored_fields': actual})
    projections = []
    for scales in ([2.0, 4.0], [0.1, 123.0], [0.25, 4096.0]):
        scale32 = [struct.unpack('<f', struct.pack('<f', v))[0] for v in scales]
        pending = iter(scale32)

        def read_scale(emu):
            emu.set_xmm_f32(0, next(pending))

        e.stubs[0x1406ca600] = read_scale
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        e.call(0x1419a8316, until=0x1419a8360, max_instructions=100)
        actual = [e.read_f32(frame + 0xe8), e.read_f32(frame + 0xec)]
        expected = [struct.unpack('<f', struct.pack('<f', 1.0 / v))[0] for v in scale32]
        if actual != expected:
            raise ValueError('TER composite projection mismatch')
        projections.append({'input': scale32, 'reciprocals': actual})
    return {'reference_sha256': REFERENCE_SHA256, 'texture_reset': {'range': ['0x1419a7616', '0x1419a765a'], 'fields': reset}, 'begin_va': '0x140c2e780',
            'exclusive_stop_va': '0x140c2f1c0', 'observations': observations,
            'ter_block': {'begin_va': '0x1419a9f82', 'exclusive_stop_va': '0x1419ab53b',
                          'observations': ter_observations},
            'ter_projection': {'begin_va': '0x1419a8316', 'exclusive_stop_va': '0x1419a8360',
                               'observations': projections},
            'stub_addresses': ['0x1406ca600', '0x14061d8d0', '0x14072f8c0'],
            'limitations': 'Starts after token recognition; number/whitespace/optional-token queries substituted, cursor and XMM10 synthetic. Verifies storage order and reciprocal arithmetic only; not complete TER parsing or GPU execution.'}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('executable')
    print(json.dumps(probe(p.parse_args().executable), indent=2))
