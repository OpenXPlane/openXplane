"""Fixture publishing tests using synthetic replay results; no reference executable."""
import hashlib
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import gen_acf_object_vectors as generator


class FixtureContract(unittest.TestCase):
    def test_incomplete_later_replay_preserves_all_existing_fixtures(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            exe = root / 'reference.exe'; exe.write_bytes(b'synthetic test artifact')
            acfs = [root / 'A.acf', root / 'B.acf']
            for acf in acfs: acf.write_text('P test 1\n')
            output = root / 'fixtures'; output.mkdir()
            target = output / 'acf_object_A.txt'; target.write_text('previous verified fixture')
            results = [({('B', 4): 7}, [], None), ({('B', 4): 9}, [], 'instruction budget exhausted')]
            with patch.object(generator, 'REFERENCE_SHA', hashlib.sha256(exe.read_bytes()).hexdigest()), patch.object(generator, 'run', side_effect=results):
                with self.assertRaisesRegex(RuntimeError, 'did not complete'):
                    generator.generate(exe, output, acfs)
            self.assertEqual(target.read_text(), 'previous verified fixture')
            self.assertEqual(list(output.iterdir()), [target])

    def test_complete_replay_publishes_original_return_metadata(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            exe = root / 'reference.exe'; exe.write_bytes(b'synthetic test artifact')
            acf = root / 'A.acf'; acf.write_text('P test 1\n')
            output = root / 'fixtures'
            with patch.object(generator, 'REFERENCE_SHA', hashlib.sha256(exe.read_bytes()).hexdigest()), patch.object(generator, 'run', return_value=({('B', 4): 7}, [], None)):
                generator.generate(exe, output, [acf])
            text = (output / 'acf_object_A.txt').read_text()
            self.assertIn('original return reached', text)
            self.assertIn('B 4=00000007', text)
            self.assertEqual(len(list(output.iterdir())), 1)

    def test_empty_inputs_and_wrong_reference_never_create_output(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root)
            exe = root / 'reference.exe'; exe.write_bytes(b'synthetic test artifact')
            output = root / 'fixtures'
            with self.assertRaisesRegex(ValueError, 'SHA256 mismatch'):
                generator.generate(exe, output, [])
            with patch.object(generator, 'REFERENCE_SHA', hashlib.sha256(exe.read_bytes()).hexdigest()):
                with self.assertRaisesRegex(ValueError, 'No ACF inputs'):
                    generator.generate(exe, output, [])
            self.assertFalse(output.exists())


if __name__ == '__main__': unittest.main()
