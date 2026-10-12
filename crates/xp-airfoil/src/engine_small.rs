//! Small engine and input helpers of the flight step, compared with the machine code in an emulator
//! (`tools/gen_small_vectors.py`).

use crate::vm::Vm;

/// `0x1411dd610(F, n)`: the throttle ceiling of engine `n`: the engine's lever setting (`[F+0x68b0] + 0x2cc n +
/// 0x4c`, forced to 1 while `B+0xa98` is set and `F+0x24c` is clear) interpolated between `B+0x9d8` and
/// `B+0x9dc`, never below the smaller of the two (and the result stays within them).
pub fn throttle_ceiling(vm: &Vm, f: u64, n: i32) -> f32 {
    let b = vm.u64(f + 0x20);
    let mut lever = vm.f32(
        vm.u64(f + 0x68b0)
            .wrapping_add((i64::from(n) * 0x2cc) as u64)
            + 0x4c,
    );
    if vm.i32(b + 0xa98) != 0 && vm.i32(f + 0x24c) == 0 {
        lever = 1.0;
    }
    let (lo, hi) = (vm.f32(b + 0x9d8), vm.f32(b + 0x9dc));
    let lever = lever - 0.0;
    let value = (hi - lo) * lever + lo;
    let smaller = if lo < hi { lo } else { hi };
    if smaller > value {
        return smaller;
    }
    let upper = if lo > hi { lo } else { hi };
    if upper < value { upper } else { value }
}

/// `0x1411b0a30(B, n)`: the lever term of engine `n` by its kind (`[B+0x5ff8] + 0x68 n`): kind 7 `B+0xb74 *
/// B+0x928`, kinds 5 and 6 `+0x24 * B+0xb70 + +0x1c * B+0xb64`, kinds 1 and 2 `+4 * B+0xb08`, kinds 3 and 4 `+8 *
/// B+0xb24`, else zero.
pub fn engine_lever(vm: &Vm, b: u64, n: i32) -> f32 {
    let record = vm
        .u64(b + 0x5ff8)
        .wrapping_add((i64::from(n) * 0x68) as u64);
    let kind = vm.u32(record);
    match kind {
        7 => vm.f32(b + 0xb74) * vm.f32(b + 0x928),
        5 | 6 => {
            vm.f32(record + 0x24) * vm.f32(b + 0xb70) + vm.f32(record + 0x1c) * vm.f32(b + 0xb64)
        }
        1 | 2 => vm.f32(record + 4) * vm.f32(b + 0xb08),
        3 | 4 => vm.f32(record + 8) * vm.f32(b + 0xb24),
        _ => 0.0,
    }
}

/// `0x1411d9d80(F)`: whether the controls count as touched: true unless `F+0x3c == 1`; with it, true when an
/// enabled binding slot (the table at `0x1460e9708`, enable words `0xba44` bytes later) holds the code 3, 0x46,
/// 0x4a or 0x4b, when one of the input globals `0x1460e0a1c`, `0x1460e0a68`, `0x1460e0a6c` is set, or, with
/// `F+0x28 == 0`, when `F+0x67cc` or `F+0x67fc` is set.
pub fn pilot_active(vm: &Vm, f: u64) -> bool {
    if vm.i32(f + 0x3c) != 1 {
        return true;
    }
    const TABLE: u64 = 0x1_460e_9708;
    for code in [3, 0x46, 0x4a, 0x4b] {
        for slot in 0..500u64 {
            let at = TABLE + 4 * slot;
            if vm.i32(at + 0xba44) != 0 && vm.i32(at) == code {
                return true;
            }
        }
    }
    if vm.i32(0x1_460e_0a1c) != 0 || vm.i32(0x1_460e_0a68) != 0 || vm.i32(0x1_460e_0a6c) != 0 {
        return true;
    }
    vm.i32(f + 0x28) == 0 && (vm.i32(f + 0x67cc) != 0 || vm.i32(f + 0x67fc) != 0)
}
