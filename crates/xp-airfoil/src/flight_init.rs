//! The initialisation of a flight object (`0x1411ce9c0`), compared with the machine code in an emulator
//! (`tools/gen_init_vectors.py`). Mostly stores of constants and aircraft-derived values; the callees outside this
//! port (the lookups, the wheel/engine resets, the atmosphere accessor, the weight sum) are replayed.

use crate::vm::{CallArgs, Callees, Vm};

const TABLE: u64 = 0x1_4612_43b0;
const INDEX: u64 = 0x1_4265_52ac;
const FLAG_ARG: u64 = 0x1_424f_5648;
const METRES_TO_FEET: f32 = f32::from_bits(0x4051_f948);
const ENGINE: u64 = 0x2cc;
const ENGINE_ITEM: u64 = 0x68;
const PART: u64 = 0x388;
const PART_AERO: u64 = 0x3770;
const GEAR: u64 = 0x88;
const ONE: u32 = 0x3f80_0000;

/// `0x1411ce9c0(ctx, ..., xmm3 = scale)`: `ctx` holds the flight object `F` at `+0` and the aircraft `B` at
/// `+8`; `scale` is the float in `xmm3`, multiplied by the metre-to-feet constant into the two ceiling records
/// `F+0xbd50` and `F+0xbda8`. Errors when an index of the gear animation vector is out of range (the original
/// calls its fatal-error function there).
pub fn flight_init(vm: &mut Vm, env: &mut dyn Callees, ctx: u64, scale: f32) -> Result<(), String> {
    let fo = |vm: &Vm| vm.u64(ctx);
    let bo = |vm: &Vm| vm.u64(ctx + 8);
    let lookup = |vm: &mut Vm, env: &mut dyn Callees| {
        let index = u64::from(vm.u32(INDEX));
        env.call(vm, 0x141a64250, CallArgs::ints(&[TABLE, index]))
            .rax
    };

    let f = fo(vm);
    vm.set_i32(f + 0xdac, 0);
    vm.set_u32(f + 0xdb8, 0x42c8_0000);
    let begin = vm.u64(f + 0x6970);
    if begin != vm.u64(f + 0x6978) {
        vm.set_u64(f + 0x6978, begin);
    }
    let object = vm.u64(f + 0x42f98);
    if object != 0 {
        env.call(vm, 0x14087e590, CallArgs::ints(&[object]));
    }
    let f = fo(vm);
    env.call(vm, 0x1411aedb0, CallArgs::ints(&[f]));
    let f = fo(vm);
    env.call(vm, 0x1411dd2c0, CallArgs::ints(&[f + 0xb580]));
    for (offset, value) in [(0xb0, 0), (0xb4, 2), (0xb8, 2), (0xd8, 0), (0xd4, 0)] {
        let f = fo(vm);
        vm.set_i32(f + offset, value);
    }
    let f = fo(vm);
    if vm.i32(f + 0x28) == 0 {
        let t = lookup(vm, env);
        env.call(vm, 0x14120ad10, CallArgs::ints(&[t + 0x6f90]));
    }

    // per-engine values from the aircraft's first engine slots
    let count = |vm: &Vm| vm.i32(bo(vm) + 0xc9c);
    let mut e = 0;
    while e < count(vm) {
        let value = f64::from(vm.f32(bo(vm) + 0xd84));
        let f = fo(vm);
        vm.set_f64(f + 0xd68 + 8 * e as u64, value);
        e += 1;
    }
    let mut e = 0;
    while e < count(vm) {
        let value = vm.u32(bo(vm) + 0x1b94);
        let f = fo(vm);
        vm.set_u32(f + 0x7a4 + 4 * e as u64, value);
        e += 1;
    }
    let mut e = 0;
    while e < count(vm) {
        let f = fo(vm);
        let off = env.call(vm, 0x1417f12c0, CallArgs::ints(&[FLAG_ARG])).rax as u8 != 0;
        let height = if off { 0.0 } else { vm.f64(f + 0x3a0) };
        let mut args = CallArgs::ints(&[f + 0xbfa8]);
        args.xmm[1] = Some((height as f32).to_bits());
        let t = env.call(vm, 0x141ba6750, args).xmm0 as u32;
        let f = fo(vm);
        vm.set_u32(f + 0x7e4 + 4 * e as u64, t);
        e += 1;
    }

    // the engine state records
    let engines = |vm: &Vm| vm.i32(bo(vm) + 0x91c);
    let parts = |vm: &Vm| vm.i32(bo(vm) + 0x920);
    let mut e = 0;
    while e < engines(vm) {
        let rec = |vm: &Vm| vm.u64(fo(vm) + 0x68b0) + ENGINE * e as u64;
        let slot = |vm: &Vm| vm.u64(bo(vm) + 0x5ff8) + ENGINE_ITEM * e as u64;
        let r = rec(vm);
        vm.set_u32(r + 0x2b8, 0x3f59_999a);
        let r = rec(vm);
        vm.set_u32(r + 0x2bc, ONE);
        for offset in [0x2a8, 0x1e4, 0x238] {
            let r = rec(vm);
            vm.set_i32(r + offset, 1);
        }
        let kind = vm.i32(slot(vm));
        let r = rec(vm);
        vm.set_i32(r + 0x208, i32::from(kind == 7));
        let f = fo(vm);
        env.call(vm, 0x14122cbc0, CallArgs::ints(&[f, e as u64]));
        let r = rec(vm);
        vm.set_i32(r, 0);
        let r = rec(vm);
        vm.set_i32(r + 0x40, 0);
        let r = rec(vm);
        vm.set_u32(r + 0x4c, ONE);
        let b = bo(vm);
        let v = vm.f32(b + 0x9e0) * 100.0;
        let r = rec(vm);
        vm.set_f32(r + 0x88, v);
        let b = bo(vm);
        let item = vm.u64(b + 0x5ff8) + ENGINE_ITEM * e as u64;
        let ratio = vm.f32(item + 0x1c) * vm.f32(b + 0x9e0) / vm.f32(b + 0x950);
        let r = rec(vm);
        vm.set_f32(r + 0xac, (f64::from(ratio) / 101325.0 + 1.0) as f32);
        for (offset, value) in [
            (0x50, 0),
            (0x44, 0x3f00_0000),
            (0x1f8, 0),
            (0x1f4, 0),
            (0x258, ONE),
            (0x214, 0),
        ] {
            let r = rec(vm);
            vm.set_u32(r + offset, value);
        }
        e += 1;
    }
    // the mode of every engine and part slot
    let mut e = 0;
    while e < engines(vm).max(parts(vm)) {
        let b = bo(vm);
        let kind = vm.i32(vm.u64(b + 0x5ff8) + ENGINE_ITEM * e as u64);
        let r = vm.u64(fo(vm) + 0x68b0) + ENGINE * e as u64;
        let level = vm.i32(b + 0xa9c);
        if kind == 3 && level == 2 {
            vm.set_i32(r + 0x298, 0);
            let r = vm.u64(fo(vm) + 0x68b0) + ENGINE * e as u64;
            vm.set_u32(r + 0xbc, 0x4348_0000);
        } else if kind == 4 && level == 3 {
            vm.set_i32(r + 0x298, 2);
            let r = vm.u64(fo(vm) + 0x68b0) + ENGINE * e as u64;
            vm.set_u32(r + 0x280, ONE);
            let r = vm.u64(fo(vm) + 0x68b0) + ENGINE * e as u64;
            vm.set_i32(r + 0x220, 1);
        } else {
            vm.set_i32(r + 0x298, 1);
        }
        e += 1;
    }
    // the part records
    let mut p = 0;
    while p < parts(vm) {
        let b = bo(vm);
        let kind = vm.i32(vm.u64(b + 0x5ff8) + ENGINE_ITEM * p as u64);
        let level = vm.i32(b + 0xa9c);
        let (value, extra) = if kind == 3 && level == 2 {
            (0.0f32, vm.u32(bo(vm) + 0xa54))
        } else if kind == 4 && level == 3 {
            let v = if p >= parts(vm) {
                0.0
            } else {
                let aero = vm.u64(b + 0x6010) + PART_AERO * p as u64;
                vm.f32(b + 0x9f8) / vm.f32(aero + 0x20)
            };
            (v, vm.u32(bo(vm) + 0xa58))
        } else {
            let slots = parts(vm);
            let v = if p >= slots {
                0.0
            } else {
                let engines = vm.i32(b + 0x91c);
                if engines == 0 || slots == 0 {
                    0.0
                } else {
                    let top = engines - 1;
                    let i = if p < 0 { 0 } else { p.min(top) };
                    let item = vm.u64(b + 0x5ff8) + ENGINE_ITEM * i as u64;
                    let aero = vm.u64(b + 0x6010) + PART_AERO * p as u64;
                    vm.f32(item + 0x18) / vm.f32(aero + 0x20)
                }
            };
            let aero = vm.u64(bo(vm) + 0x6010) + PART_AERO * p as u64;
            (v, vm.u32(aero + 0x24))
        };
        let rec = vm.u64(fo(vm) + 0x68c8) + PART * p as u64;
        vm.set_f32(rec, value);
        let rec = vm.u64(fo(vm) + 0x68c8) + PART * p as u64;
        vm.set_u32(rec + 8, extra);
        p += 1;
    }

    let f = fo(vm);
    env.call(vm, 0x14121a690, CallArgs::ints(&[f + 0xbb30, 0]));
    let value = vm.u32(bo(vm) + 0x2924);
    let f = fo(vm);
    vm.set_u32(f + 0x6534, value);
    let f = fo(vm);
    env.call(vm, 0x1411ddfd0, CallArgs::ints(&[f, f + 0x288]));
    let f = fo(vm);
    let x = vm.u64(f + 0xbce8);
    vm.set_i32(f + 0xbcf0, 1);
    let mut ready = 1;
    if vm.i32(x + 0x1ca4) != 0 {
        vm.set_i32(f + 0xbcf0, 0);
        ready = 0;
    }
    if vm.i32(x + 0x1ca8) != 0 {
        vm.set_i32(f + 0xbcf0, 0);
        ready = 0;
    }
    vm.set_i32(f + 0xbcf4, 1);
    vm.set_f32(f + 0xbcf8, ready as f32);
    if vm.f32(x + 0xc04) == 0.0 {
        vm.set_u32(x + 0xc04, 0x41f0_0000);
    }

    // the controls object at F+0xbd00
    let g = fo(vm) + 0xbd00;
    let b = vm.u64(g + 8);
    vm.set_i32(g + 0x14, 1);
    vm.set_i32(g + 0x10, 1);
    vm.set_u64(g + 0x18, 0);
    vm.set_i32(g + 0x20, 0);
    let slow = |vm: &Vm, b: u64| {
        let part = vm.u64(b + 0x6010);
        vm.i32(b + 0x91c) == 1 && 1.0 > vm.f32(part + 0x790).abs()
    };
    if slow(vm, b) {
        let r = vm.u64(vm.u64(g) + 0x68b0);
        vm.set_i32(r + 0x2a8, 0);
    }
    let b = vm.u64(g + 8);
    if vm.i32(b + 0x1cf8) == 0 {
        vm.set_i32(g + 0x1c, 1);
        vm.set_i32(g + 0x18, 1);
        vm.set_i32(g + 0x20, 1);
        if slow(vm, b) {
            let r = vm.u64(vm.u64(g) + 0x68b0);
            vm.set_i32(r + 0x2a8, 1);
        }
    }

    // the ceiling records
    let h = fo(vm) + 0xbd50;
    let owner = vm.u64(h);
    if vm.i32(owner + 0x28) != 0 || vm.i32(owner + 0x68a0) == 0 {
        let b = vm.u64(h + 8);
        let feet = scale * METRES_TO_FEET;
        vm.set_u64(h + 0x10, 0);
        vm.set_i32(h + 0x18, 0);
        vm.set_u32(h + 0x1c, 0x45ea_6000);
        vm.set_f32(h + 0x24, feet);
        vm.set_u32(h + 0x20, 0x43fa_0000);
        vm.set_u64(h + 0x28, 0);
        vm.set_i32(h + 0x30, 0);
        vm.set_u64(h + 0x38, 0);
        vm.set_u64(h + 0x4c, 1);
        vm.set_u64(h + 0x44, 0);
        vm.set_i32(h + 0x40, 0);
        if vm.f32(b + 0xea8) > 0.0 {
            let v = crate::scalar::clamp(feet, 0.0, 7500.0);
            vm.set_f32(h + 0x24, v);
        }
    }
    let k = fo(vm) + 0xbda8;
    let owner = vm.u64(k);
    if vm.i32(owner + 0x28) != 0 || vm.i32(owner + 0x68a4) == 0 {
        let b = vm.u64(k + 8);
        let feet = scale * METRES_TO_FEET;
        let v = vm.u32(b + 0xeb8);
        vm.set_f32(k + 0x14, feet);
        vm.set_u32(k + 0x10, v);
        vm.set_u64(k + 0x18, 2);
        vm.set_i32(k + 0x20, 2);
        vm.set_u64(k + 0x24, 0x41a0_0000);
    }

    // the switch object at F+0xbdd8
    let m = fo(vm) + 0xbdd8;
    vm.set_u64(m + 0x60, 0);
    vm.set_i32(m + 0x68, 0);
    let b = vm.u64(m + 8);
    let locked = vm.i32(b + 0x1cb0) != 0;
    vm.set_i32(m + 0x10, i32::from(vm.i32(b + 0xdc4) != 0 && !locked));
    vm.set_i32(m + 0x14, i32::from(vm.i32(b + 0xdc8) != 0 && !locked));
    vm.set_i32(m + 0x18, i32::from(vm.i32(b + 0xdcc) != 0 && !locked));
    vm.set_i32(m + 0x1c, 0);
    vm.set_u64(m + 0x20, 1);
    for offset in [0x50, 0x54, 0x58, 0x5c] {
        vm.set_u32(m + offset, 0x3f59_999a);
    }
    for k in 0..10 {
        vm.set_u32(m + 0x28 + 4 * k, ONE);
    }
    let mb = |vm: &Vm| vm.u64(m + 8);
    if vm.i32(b + 0x91c) > 0 {
        let mut k = 0;
        loop {
            for offset in [0x29c, 0x2a0, 0x2a4] {
                let r = vm.u64(vm.u64(m) + 0x68b0) + ENGINE * k as u64;
                vm.set_i32(r + offset, 1);
            }
            k += 1;
            if k >= vm.i32(mb(vm) + 0x91c) {
                break;
            }
        }
    }
    if vm.i32(mb(vm) + 0x920) > 0 {
        let mut k = 0;
        loop {
            for offset in [0x8c, 0x90, 0x94] {
                let r = vm.u64(vm.u64(m) + 0x68c8) + PART * k as u64;
                vm.set_i32(r + offset, 1);
            }
            k += 1;
            if k >= vm.i32(mb(vm) + 0x920) {
                break;
            }
        }
    }

    // flags and constants
    let f = fo(vm);
    vm.set_u64(f + 0x6f4, 0);
    vm.set_i32(f + 0x6fc, 0);
    vm.set_i32(f + 0xb8b0, -1);
    vm.set_i32(f + 0x659c, 1);
    if vm.i32(bo(vm) + 0x914) == 0 {
        vm.set_i32(f + 0x55c, 1);
    }
    for (offset, value) in [
        (0x64c4, 0),
        (0x64c8, 0),
        (0x57c, 1),
        (0x570, 1),
        (0x7c, 1),
        (0x80, 1),
    ] {
        vm.set_i32(f + offset, value);
    }
    let b = bo(vm);
    let moving = vm.f32(b + 0x22b0).abs() > 0.0 || vm.f32(b + 0x22cc).abs() > 0.0;
    vm.set_i32(f + 0x84, i32::from(moving));
    for (offset, value) in [
        (0x651c, 0),
        (0x653c, ONE),
        (0x6520, 0),
        (0x6540, ONE),
        (0x6524, 0),
        (0x6504, 0),
        (0x6454, 0x3f40_0000),
        (0x6458, 0x3f40_0000),
        (0x64e4, 0),
        (0x580, 1),
    ] {
        vm.set_u32(f + offset, value);
    }
    let b = bo(vm);
    if vm.i32(vm.u64(b + 0x5ff8)) != 4 || vm.i32(b + 0xa9c) != 3 {
        vm.set_i32(f + 0x6440, 1);
    }
    vm.set_u32(f + 0x598, ONE);
    vm.set_i32(f + 0x584, 0);
    vm.set_i32(f + 0x588, i32::from(vm.i32(b + 0xae0) == 0));
    for offset in [0x28f4, 0x28f8, 0x28fc] {
        vm.set_i32(b + offset, 0);
    }
    for (offset, value) in [
        (0x5ac, 1),
        (0x5f0, 1),
        (0x5b0, 1),
        (0x5f4, 1),
        (0x5b4, 1),
        (0x5b4, 2),
        (0x5f8, 1),
        (0x5b8, 1),
        (0x5b8, 2),
        (0x5fc, 1),
        (0x5bc, 1),
        (0x600, 1),
        (0x5c0, 1),
        (0x604, 1),
        (0x5c4, 1),
        (0x608, 1),
        (0x5c8, 1),
        (0x60c, 1),
        (0x5cc, 1),
        (0x610, 1),
        (0x5d0, 1),
        (0x614, 1),
        (0x5d4, 1),
        (0x618, 1),
        (0x5d8, 1),
        (0x61c, 1),
        (0x5dc, 1),
        (0x620, 1),
        (0x5e0, 1),
        (0x624, 1),
        (0x5e4, 1),
        (0x628, 1),
        (0x5e8, 1),
        (0x62c, 1),
        (0x5ec, 1),
        (0x630, 1),
    ] {
        vm.set_i32(f + offset, value);
    }

    // the primary object only
    let f = fo(vm);
    if vm.i32(f + 0x28) == 0 {
        let handle = vm.u64(bo(vm) + 0x1d18);
        let mut present = false;
        if handle != 0 {
            let r = env.call(vm, 0x14082be10, CallArgs::ints(&[handle, 2]));
            present = r.rax != 0;
        }
        let words: [(u64, u32); 8] = [
            (0xaf38, 1),
            (0xaf30, 0),
            (0xaf40, 2),
            (0xb230, if present { 4 } else { 0 }),
            (0xb234, if present { 4 } else { 1 }),
            (0xb084, 0xc47a_0000),
            (0xb088, 0xc47a_0000),
            (0xb09c, 0xbf80_0000),
        ];
        for (offset, value) in words {
            vm.set_u32(f + offset, value);
        }
        for offset in [0xb0a0, 0xb0a4, 0xb0a8, 0xb0b0, 0xb0ac] {
            vm.set_u32(f + offset, 0xbf80_0000);
        }
        for offset in [0xb0c0, 0xb0bc, 0xb0c8, 0xb0c4] {
            vm.set_u32(f + offset, 0x4650);
        }
        vm.set_u64(f + 0xb0d4, u64::MAX);
        vm.set_u32(f + 0x8ddc, 0x4273_d70a);
        vm.set_u32(f + 0x8de0, 0x4273_d70a);
        let zero16 = |vm: &mut Vm, at: u64| {
            vm.set_u64(at, 0);
            vm.set_u64(at + 8, 0);
        };
        vm.set_u64(f + 0xae88, 0);
        vm.set_u64(f + 0xb078, 0);
        vm.set_u32(f + 0xb080, 0);
        for at in [0xafd4, 0xafe4, 0xaff4, 0xb004] {
            zero16(vm, f + at);
        }
        for at in [0xb054, 0xb05c, 0xb068] {
            vm.set_u64(f + at, 0);
        }
        for at in [0xb014, 0xb024, 0xb034, 0xb044] {
            zero16(vm, f + at);
        }
        vm.set_u64(f + 0xb070, 0);
        for offset in [0xaf9c, 0xafa0, 0xafa4] {
            vm.set_u32(f + offset, ONE);
        }
        vm.set_u64(f + 0xafa8, 0x3f80_0000);
        vm.set_u64(f + 0xaf64, 0);
        vm.set_u32(f + 0xaf6c, 0xc47a_0000);
        vm.set_u32(f + 0xaf70, 0xc47a_0000);
        vm.set_u64(f + 0xaf84, 0);
        vm.set_u32(f + 0xaf74, 1);
        vm.set_u64(f + 0xaf78, 1);
        vm.set_u64(f + 0xaf94, 0);
        vm.set_u64(f + 0xaf8c, 0);
        vm.set_u64(f + 0xafb0, 0);
        vm.set_u32(f + 0xafb8, 0x4248_0000);
        vm.set_u32(f + 0xafbc, 0x4000_0000);
        vm.set_u32(f + 0xafc0, 0x4248_0000);
        let f = fo(vm);
        for (offset, value) in [
            (0x6f60, 0),
            (0x6f5c, 0),
            (0x6f64, 0),
            (0x6de0, 1),
            (0x6f18, 0),
            (0x6f0c, 1),
            (0x6f10, 0),
        ] {
            vm.set_i32(f + offset, value);
        }
        let t = lookup(vm, env);
        let reset = t + 0x6f90;
        for (r8, r9) in [(1, 0), (0, 1), (0, 2)] {
            env.call(vm, 0x1411b0420, CallArgs::ints(&[reset, 0, r8, r9]));
        }
        let owner = vm.u64(reset + 0xc10);
        if owner != 0 {
            let method = vm.u64(vm.u64(owner) + 8);
            env.call(vm, method, CallArgs::ints(&[owner]));
        }
        let t = lookup(vm, env);
        vm.set_u32(t + 0x7c48, 0xc479_c000);
        let t = lookup(vm, env);
        vm.set_u32(t + 0x7c5c, 0xc479_c000);
        let t = lookup(vm, env);
        vm.set_i32(t + 0x7018, 6);
        vm.set_i32(t + 0x73d0, 6);
        let t = lookup(vm, env);
        if vm.i32(t + 0x7c68) != 0 {
            let t = lookup(vm, env);
            vm.set_i32(t + 0x7c68, 0);
            let t = lookup(vm, env);
            vm.set_i32(t + 0x7bbc, 0);
        }
    }

    // gear presence
    let f = fo(vm);
    vm.set_u32(f + 0x210, ONE);
    for i in 0..10u64 {
        let base = vm.u64(bo(vm) + 0x6080) + GEAR * i;
        if vm.i32(base) != 0 && vm.i32(base + 0xc) != 0 {
            let f = fo(vm);
            vm.set_i32(f + 0x210, 0);
        }
    }
    let f = fo(vm);
    vm.set_i32(f + 0x574, 1);
    vm.set_u32(f + 0x254, 0x4479_c000);
    for (offset, value) in [
        (0x240, 0),
        (0x244, 0),
        (0x200, 0),
        (0x204, 1),
        (0x214, 0),
        (0x218, 0),
    ] {
        vm.set_i32(f + offset, value);
    }
    let vtable = vm.u64(f);
    let method = vm.u64(vtable);
    env.call(vm, method, CallArgs::ints(&[f, 0x25b]));

    // the gear animation entries (`[F+0x6958]`, 0x90 bytes each) from the gear records
    let entry = |vm: &Vm, i: u64| -> Result<u64, String> {
        let f = fo(vm);
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        if count as u64 <= i {
            return Err("gear animation index out of range".into());
        }
        Ok(begin + 0x90 * i)
    };
    for i in 0..10u64 {
        let target = entry(vm, i)?;
        let found = |vm: &Vm| {
            let g = vm.u64(bo(vm) + 0x6080);
            (0..10u64).any(|j| vm.i32(g + GEAR * j) > 0 && vm.i32(g + GEAR * j + 4) != 0)
        };
        let g = |vm: &Vm| vm.u64(bo(vm) + 0x6080) + GEAR * i;
        let first = if found(vm) && vm.i32(fo(vm) + 0x1f0) != 0 {
            1.0f32
        } else if found(vm) && vm.i32(g(vm) + 4) != 0 {
            0.0
        } else {
            1.0
        };
        vm.set_f32(target + 0x10, first);
        for (store, lit, plain) in [(0x14, 0x6c, 0x68), (0x18, 0x78, 0x74), (0x1c, 0x84, 0x80)] {
            let use_lit = found(vm) && vm.i32(fo(vm) + 0x1f0) == 0 && vm.i32(g(vm) + 4) != 0;
            let v = vm.f32(g(vm) + if use_lit { lit } else { plain });
            let target = entry(vm, i)?;
            vm.set_f32(target + store, v);
        }
    }
    let f = fo(vm);
    vm.set_i32(f + 0x140, 0);
    vm.set_i32(f + 0x180, 0);
    let b = bo(vm);
    let index = i64::from(vm.i32(b + 0x1eb8));
    if index != 0 {
        let at = |o: u64| (b + o).wrapping_add((4 * index) as u64);
        let v = (vm.f32(at(0x1f88)) + vm.f32(at(0x1f80))) * 0.5;
        let f = fo(vm);
        vm.set_f32(f + 0x18c, v);
    }
    let f = fo(vm);
    vm.set_u32(f + 0x1d8, 0x3f00_0000);
    vm.set_i32(f + 0xf0, 0);
    vm.set_i32(f + 0xfc, 0);
    let v = vm.u32(vm.u64(ctx + 8) + 0x204c);
    let f = fo(vm);
    vm.set_u32(f + 0x1d0, v);
    for offset in [0xe8, 0xf4, 0x1cc, 0xec, 0xf8, 0x1d4] {
        let f = fo(vm);
        vm.set_i32(f + offset, 0);
    }
    Ok(())
}
