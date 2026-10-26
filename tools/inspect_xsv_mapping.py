"""Inspect the recovered XSV6 prefix; the remaining variant payload is opaque."""
import argparse
import hashlib
import json
from pathlib import Path
import struct

MAX_BYTES = 64 * 1024 * 1024


def inspect(data):
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
    return {
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("path", type=Path)
    args = parser.parse_args()
    with args.path.open("rb") as stream:
        data = stream.read(MAX_BYTES + 1)
    try:
        report = inspect(data)
    except ValueError as error:
        parser.error(str(error))
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
