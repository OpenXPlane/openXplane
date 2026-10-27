"""Observe terrain registration in a known reference PE with disclosed stubs."""
import argparse
import hashlib
import json
import math
from pathlib import Path

from emulate_xp import (Emulator, RET_ADDR, UC_X86_REG_RAX, UC_X86_REG_RCX,
                        UC_X86_REG_RDX, UC_X86_REG_R8)
from unicorn import UC_HOOK_CODE

REFERENCE_SHA256 = "2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
FUNCTIONS = [(0x14176e120, 0x14176eb6d),
             (0x1409168d0, 0x140916957), (0x1417697a0, 0x141769828),
             (0x14068e2c0, 0x14068e38f), (0x14103cdb0, 0x14103cdc8)]


def probe(path):
    digest = hashlib.sha256(Path(path).read_bytes()).hexdigest()
    if digest != REFERENCE_SHA256:
        raise ValueError("unsupported executable identity; addresses are build-specific")
    emu = Emulator(path)
    obj = emu.alloc(0x1000)
    captures = []

    def string(address):
        length = emu.read_u64(address + 16)
        if length > 255:
            raise ValueError("unexpected string length")
        source = address if emu.read_u64(address + 24) <= 15 else emu.read_u64(address)
        return emu.read(source, length)

    def put(address, data):
        emu.write(address, b"\0" * 32)
        source = address
        if len(data) > 15:
            source = emu.alloc(len(data) + 1)
            emu.write_u64(address, source)
        emu.write(source, data + b"\0")
        emu.write_u64(address + 16, len(data))
        emu.write_u64(address + 24, max(15, len(data)))
        emu.uc.reg_write(UC_X86_REG_RAX, address)

    def assign(e):
        put(e.reg(UC_X86_REG_RCX), e.read(e.reg(UC_X86_REG_RDX), e.reg(UC_X86_REG_R8)))

    def copy(e):
        put(e.reg(UC_X86_REG_RCX), string(e.reg(UC_X86_REG_RDX)))

    def capture(e):
        offset = e.reg(UC_X86_REG_RCX) - obj
        if offset not in (0x58, 0x1b0, 0x308):
            raise ValueError("unexpected destination vector")
        start = e.read_u64(e.reg(UC_X86_REG_RDX))
        end = e.read_u64(e.reg(UC_X86_REG_RDX) + 8)
        stride = 32 if offset == 0x308 else 40
        if end < start or (end - start) % stride or end - start > 4096:
            raise ValueError("unexpected vector bounds")
        entries = []
        for address in range(start, end, stride):
            record = {"name": string(address).decode("ascii")}
            if offset == 0x58:
                record["range"] = e.read_u32(address + 32)
            elif offset == 0x1b0:
                record["flag"] = e.read(address + 32, 1)[0]
            entries.append(record)
        captures.append({"offset": hex(offset), "entries": entries})

    emu.stubs.update({
        0x1405df240: assign, 0x1405dcad0: assign,
        0x1405dd5c0: copy, 0x140917b20: capture, 0x141769b40: capture,
        0x140917c10: capture, 0x1422e8078: lambda e: None,
        0x1422e7c2c: lambda e: None,
    })

    def guard(uc, address, size, user):
        if address == RET_ADDR or address in emu.stubs:
            return
        if not any(start <= address < end for start, end in FUNCTIONS):
            raise ValueError(f"unexpected native callee at {address:#x}")

    emu.uc.hook_add(UC_HOOK_CODE, guard)
    result = emu.call(FUNCTIONS[0][0], ints=[obj], max_instructions=50000)
    if result != obj or string(obj) != b"terrain":
        raise ValueError("registration did not return the expected terrain object")
    if [c["offset"] for c in captures] != ["0x58", "0x1b0", "0x308"]:
        raise ValueError("registration capture incomplete")
    stage_start = emu.read_u64(obj + 0x30)
    stage_end = emu.read_u64(obj + 0x38)
    if stage_start != obj + 0x48 or stage_end - stage_start != 2:
        raise ValueError("unexpected inline stage vector")
    return {
        "reference_sha256": digest,
        "entry_va": hex(FUNCTIONS[0][0]),
        "native_helpers": [hex(a) for a, _ in FUNCTIONS[1:]],
        "stage_bytes": list(emu.read(stage_start, 2)),
        "stub_addresses": [hex(a) for a in emu.stubs],
        "captures": captures,
        "range_product": math.prod(r["range"] for r in captures[0]["entries"]),
        "limitations": "Zero-initialized scratch object with native base initialization; string operations, vector assignment, destruction and free are stubbed. No live draw state, index encoding or shader execution is verified.",
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    args = parser.parse_args()
    print(json.dumps(probe(args.executable), indent=2))


if __name__ == "__main__":
    main()
