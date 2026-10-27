import struct
import unittest
import numpy as np
from spirv_arithmetic import evaluate


def instruction(opcode, *args):
    return [((len(args) + 1) << 16) | opcode, *args]


def fixture(ops):
    words = [0x07230203, 0x00010600, 0, 100, 0]
    words += instruction(11, 1, *struct.unpack('<4I', b'GLSL.std.450\0\0\0\0'))
    words += instruction(22, 2, 32)
    words += instruction(21, 3, 32, 0)
    words += instruction(43, 3, 4, 0)
    begin = len(words) * 4
    for op in ops:
        words += op
    return struct.pack('<' + 'I' * len(words), *words), begin, len(words) * 4


class ArithmeticTests(unittest.TestCase):
    def test_memory_sample_shuffle_and_mix(self):
        raw, begin, end = fixture([
            instruction(65, 2, 20, 10, 4), instruction(61, 2, 21, 20),
            instruction(87, 2, 22, 11, 21), instruction(79, 2, 23, 22, 22, 2, 0),
            instruction(12, 2, 24, 1, 46, 23, 12, 13)])
        calls = []

        def sample(result, handle, uv):
            calls.append((result, handle, uv.tolist()))
            return [1, 2, 3, 4]

        values, report = evaluate(raw, begin, end,
                                  {11: 'texture', 12: np.array([7, 9], dtype=np.float32),
                                   13: np.float32(.25)},
                                  {10: [np.array([.1, .2], dtype=np.float32)]}, sample)
        np.testing.assert_array_equal(values[24], [4, 3])
        self.assertEqual(calls[0][:2], (22, 'texture'))
        self.assertEqual(report['substituted_samples'], 1)

    def test_unknown_control_flow_and_bad_boundary_fail(self):
        raw, begin, end = fixture([instruction(249, 20)])
        with self.assertRaisesRegex(ValueError, 'unsupported opcode'):
            evaluate(raw, begin, end, {}, {}, None)
        with self.assertRaisesRegex(ValueError, 'boundaries'):
            evaluate(raw, begin + 1, end, {}, {}, None)

    def test_smoothstep_clamps_outside_edges(self):
        raw, begin, end = fixture([instruction(12, 2, 20, 1, 49, 10, 11, 12)])
        for x, expected in [(-1, 0), (.5, .5), (2, 1)]:
            v, _ = evaluate(raw, begin, end,
                            {10: np.float32(0), 11: np.float32(1), 12: np.float32(x)}, {}, None)
            self.assertEqual(float(v[20]), expected)


if __name__ == '__main__':
    unittest.main()
