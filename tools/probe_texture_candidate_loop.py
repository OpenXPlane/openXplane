"""Check original candidate iteration with controlled filesystem answers."""
import argparse
import hashlib
import json
from pathlib import Path
from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RSP, UC_X86_REG_RCX, UC_X86_REG_RAX


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError('unsupported reference identity')
    e = Emulator(path)
    records = e.alloc(96)
    observations = []
    for hit in (None, 0, 1, 2):
        seen = []
        def check(emu):
            i = (emu.reg(UC_X86_REG_RCX) - records) // 32
            seen.append(i)
            emu.uc.reg_write(UC_X86_REG_RAX, int(i == hit))
        e.stubs[0x141620280] = lambda emu: None
        e.stubs[0x1415404e0] = check
        # Prepare the original loop's stack slots without executing its prologue.
        from emulate_xp import STACK_TOP
        sp = STACK_TOP - 0x1000
        e.uc.reg_write(UC_X86_REG_RSP, sp)
        e.write_u64(sp + 0x40, records)
        e.write_u64(sp + 0x48, records + 96)
        stop = 0x1419ec583 if hit is None else 0x1419ec795
        e.run_range(0x1419ec550, stop, max_instructions=200)
        expected = list(range(3 if hit is None else hit + 1))
        if seen != expected:
            raise ValueError('candidate iteration mismatch')
        observations.append({'first_hit': hit, 'visited_indices': seen})
    return {'reference_sha256': REFERENCE_SHA256, 'observations': observations,
            'limitations': 'Path normalization and filesystem predicate substituted; synthetic vector. Tests iteration and first-success stop only, not extension ordering or real file access.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('executable')
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
