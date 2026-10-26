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

    def test_truncated_and_zero_size_instructions(self):
        with self.assertRaises(ValueError):
            inspect_module(fixture()[:-1])
        with self.assertRaises(ValueError):
            inspect_module(fixture() + b"\0\0\0\0")
        with self.assertRaises(ValueError):
            inspect_module(fixture() + struct.pack("<I", (4 << 16) | 5))


if __name__ == "__main__":
    unittest.main()
