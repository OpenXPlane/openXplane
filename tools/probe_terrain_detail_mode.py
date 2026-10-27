"""Observe the DETAIL arithmetic block, after directive recognition only."""
import argparse
import hashlib
import json
import struct
from pathlib import Path

from emulate_xp import Emulator, UC_X86_REG_RAX
from probe_terrain_shader_registration import REFERENCE_SHA256
from unicorn.x86_const import UC_X86_REG_RBP, UC_X86_REG_XMM10


def probe(path):
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != REFERENCE_SHA256:
        raise ValueError("unsupported reference identity")
    e = Emulator(path)
    frame = e.alloc(0x400)
    observations = []
    for scale in [0.25, 1.0, 2.0, 16.0, 100.0, 1024.0]:
        reads = []

        def read_float(emu):
            reads.append(scale)
            emu.set_xmm_f32(0, scale)

        e.stubs[0x1406ca600] = read_float
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.uc.reg_write(UC_X86_REG_XMM10, struct.unpack('<Q', struct.pack('<d', 1.0))[0])
        e.call(0x1419a84d1, until=0x1419a851e, max_instructions=100)
        actual = [e.read_f32(frame + o) for o in (0xe8, 0xec, 0xf0, 0xf4)]
        expected = struct.unpack('<f', struct.pack('<f', 1.0 / scale))[0]
        if actual != [expected, expected, 1.0, 1.0] or len(reads) != 1:
            raise ValueError("DETAIL block mismatch")
        if e.read_u32(frame + 0xd4) != 9:
            raise ValueError("DETAIL did not select mode9")
        observations.append({"scale": scale, "fields_e8_ec_f0_f4": actual, "texture_mode": 9})
    projected = []
    for scales in [(32, 64), (4096, 2048)]:
        pending = iter(scales)

        def read_int(emu):
            emu.uc.reg_write(UC_X86_REG_RAX, next(pending))

        e.stubs[0x1406ca750] = read_int
        e.write_u32(frame + 0xd4, 5)
        e.uc.reg_write(UC_X86_REG_RBP, frame)
        e.call(0x1419a7e23, until=0x1419a7e59, max_instructions=100)
        actual = [e.read_f32(frame + o) for o in (0x290, 0x294)]
        if actual != list(scales) or e.read_u32(frame + 0x2b0) != 1:
            raise ValueError("PROJECTED fields mismatch")
        if e.read_u32(frame + 0xd4) != 5:
            raise ValueError("PROJECTED unexpectedly changed texture mode")
        projected.append({"integer_scales": scales, "stored_scales": actual,
                          "texture_mode_preserved": 5, "projection_field": 1})
    return {"reference_sha256": REFERENCE_SHA256, "begin_va": "0x1419a84d1",
            "exclusive_stop_va": "0x1419a851e", "observations": observations,
            "projected_block": {"begin_va": "0x1419a7e23", "exclusive_stop_va": "0x1419a7e59",
                                "observations": projected},
            "stub_addresses": ["0x1406ca600", "0x1406ca750"],
            "limitations": "Starts after token recognition; number parsers substituted, frame synthetic, XMM10 supplied as double1. Does not execute the complete TER loader, texture upload or shader."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable")
    print(json.dumps(probe(parser.parse_args().executable), indent=2))
