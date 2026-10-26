#!/usr/bin/env python3
"""Generate synthetic common attachment-loader vectors from the original PE.

Usage: python tools/generate_attachment_vectors.py <X-Plane.exe> <output.tsv> [aircraft.acf]
The default fixture contains only synthetic inputs and resulting loader words.
With an aircraft path, the output contains private reference property values:
keep that output in ignored local research, never commit it.
"""
import hashlib
import random
import struct
import sys
from pathlib import Path

from emulate_xp import Emulator
from run_acf_loader import build_context, parse_acf
from unicorn.x86_const import UC_X86_REG_RAX, UC_X86_REG_RCX

SHA256 = "2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
FIELDS = ["_v10_steers_with_gear", "_v10_is_internal", "_v10_att_wing",
          "_v10_att_body", "_v10_att_gear", "_v10_att_x_acf_prt_ref",
          "_v10_att_psi_ref", "_v10_att_y_acf_prt_ref", "_v10_att_the_ref",
          "_v10_att_z_acf_prt_ref", "_v10_att_phi_ref"]


def main():
    exe, output = map(Path, sys.argv[1:3])
    if len(sys.argv) > 3:
        assert output.resolve().is_relative_to(Path('research/local').resolve()), "installed-content fixtures must stay local"
    assert hashlib.sha256(exe.read_bytes()).hexdigest() == SHA256, "reference build mismatch"
    e = Emulator(exe)
    e.stubs[0x1422e7bf0] = lambda m: m.uc.reg_write(UC_X86_REG_RAX, m.alloc(max(m.reg(UC_X86_REG_RCX), 16) + 16))
    e.stubs[0x1422e7c2c] = lambda m: None
    initial = [7, 123456, 0xffffffff, 0xffffffff, 0xffffffff] + [struct.unpack("<I", struct.pack("<f", f))[0] for f in (2.0, 11.0, -3.0, 22.0, 4.0, 33.0)]
    rng = random.Random(610)
    cases = [("missing", {})]
    for part in (-1, 0, 7, 8, 55, 56, 84, 85, 94, 95):
        cases.append((f"legacy-{part}", {"_v10_att_part": str(part)}))
    cases.append(("legacy-explicit-wing", {"_v10_att_part": "85", "_v10_att_wing": "-1"}))
    for target in ("wing", "body", "gear"):
        cases.append((f"legacy-present-{target}-zero", {"_v10_att_part": "85", f"_v10_att_{target}": "0"}))
    cases.append(("empty-name", {"_v10_att_file_stl": ""}))
    for i in range(32):
        values = {f: str(rng.randint(-8, 9)) if j < 5 else str(rng.randint(-4000, 4000) / 16) for j, f in enumerate(FIELDS) if rng.randrange(3)}
        values["_v10_att_file_stl"] = "synthetic/long attachment.obj" if i % 2 else "mesh.obj"
        if i % 4 == 0:
            values.update({f: "-1" for f in FIELDS[2:5]})
        cases.append((f"mixed-{i}", values))
    if len(sys.argv) > 3:
        entries = parse_acf(sys.argv[3])
        for key in sorted(entries):
            if key.startswith(b'_obja/') and key.endswith(b'/_v10_att_file_stl') and entries[key]:
                prefix = key[:-len(b'_v10_att_file_stl')]
                values = {k[len(prefix):].decode(): v.decode() for k, v in entries.items()
                          if k.startswith(prefix) and b';' not in v}
                cases.append((f"installed-{prefix.decode()}", values))
    rows = ["# case\tproperties (key=value;...)\tinitial words\tcg_y,cg_z bits\texpected words\texpected filename"]
    for index, (name, values) in enumerate(cases):
        ctx = build_context(e, {k.encode(): v.encode() for k, v in values.items()})
        obj = e.alloc(0x120)
        e.write(obj, bytes(0x120))
        for off in (0, 0x20):
            e.write(obj + off, b"before.obj\0")
            e.write_u64(obj + off + 0x10, 10)
            e.write_u64(obj + off + 0x18, 15)
        e.write(obj + 0x40, struct.pack("<11I", *initial))
        cg = (10.25 if index % 2 else -0.75, -20.5 if index % 3 else 1.125)
        e.write_f32(0x146125cf4, cg[0])
        e.write_f32(0x146125cf8, cg[1])
        assert e.call(0x141aaa610, ints=(obj, ctx)) == 1
        names = []
        for off in (0, 0x20):
            ln, cap = e.read_u64(obj + off + 0x10), e.read_u64(obj + off + 0x18)
            ptr = obj + off if cap <= 15 else e.read_u64(obj + off)
            names.append(e.read(ptr, ln).decode())
        assert names[0] == names[1]
        words = struct.unpack("<11I", e.read(obj + 0x40, 44))
        hexes = lambda ws: ",".join(f"{w:08x}" for w in ws)
        cg_words = struct.unpack("<2I", struct.pack("<2f", *cg))
        props = ";".join(f"{k}={v}" for k, v in values.items())
        rows.append("\t".join((name, props, hexes(initial), hexes(cg_words), hexes(words), names[0])))
    output.write_text("\n".join(rows) + "\n", encoding="utf-8")
    print(f"{len(cases)} native attachment cases; exact return boundary reached; {SHA256}")


if __name__ == "__main__":
    main()
