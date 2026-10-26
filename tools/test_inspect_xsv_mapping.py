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


if __name__ == "__main__":
    unittest.main()
