"""Synthetic execution-contract tests. No X-Plane executable or original code required."""
import unittest
from unicorn import UC_ARCH_X86, UC_MODE_64, UC_HOOK_CODE, Uc
from unicorn.x86_const import UC_X86_REG_RIP
from emulate_xp import Emulator, RET_ADDR, STACK_TOP, STACK_SIZE


class ExecutionContract(unittest.TestCase):
    def emulator(self, code):
        e = Emulator.__new__(Emulator)
        e.uc = Uc(UC_ARCH_X86, UC_MODE_64)
        e.stubs = {}
        e.uc.mem_map(0x1000, 0x1000)
        e.uc.mem_write(0x1000, code)
        e.uc.mem_map(STACK_TOP - STACK_SIZE, STACK_SIZE)
        e.uc.mem_map(RET_ADDR, 0x1000)
        return e

    def test_function_must_return(self):
        e = self.emulator(b'\xb8\x07\x00\x00\x00\xc3')
        self.assertEqual(e.call(0x1000), 7)
        self.assertEqual(e.uc.reg_read(UC_X86_REG_RIP), RET_ADDR)

    def test_budget_exhaustion_is_not_a_function_result(self):
        e = self.emulator(b'\xeb\xfe')
        with self.assertRaisesRegex(RuntimeError, 'did not reach'):
            e.call(0x1000, max_instructions=20)
        with self.assertRaisesRegex(RuntimeError, 'did not reach'):
            e.run_range(0x1000, 0x1002, max_instructions=20)

    def test_checkpoints_inside_cached_translation_block(self):
        e = self.emulator(b'\xb8\x01\x00\x00\x00\x83\xc0\x02\xc3')
        self.assertEqual(e.call(0x1000), 3)
        self.assertEqual(e.call(0x1000, until=0x1005), 1)
        self.assertEqual(e.uc.reg_read(UC_X86_REG_RIP), 0x1005)
        self.assertEqual(e.call(0x1000, until=0x1008), 3)
        self.assertEqual(e.uc.reg_read(UC_X86_REG_RIP), 0x1008)

    def test_early_external_stop_is_rejected(self):
        e = self.emulator(b'\x90\xc3')
        hook = e.uc.hook_add(UC_HOOK_CODE, lambda uc, address, size, user: uc.emu_stop(), begin=0x1000, end=0x1000)
        with self.assertRaisesRegex(RuntimeError, 'did not reach'):
            e.call(0x1000)
        e.uc.hook_del(hook)
        self.assertEqual(e.call(0x1000), 0)

    def test_zero_budget_cannot_enable_unbounded_execution(self):
        e = self.emulator(b'\xeb\xfe')
        with self.assertRaisesRegex(ValueError, 'positive'):
            e.call(0x1000, max_instructions=0)


if __name__ == '__main__':
    unittest.main()
