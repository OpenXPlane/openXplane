"""Inspect recovered XSV6 records without executing original shaders."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

MAX_BYTES = 64 * 1024 * 1024


def inspect(data, layouts=False, variants=False, limit=8, module_id=None):
    if len(data) > MAX_BYTES:
        raise ValueError("mapping exceeds inspection limit")
    position = 0

    def take(size):
        nonlocal position
        end = position + size
        if end > len(data):
            raise ValueError(f"truncated XSV prefix at offset {position}")
        result = data[position:end]
        position = end
        return result

    def text():
        # Decode for display only: this does not assign resource semantics.
        return take(take(1)[0]).decode("ascii")

    if take(4) != b"XSV6":
        raise ValueError("expected XSV6 signature")
    count = struct.unpack("<H", take(2))[0]
    names = [text() for _ in range(count)]
    names_end = position
    # The reader assembles these words little-endian, then consumes a byte
    # and a length-prefixed entry string. Their full semantics remain unknown.
    word_0, word_1 = struct.unpack("<II", take(8))
    flags = take(1)[0]
    entry = text()
    report = {
        "sha256": hashlib.sha256(data).hexdigest(),
        "size_bytes": len(data),
        "signature": "XSV6",
        "names": names,
        "name_count": count,
        "names_end_offset": names_end,
        "word_0": word_0,
        "word_1": word_1,
        "flags_byte": flags,
        "entry_string": entry,
        "opaque_payload_offset": position,
        "opaque_payload_bytes": len(data) - position,
        "limitations": "Prefix inspection only; no variant selection or shader execution",
    }
    if layouts or variants:
        groups = []
        group_count = take(1)[0]
        if group_count > 4:
            raise ValueError("more than four mapping groups")
        for _ in range(group_count):
            kind, field_count = take(2)
            fields = [list(take(2)) for _ in range(field_count)]
            groups.append({"kind": kind, "fields": fields})
        layout_count = struct.unpack("<I", take(4))[0]
        if layout_count > len(data) - position:
            raise ValueError("layout count exceeds remaining bytes")
        records = []
        for _ in range(layout_count):
            resources = []
            resource_count = take(1)[0]
            for _ in range(resource_count):
                fields = list(take(4))
                name_index = struct.unpack("<H", take(2))[0]
                if name_index >= len(names):
                    raise ValueError("resource name index outside name table")
                tail = list(take(3))
                resources.append({"fields": fields, "name_index": name_index,
                                  "name": names[name_index], "tail": tail})
            records.append(resources)
        report.update({"groups": groups, "layouts": records,
                       "layout_count": layout_count,
                       "opaque_payload_offset": position,
                       "opaque_payload_bytes": len(data) - position,
                       "limitations": "Serialized layouts only; native runtime remapping, variant selection and shader execution are not reproduced"})
    if variants:
        if flags & ~7 or not flags & 5:
            raise ValueError("unsupported stage flags")
        conditions = []
        condition_count = struct.unpack("<H", take(2))[0]
        for _ in range(condition_count):
            kind, item_count = take(2)
            items = []
            for _ in range(item_count):
                key, operation = take(2)
                extra_size = {1: 2, 3: 2, 4: 3}.get(operation, 0)
                items.append({"key": key, "operation": operation,
                              "extra": list(take(extra_size))})
            conditions.append({"kind": kind, "items": items})
        if word_0 > len(data) - position:
            raise ValueError("variant count exceeds remaining bytes")
        # Every live stage has serialized IDs, also after bit0 construction.
        if word_1 > 256:
            raise ValueError("stage repetition count exceeds inspection limit")
        valid_count = 0
        samples = []
        stage_ids = set()
        matching_count = 0
        for index in range(word_0):
            live = take(1)[0]
            if not live:
                continue
            valid_count += 1
            layout_index = struct.unpack("<I", take(4))[0]
            if layout_index >= layout_count:
                raise ValueError("variant layout index outside layout table")
            indices = list(struct.unpack("<4H", take(8)))
            for condition_index in indices:
                if condition_index != 65535 and condition_index >= condition_count:
                    raise ValueError(
                        f"variant {index} condition {condition_index} outside "
                        f"table of {condition_count} at {position}")
            attributes = []
            if flags & 1:
                attributes = [list(take(4)) for _ in range(take(1)[0])]
            stages = []
            for _ in range(word_1):
                stage = []
                for _ in range(flags.bit_count()):
                    kind = take(1)[0]
                    if kind not in (1, 2, 4):
                        raise ValueError("unknown serialized stage kind")
                    values = list(struct.unpack("<" + "I" * (4 if kind == 4 else 1),
                                                take(16 if kind == 4 else 4)))
                    stage_ids.add(values[0])
                    stage.append({"kind": kind, "values": values})
                stages.append(stage)
            matches = module_id is None or any(
                s["values"][0] == module_id for stage in stages for s in stage)
            matching_count += matches
            if matches and len(samples) < limit:
                samples.append({"index": index, "live_byte": live,
                                "layout_index": layout_index,
                                "condition_indices": indices,
                                "attributes": attributes, "stages": stages})
        report.update({"conditions": conditions, "variant_count": word_0,
                       "live_variant_count": valid_count, "variant_samples": samples,
                       "matching_variant_count": matching_count,
                       "unique_stage_id_count": len(stage_ids),
                       "stage_id_range": [min(stage_ids), max(stage_ids)] if stage_ids else None,
                       "opaque_payload_offset": position,
                       "opaque_payload_bytes": len(data) - position,
                       "limitations": "Serialized records only; bit0 stage construction, backend remapping and active draw selection are not reproduced"})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path)
    parser.add_argument("--layouts", action="store_true",
                        help="also inspect serialized resource layouts")
    parser.add_argument("--variants", action="store_true",
                        help="also inspect conditions and serialized variant records")
    parser.add_argument("--limit", type=int, default=8,
                        help="maximum live variant samples to display")
    parser.add_argument("--module", type=int,
                        help="only sample variants containing this primary stage ID")
    args = parser.parse_args()
    with args.path.open("rb") as stream:
        data = stream.read(MAX_BYTES + 1)
    try:
        if args.limit < 0:
            raise ValueError("sample limit must be nonnegative")
        if args.module is not None and not args.variants:
            raise ValueError("--module requires --variants")
        report = inspect(data, layouts=args.layouts, variants=args.variants,
                         limit=args.limit, module_id=args.module)
    except ValueError as error:
        parser.error(str(error))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
