import unittest
from shader_variant_index import decode, encode


class SlotTests(unittest.TestCase):
    def test_exhaustive_small_radices(self):
        for slot in range(2 * 2 * 3 * 5):
            prefix, values = decode([3, 5], slot)
            self.assertEqual(encode([3, 5], values, prefix), slot)
        for slot in range(15):
            prefix, values = decode([3, 5], slot, with_prefix=False)
            self.assertEqual(encode([3, 5], values, prefix), slot)

    def test_invalid_inputs(self):
        for ranges, values, prefix in [([0], [0], None), ([2], [2], None),
                                       ([2], [], None), ([], [], [0]), ([], [], [2, 0])]:
            with self.assertRaises(ValueError):
                encode(ranges, values, prefix)
        for slot in [-1, 60]:
            with self.assertRaises(ValueError):
                decode([3, 5], slot)

    def test_unsigned_wrap(self):
        self.assertEqual(encode([65536, 65536, 2], [0, 0, 1]), 0)
