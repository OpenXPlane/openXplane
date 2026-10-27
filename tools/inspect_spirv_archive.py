#!/usr/bin/env python3
"""Inventory shader candidates without executing them or emitting source code.

Usage: python tools/inspect_spirv_archive.py terrain.xsa --resource tex_comp
Keep reports from original installations in ignored local research.
This is structural inspection, not SPIR-V validation or shader decompilation.
"""
import argparse
import collections
import hashlib
import json
import struct
from pathlib import Path
from zipfile import ZipFile


def inspect_module(raw):
    if len(raw) < 20 or len(raw) % 4:
        raise ValueError("invalid SPIR-V byte length")
    words = struct.unpack(f"<{len(raw) // 4}I", raw)
    if words[0] != 0x07230203 or words[3] == 0 or words[4] != 0:
        raise ValueError("unsupported SPIR-V header")
    names = {}
    entries = []
    decorations = collections.defaultdict(dict)
    counts = collections.Counter()
    handles = {}
    samples = []
    offset = 5
    while offset < len(words):
        size, opcode = words[offset] >> 16, words[offset] & 65535
        if not size or offset + size > len(words):
            raise ValueError("truncated or zero-size SPIR-V instruction")
        args = words[offset + 1:offset + size]
        counts[opcode] += 1
        if opcode == 5:  # OpName
            if len(args) < 2:
                raise ValueError("truncated OpName")
            encoded = raw[(offset + 2) * 4:(offset + size) * 4]
            if b"\0" not in encoded:
                raise ValueError("unterminated OpName")
            names[args[0]] = encoded.split(b"\0", 1)[0].decode("utf-8")
        elif opcode == 15:  # OpEntryPoint
            if len(args) < 3:
                raise ValueError("truncated OpEntryPoint")
            entries.append({"execution_model": args[0], "function_id": args[1]})
        elif opcode == 71 and len(args) == 3 and args[1] in (33, 34):
            decorations[args[0]]["binding" if args[1] == 33 else "set"] = args[2]
        elif opcode in (61, 83, 86):  # Load, CopyObject, SampledImage
            required = 4 if opcode == 86 else 3
            if len(args) < required:
                raise ValueError("truncated image-handle instruction")
            handles[args[1]] = args[2:required]
        elif opcode in (87, 88):  # Implicit/explicit LOD samples
            if len(args) < 4:
                raise ValueError("truncated image sample")
            samples.append({"byte_offset": offset * 4, "result_id": args[1],
                            "image_id": args[2], "coordinate_id": args[3],
                            "lod": "implicit" if opcode == 87 else "explicit"})
        offset += size

    def resolve(identifier, active):
        if identifier in active or len(active) >= 64:
            return set(), {identifier}
        if identifier in handles:
            roots, unresolved = set(), set()
            for child in handles[identifier]:
                found, missing = resolve(child, active | {identifier})
                roots.update(found)
                unresolved.update(missing)
            return roots, unresolved
        if identifier in names:
            return {identifier}, set()
        return set(), {identifier}

    for sample in samples:
        roots, missing = resolve(sample["image_id"], set())
        sample["named_handle_roots"] = [{"id": i, "name": names[i], **decorations[i]}
                                        for i in sorted(roots)]
        sample["unresolved_handle_ids"] = sorted(missing)
    return {
        "sha256": hashlib.sha256(raw).hexdigest(),
        "bytes": len(raw),
        "version_word": hex(words[1]),
        "entry_points": entries,
        "named_ids": [{"id": key, "name": value, **decorations[key]}
                      for key, value in sorted(names.items()) if value],
        "instruction_count": sum(counts.values()),
        "spec_constant_op_count": counts[52],
        "implicit_sample_count": counts[87],
        "image_samples": samples,
    }


def inspect_archive(path, resource, limit, member=None):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    candidates = []
    models = collections.Counter()
    with ZipFile(path) as archive:
        entries = archive.infolist()
        if len(entries) > 100000 or sum(e.file_size for e in entries) > 512 * 1024 * 1024:
            raise ValueError("archive exceeds inspection bounds")
        if len({e.filename for e in entries}) != len(entries):
            raise ValueError("duplicate archive member names")
        for entry in entries:
            if not entry.filename.endswith(".spv"):
                continue
            if entry.file_size > 16 * 1024 * 1024:
                raise ValueError("shader exceeds inspection bounds")
            module = inspect_module(archive.read(entry))  # ZIP CRC checked here
            for ep in module["entry_points"]:
                models[ep["execution_model"]] += 1
            if (entry.filename == member if member is not None else
                    any(n["name"] == resource for n in module["named_ids"])):
                candidates.append({"member": entry.filename, **module})
    candidates.sort(key=lambda row: (row["bytes"], row["member"]))
    return {"archive_sha256": digest.hexdigest(), "archive_entries": len(entries),
            "entry_points_by_execution_model": dict(models), "resource": resource,
            "candidate_count": len(candidates), "candidates": candidates[:limit],
            "selected_member": member,
            "limitations": ["Debug resource names do not establish runtime selection or arithmetic.",
                            "No specialization, execution, decompilation or full SPIR-V validation."]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--resource", default="tex_comp")
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--member", help="select an exact archive member instead of filtering names")
    args = parser.parse_args()
    if not 1 <= args.limit <= 100:
        parser.error("limit must be between1 and100")
    print(json.dumps(inspect_archive(args.archive, args.resource, args.limit, args.member), indent=2))


if __name__ == "__main__":
    main()
