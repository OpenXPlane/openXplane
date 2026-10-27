"""Compare slot arithmetic with the known reference machine code in Unicorn."""
import argparse
import json
import random
import struct

from emulate_xp import Emulator
from probe_terrain_shader_registration import REFERENCE_SHA256, probe
from shader_variant_index import decode, encode


def compare(path):
    registration = probe(path)  # Includes executable identity and native guards.
    assert registration["reference_sha256"] == REFERENCE_SHA256
    entries = registration["captures"][0]["entries"]
    ranges = [r["range"] for r in entries]
    emu = Emulator(path)
    shader, state = emu.alloc(0x1200), emu.alloc(0x100)
    records = emu.alloc(40 * len(ranges))
    values_address = emu.alloc(4 * len(ranges))
    emu.write_u64(shader + 0xf88, records)
    emu.write_u64(shader + 0xf90, records + 40 * len(ranges))
    emu.write_u64(state, values_address)
    for i, radix in enumerate(ranges):
        emu.write_u32(records + 40 * i + 32, radix)
    states = [[0] * len(ranges), [r - 1 for r in ranges]]
    for i, radix in enumerate(ranges):
        v = [0] * len(ranges)
        v[i] = radix - 1
        states.append(v)
    rng = random.Random(0x58535636)
    states += [[rng.randrange(r) for r in ranges] for _ in range(128)]
    checks = 0
    for mode in [0, 1]:
        emu.write_u32(shader + 0x112c, mode)
        for a in range(2):
            for b in range(2):
                emu.write(state + 0x80, bytes([b]))
                for values in states:
                    emu.write(values_address, struct.pack("<" + "I" * len(values), *values))
                    actual = emu.call(0x14104f0a0, ints=[shader, state, a], max_instructions=1000)
                    expected = encode(ranges, values, [a, b] if mode == 0 else None)
                    if actual != expected:
                        raise ValueError(f"slot mismatch: {actual} != {expected}")
                    checks += 1
    candidates = []
    for slot in [13465, 39385]:
        prefix, values = decode(ranges, slot)
        candidates.append({"slot": slot, "prefix": prefix,
                           "parameters": dict(zip([r["name"] for r in entries], values))})
    return {"reference_sha256": REFERENCE_SHA256, "function_va": "0x14104f0a0",
            "comparisons": checks, "stubs_in_index_function": [],
            "slot_capacity_mode0": registration["range_product"] * 4,
            "candidates_under_mode0": candidates,
            "limitations": "Synthetic valid digits and layout; declaration probe retains disclosed stubs. Prefix meanings and active live draw state are not verified."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable")
    print(json.dumps(compare(parser.parse_args().executable), indent=2))
