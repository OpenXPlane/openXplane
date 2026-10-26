import struct
import unittest

from inspect_xsv_mapping import inspect


class MappingPrefixTests(unittest.TestCase):
    def test_lengths_offsets_and_opaque_payload(self):
        names = b"\x01a\x00\x03abc"
        prefix = b"XSV6" + struct.pack("<H", 3) + names
        prefix += struct.pack("<II", 0x12345678, 0x87654321) + b"\x03\x04main"
        report = inspect(prefix + b"\xff\x00")
        self.assertEqual(report["names"], ["a", "", "abc"])
        self.assertEqual(report["names_end_offset"], 13)
        self.assertEqual(report["word_0"], 0x12345678)
        self.assertEqual(report["word_1"], 0x87654321)
        self.assertEqual(report["flags_byte"], 3)
        self.assertEqual(report["entry_string"], "main")
        self.assertEqual(report["opaque_payload_offset"], len(prefix))
        self.assertEqual(report["opaque_payload_bytes"], 2)
        for size in range(len(prefix)):
            with self.subTest(size=size), self.assertRaises(ValueError):
                inspect(prefix[:size])

    def test_signature_and_display_encoding(self):
        with self.assertRaises(ValueError):
            inspect(b"XSV5\x00\x00")
        with self.assertRaises(ValueError):
            inspect(b"XSV6\x01\x00\x01\xff")

    def test_serialized_layouts_and_bad_references(self):
        prefix = b"XSV6\x01\x00\x03tex" + struct.pack("<II", 2, 1) + b"\x03\x00"
        groups = b"\x01\x05\x02\x01\x04\x02\x03"
        resource = b"\x01\x02\x03\x05\x00\x00\x04\x01\x03"
        data = prefix + groups + struct.pack("<I", 1) + b"\x01" + resource
        report = inspect(data + b"opaque", layouts=True)
        self.assertEqual(report["groups"], [{"kind": 5, "fields": [[1, 4], [2, 3]]}])
        self.assertEqual(report["layouts"][0][0], {
            "fields": [1, 2, 3, 5], "name_index": 0, "name": "tex", "tail": [4, 1, 3]})
        self.assertEqual(report["opaque_payload_offset"], len(data))
        for size in range(len(data)):
            with self.subTest(size=size), self.assertRaises(ValueError):
                inspect(data[:size], layouts=True)
        with self.assertRaises(ValueError):
            inspect(data[:-5] + b"\x01\x00" + data[-3:], layouts=True)
        with self.assertRaises(ValueError):
            inspect(prefix + b"\x05", layouts=True)
        with self.assertRaises(ValueError):
            inspect(prefix + b"\x00\xff\xff\xff\xff", layouts=True)


if __name__ == "__main__":
    unittest.main()
