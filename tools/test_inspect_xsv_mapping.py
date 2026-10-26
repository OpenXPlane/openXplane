import struct
import unittest

from inspect_xsv_mapping import inspect


class MappingPrefixTests(unittest.TestCase):
    @staticmethod
    def variant_fixture(flags=3, stage=None):
        prefix = b"XSV6\x00\x00" + struct.pack("<II", 3, 1) + bytes([flags, 0])
        layouts = b"\x00" + struct.pack("<I", 1) + b"\x00"
        conditions = b"\x01\x00\x07\x04"
        conditions += b"\x00\x01\x12\x34\x01\x03\x56\x78"
        conditions += b"\x02\x04\x01\x02\x03\x03\x02"
        live = b"\x01" + struct.pack("<I4H", 0, 0, 65535, 65535, 65535)
        if flags & 1:
            live += b"\x01\x03\x00\x02\x04"
        if stage is None:
            stage = b"\x01" + struct.pack("<I", 12) + b"\x02" + struct.pack("<I", 34)
        return prefix + layouts + conditions + b"\x00" + live + stage + b"\x00"

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

    def test_complete_variants_and_filter(self):
        data = self.variant_fixture()
        report = inspect(data, variants=True, module_id=34)
        self.assertEqual(report["opaque_payload_bytes"], 0)
        self.assertEqual(report["variant_count"], 3)
        self.assertEqual(report["live_variant_count"], 1)
        self.assertEqual(report["unique_stage_id_count"], 2)
        self.assertEqual(report["stage_id_range"], [12, 34])
        self.assertEqual(report["variant_samples"][0]["index"], 1)
        self.assertEqual(report["variant_samples"][0]["attributes"], [[3, 0, 2, 4]])
        self.assertEqual(report["conditions"][0]["items"][2]["extra"], [1, 2, 3])
        self.assertEqual(inspect(data, variants=True, module_id=99)["matching_variant_count"], 0)
        self.assertEqual(inspect(data, variants=True, limit=0)["variant_samples"], [])
        for size in range(len(data)):
            with self.subTest(size=size), self.assertRaises(ValueError):
                inspect(data[:size], variants=True)

    def test_compute_stage_and_unknown_stage(self):
        stage = b"\x04" + struct.pack("<4I", 42, 8, 4, 2)
        report = inspect(self.variant_fixture(flags=4, stage=stage), variants=True)
        self.assertEqual(report["variant_samples"][0]["stages"], [
            [{"kind": 4, "values": [42, 8, 4, 2]}]])
        with self.assertRaises(ValueError):
            inspect(self.variant_fixture(flags=4, stage=b"\x03\x00\x00\x00\x00"), variants=True)


if __name__ == "__main__":
    unittest.main()
