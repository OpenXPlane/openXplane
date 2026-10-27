import struct
import tempfile
import unittest
from pathlib import Path
from zipfile import ZipFile

from inspect_spirv_archive import inspect_archive, inspect_module


def fixture():
    words = [0x07230203, 0x00010600, 0, 10, 0]
    words += [(5 << 16) | 5, 2, *struct.unpack("<3I", b"tex_comp\0\0\0\0")]
    words += [(4 << 16) | 71, 2, 33, 7]
    words += [(4 << 16) | 71, 2, 34, 1]
    words += [(5 << 16) | 15, 4, 3, *struct.unpack("<2I", b"main\0\0\0\0")]
    return struct.pack(f"<{len(words)}I", *words)


class ShaderInspectionTests(unittest.TestCase):
    def test_entry_binding_and_archive_selection(self):
        module = inspect_module(fixture())
        self.assertEqual(module["entry_points"][0]["execution_model"], 4)
        self.assertEqual(module["named_ids"], [{"id": 2, "name": "tex_comp", "binding": 7, "set": 1}])
        with tempfile.TemporaryDirectory(dir=Path(__file__).resolve().parent) as directory:
            path = Path(directory) / "synthetic.zip"
            with ZipFile(path, "w") as archive:
                archive.writestr("test.spv", fixture())
            report = inspect_archive(path, "tex_comp", 1)
            self.assertEqual(report["candidate_count"], 1)
            self.assertEqual(report["candidates"][0]["member"], "test.spv")
            self.assertEqual(inspect_archive(path, "missing", 1)["candidate_count"], 0)
            self.assertEqual(inspect_archive(path, "missing", 1, "test.spv")["candidate_count"], 1)
            self.assertEqual(inspect_archive(path, "tex_comp", 1, "absent.spv")["candidate_count"], 0)

    def test_sampled_image_handle_chain_and_unresolved(self):
        raw = fixture()
        words = []
        for opcode, args in [(61, [1, 4, 2]), (83, [1, 5, 4]),
                             (86, [1, 6, 5, 9]), (87, [1, 7, 6, 8])]:
            words += [((len(args) + 1) << 16) | opcode, *args]
        report = inspect_module(raw + struct.pack('<' + 'I' * len(words), *words))
        sample = report["image_samples"][0]
        self.assertEqual(sample["result_id"], 7)
        self.assertEqual(sample["coordinate_id"], 8)
        self.assertEqual(sample["named_handle_roots"], report["named_ids"])
        self.assertEqual(sample["unresolved_handle_ids"], [9])
        # A malformed cyclic handle graph must terminate and stay unresolved.
        cycle = struct.pack('<4I', (4 << 16) | 83, 1, 4, 4)
        sample = inspect_module(raw + struct.pack('<' + 'I' * len(words), *words) + cycle)["image_samples"][0]
        self.assertEqual(sample["unresolved_handle_ids"], [4, 9])
        with self.assertRaises(ValueError):
            inspect_module(raw + struct.pack('<3I', (3 << 16) | 87, 1, 2))

    def test_truncated_and_zero_size_instructions(self):
        with self.assertRaises(ValueError):
            inspect_module(fixture()[:-1])
        with self.assertRaises(ValueError):
            inspect_module(fixture() + b"\0\0\0\0")
        with self.assertRaises(ValueError):
            inspect_module(fixture() + struct.pack("<I", (4 << 16) | 5))


if __name__ == "__main__":
    unittest.main()
