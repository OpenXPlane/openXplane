//! Small input helpers of the reference build, compared with the machine code in an emulator
//! (`tools/gen_input_vectors.py`).

use crate::vm::{CallArgs, Callees, Vm};

/// `0x1417da870(obj, code)`: the index of the first key slot (of the 500 words at `obj+0x8cf8`, enabled by the flag
/// at `+0xba44` from the slot) whose code equals `code`. A slot whose code is already held by an earlier enabled
/// slot counts as code zero; a disabled slot also; 500 when none matches.
pub fn key_slot(vm: &Vm, obj: u64, code: i32) -> u32 {
    let table = obj + 0x8cf8;
    for slot in 0..500u64 {
        let enabled = |k: u64| vm.i32(table + 4 * k + 0xba44) != 0;
        let held = |j: u64| enabled(j) && vm.i32(table + 4 * j) == vm.i32(table + 4 * slot);
        let value = if !enabled(slot) || (0..slot).any(held) {
            0
        } else {
            vm.i32(table + 4 * slot)
        };
        if value == code {
            return slot as u32;
        }
    }
    500
}

/// `0x141185030(obj)`: true when `obj+0x7c9c` is positive, the byte `+0x7d34` is set, the binding `0x4a` (queried with
/// the object at `+0x7b90`) is clear and the word at `[obj+0x20]+0x2270` is below 2.
pub fn bank_available(vm: &Vm, env: &mut dyn Callees, obj: u64) -> bool {
    if vm.i32(obj + 0x7c9c) <= 0 || vm.u8(obj + 0x7d34) == 0 {
        return false;
    }
    let args = CallArgs::ints(&[vm.u64(obj + 0x7b90), 1, 0x4a, 0]);
    if env.call(&mut Vm::default(), 0x1407ace10, args).rax as u32 != 0 {
        return false;
    }
    vm.i32(vm.u64(obj + 0x20) + 0x2270) < 2
}
