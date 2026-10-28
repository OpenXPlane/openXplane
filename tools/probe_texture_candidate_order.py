"""Observe mode-1 native candidate assembly with controlled string concatenation."""
import argparse
import hashlib
import json
from pathlib import Path
from emulate_xp import Emulator, STACK_TOP
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import (UC_X86_REG_RBP, UC_X86_REG_RSP,
                               UC_X86_REG_R12, UC_X86_REG_R14,
                               UC_X86_REG_RCX, UC_X86_REG_R8,
                               UC_X86_REG_XMM6)


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    sp = STACK_TOP - 0x1000
    frame = sp + 0x100
    prefix = e.alloc(32)
    records = e.alloc(128)
    e.uc.reg_write(UC_X86_REG_RSP, sp)
    e.uc.reg_write(UC_X86_REG_RBP, frame)
    e.write_u64(frame + 0x80, prefix)
    e.run_range(0x140c3307c, 0x140c33097, max_instructions=20)
    if e.reg(UC_X86_REG_RCX) != 1 or e.reg(UC_X86_REG_R8) != prefix:
        raise ValueError('POL base-texture call mode mismatch')

    def string(address, value):
        data = value.encode('ascii')
        assert len(data) <= 15
        e.write(address, data + bytes(16 - len(data)))
        e.write_u64(address + 16, len(data))
        e.write_u64(address + 24, 15)

    observations = []
    for stem in ('tile', 'pavement'):
        seen = []
        string(prefix, 'pack/')
        for offset, extension in ((-0x88, '.dds'), (-0x68, '.png'),
                                  (-0x48, '.bmp'), (0x20, '.ktx2')):
            string(frame + offset, stem + extension)

        def concat(emu):
            call_sp = emu.reg(UC_X86_REG_RSP)
            name = emu.read_u64(call_sp + 0x30)
            length = emu.read_u64(call_sp + 0x38)
            seen.append(emu.read(name, length).decode('ascii'))
            # Supply an empty inline result; the original move/cleanup still runs.
            string(emu.reg(UC_X86_REG_RCX), '')

        e.stubs[0x1405f3830] = concat
        e.uc.reg_write(UC_X86_REG_RSP, sp)
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.uc.reg_write(UC_X86_REG_R12, prefix)
        e.uc.reg_write(UC_X86_REG_R14, 1)
        e.uc.reg_write(UC_X86_REG_XMM6, len(stem + '.ktx2'))
        e.write_u64(sp + 0x40, records)
        e.write_u64(sp + 0x48, records)
        e.write_u64(sp + 0x50, records + 128)
        e.run_range(0x1419ebce3, 0x1419ec014, max_instructions=1000)
        expected = [stem + ext for ext in ('.dds', '.png', '.bmp', '.ktx2')]
        if seen != expected or e.read_u64(sp + 0x48) != records + 128:
            raise ValueError('native candidate order mismatch')
        observations.append(seen)
    return dict(reference_sha256=REFERENCE_SHA256, mode=1,
                observations=observations,
                limitations='Nonempty prefix, prebuilt inline candidates and vector capacity; concatenation substituted. No filesystem, decoding, optional prefix replacement or other modes.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
