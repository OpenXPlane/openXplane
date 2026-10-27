"""Observe a bounded material-to-variant stage on synthetic reference state."""
import argparse
import hashlib
from itertools import product
import json
import struct
from pathlib import Path

from emulate_xp import Emulator, UC_X86_REG_RAX, UC_X86_REG_RCX, UC_X86_REG_RDX
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn import UC_HOOK_CODE


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError("unsupported reference identity")
    e = Emulator(path)
    material, output = e.alloc(0x300), e.alloc(0x100)

    def zero(emu):
        emu.uc.reg_write(UC_X86_REG_RAX, 0)

    def resize(emu):
        vector, count = emu.reg(UC_X86_REG_RCX), emu.reg(UC_X86_REG_RDX)
        if count not in (5, 11):
            raise ValueError("unexpected vector size")
        data = emu.alloc(count * 4)
        emu.write_u64(vector, data)
        emu.write_u64(vector + 8, data + count * 4)
        emu.write_u64(vector + 16, data + count * 4)

    e.stubs.update({0x140ca7e50: zero, 0x140b4bdd0: zero,
                    0x14093ccc0: resize, 0x141991300: zero})

    def guard(uc, address, size, user):
        if address not in e.stubs and not 0x14176b3f0 <= address <= 0x14176b757:
            raise ValueError(f"unexpected native callee {address:#x}")

    e.uc.hook_add(UC_HOOK_CODE, guard)
    observations = []
    for rendering_mode, texture_mode, projected in product(range(5), range(18), (False, True)):
            e.write(material, bytes(0x300))
            e.write(output, bytes(0x100))
            e.write_u32(material + 0x44, texture_mode)
            e.write_u32(material + 0x220, 0 if projected else 0xffffffff)
            e.write_u32(material + 0x224, 1)  # Recorded border digit; not a TER rule.
            e.write_u32(material + 0x1b4, 1)  # Recorded geometry digit.
            e.write_u32(material + 0x198, 1)  # Avoid rendering-mode1 texture fallback.
            e.call(0x14176b3f0, ints=[output, material - 8, material, rendering_mode],
                   stack=[0, 0, 0], until=0x14176b757, max_instructions=2000)
            digits = list(struct.unpack("<11I", e.read(e.read_u64(output), 44)))
            expected = [rendering_mode, 1, 0, 0, 1, texture_mode, int(projected), 0, 0, 0, 0]
            if digits != expected:
                raise ValueError(f"unexpected material digits {digits} != {expected}")
            observations.append({"rendering_mode": rendering_mode, "texture_mode": texture_mode,
                                 "projected": projected, "digits": digits})
    return {"reference_sha256": REFERENCE_SHA256,
            "entry_va": "0x14176b3f0", "exclusive_stop_va": "0x14176b757",
            "observations": observations,
            "stub_addresses": [hex(a) for a in e.stubs],
            "limitations": "Synthetic material fields; texture queries return absent, vector resize is substituted and feature query returns false. Stops before native validation. No TER loading, live state or complete material equivalence is verified."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable")
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
