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
    let lookup = |vm: &mut Vm, _env: &mut dyn Callees| flight_object(vm, vm.i32(INDEX));

    let f = fo(vm);
    vm.set_i32(f + 0xdac, 0);
    vm.set_u32(f + 0xdb8, 0x42c8_0000);
    let begin = vm.u64(f + 0x6970);
    if begin != vm.u64(f + 0x6978) {
        vm.set_u64(f + 0x6978, begin);
    }
    let object = vm.u64(f + 0x42f98);
    if object != 0 {
        clear_arrays(vm, object);
    }
    let f = fo(vm);
    reset_flight_object(vm, f)?;
    let f = fo(vm);
    reset_balance_state(vm, env, f + 0xb580);
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
        throttle_step(vm, env, f, e, 0.0, 0.0);
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
    reset_flags(vm, env, f + 0xbb30, 0);
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
        let present = handle != 0 && handle_slot(vm, handle, 2)? != 0;
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

/// `0x141a64250(table, index)`: the flight object `index`, at `[table] + index * 0x433c0` (the table of flight
/// objects is at `0x1461243b0`).
pub fn flight_object(vm: &Vm, index: i32) -> u64 {
    vm.u64(TABLE)
        .wrapping_add((i64::from(index) * 0x433c0) as u64)
}

/// `0x14082be10(handle, index)`: the pointer in slot `index` (`0..2`) at `handle + 0xe8`; a larger index is the
/// original's fatal-error call (an `Err` here).
pub fn handle_slot(vm: &Vm, handle: u64, index: i32) -> Result<u64, String> {
    if index >= 3 {
        return Err("handle slot index out of range".into());
    }
    Ok(vm.u64(handle.wrapping_add((0xe8 + 8 * i64::from(index)) as u64)))
}

/// `0x14087e590(object)`: zeroes the three float arrays `[object+0x10]`, `[object+0x18]`, `[object+0x20]` for as
/// many entries as the vector `[[object]+0x58] .. [[object]+0x60]` of 0x180-byte elements holds (the original's
/// vectorised `memset` calls and its scalar tail amount to the same words).
pub fn clear_arrays(vm: &mut Vm, object: u64) {
    let owner = vm.u64(object);
    let span = vm.u64(owner + 0x60).wrapping_sub(vm.u64(owner + 0x58)) as i64;
    let count = (span / 0x180) as i32;
    for i in 0..i64::from(count.max(0)) {
        for array in [0x10, 0x18, 0x20] {
            let base = vm.u64(object + array);
            vm.set_u32(base.wrapping_add((4 * i) as u64), 0);
        }
    }
}

/// `0x14122cbc0(F, engine, a, b)`: the throttle state `E+0` / the lever position `E+0x280` of an engine record
/// (`[F+0x68b0] + 0x2cc engine`) stepped by the input `a` and the delta `b`. With both zero both are cleared; modes
/// 2 and 3 of the record (`+0x298`) with `B+0xa90` set go on to `0x14122c470` (replayed); mode 2 integrates the
/// lever `E+0x280 += b` held to `0..1`; otherwise `E+0 = max(0, min(x, 1))` of `a + b`, where with `B+0xae4 == 3`
/// and `a > 0.05` the upper end is the held `0.0505` rather than `x` when `x < 0.0505`.
pub fn throttle_step(vm: &mut Vm, env: &mut dyn Callees, f: u64, engine: i32, a: f32, b: f32) {
    let record = |vm: &Vm| {
        vm.u64(f + 0x68b0)
            .wrapping_add((i64::from(engine) * 0x2cc) as u64)
    };
    if a == 0.0 && b == 0.0 {
        let r = record(vm);
        vm.set_u32(r + 0x280, 0);
        vm.set_u32(r, 0);
        return;
    }
    let aircraft = vm.u64(f + 0x20);
    if vm.i32(aircraft + 0xa90) != 0 {
        let mode = vm.i32(record(vm) + 0x298);
        if mode == 2 || mode == 3 {
            let mut args = CallArgs::ints(&[f, engine as u32 as u64]);
            args.xmm[2] = Some(a.to_bits());
            args.xmm[3] = Some(b.to_bits());
            env.call(vm, 0x14122c470, args);
            return;
        }
    }
    let r = record(vm);
    if vm.i32(r + 0x298) == 2 {
        let x = b + vm.f32(r + 0x280);
        vm.set_f32(r + 0x280, crate::scalar::clamp(x, 0.0, 1.0));
        return;
    }
    let x = a + b;
    let top = if vm.i32(aircraft + 0xae4) == 3 && a > 0.05 && f32::from_bits(0x3d4e_d917) > x {
        f32::from_bits(0x3d4e_d917)
    } else if 1.0 < x {
        1.0
    } else {
        x
    };
    let v = if 0.0 > top { 0.0 } else { top };
    vm.set_f32(r, v);
}

/// `0x1411dd2c0(this)`: resets the weight-and-balance state object at `F+0xb580` (`[this]` is `F`, `[this+8]`
/// is `B`): the counters, the per-engine and per-part arrays (`B+0x91c`, `B+0x920` entries), the engine records'
/// `+0x2b4`, the entries of the vector `[B+0x60e0] .. [B+0x60e8]` (0x120 bytes; their pointer at `+0x118` is
/// released by `0x140b0c6d0`, replayed) and the two values of `B+0x225c` and `B+0x2260`.
pub fn reset_balance_state(vm: &mut Vm, env: &mut dyn Callees, this: u64) {
    vm.set_u64(this + 0x10, 1);
    for (offset, wide) in [
        (0x18, true),
        (0x20, true),
        (0x28, true),
        (0x38, false),
        (0x44, true),
        (0x50, true),
        (0x30, true),
        (0x3c, false),
        (0x40, false),
        (0x4c, false),
        (0x58, true),
        (0x60, true),
        (0x2cc, true),
        (0x8c, true),
        (0x68, true),
        (0x94, true),
        (0x9c, true),
        (0x2d4, true),
        (0xc4, true),
        (0xa4, true),
        (0x70, true),
        (0xac, true),
        (0x2a8, true),
        (0x78, false),
    ] {
        match (offset, wide) {
            (0x38 | 0x3c | 0x40, _) => vm.set_u32(this + offset, 1),
            (_, true) => vm.set_u64(this + offset, 0),
            (_, false) => vm.set_u32(this + offset, 0),
        }
    }
    let b = vm.u64(this + 8);
    vm.set_u8(this + 0x2bd, 0);
    vm.set_u32(this + 0x2c0, 0);
    if vm.i32(b + 0x91c) > 0 {
        let mut k = 0u64;
        loop {
            vm.set_u32(this + 0xcc + 4 * k, 0);
            vm.set_u32(this + 0x10c + 4 * k, 0);
            vm.set_u32(this + 0x2dc + 4 * k, 0);
            k += 1;
            if k as i64 >= i64::from(vm.i32(vm.u64(this + 8) + 0x91c)) {
                break;
            }
        }
    }
    if vm.i32(b + 0x920) > 0 {
        let mut k = 0u64;
        loop {
            vm.set_u32(this + 0x14c + 4 * k, 0);
            vm.set_u32(this + 0x18c + 4 * k, 0);
            k += 1;
            if k as i64 >= i64::from(vm.i32(vm.u64(this + 8) + 0x920)) {
                break;
            }
        }
    }
    for offset in [
        0x1cc, 0x1d4, 0x1dc, 0x1e4, 0x1f4, 0x200, 0x20c, 0x1ec, 0x214, 0x21c,
    ] {
        vm.set_u64(this + offset, 0);
    }
    vm.set_u32(this + 0x1fc, 0);
    vm.set_u32(this + 0x208, 0);
    vm.set_u32(this + 0x224, 0);
    if vm.i32(b + 0x91c) > 0 {
        let mut k = 0u64;
        loop {
            vm.set_u32(this + 0x228 + 4 * k, 0);
            let r = vm.u64(vm.u64(this) + 0x68b0);
            vm.set_u32(r + 0x2cc * k + 0x2b4, 0);
            k += 1;
            if k as i64 >= i64::from(vm.i32(vm.u64(this + 8) + 0x91c)) {
                break;
            }
        }
    }
    if vm.i32(b + 0x920) > 0 {
        let mut k = 0u64;
        loop {
            vm.set_u32(this + 0x268 + 4 * k, 0);
            k += 1;
            if k as i64 >= i64::from(vm.i32(vm.u64(this + 8) + 0x920)) {
                break;
            }
        }
    }
    let (end, mut entry) = (vm.u64(b + 0x60e8), vm.u64(b + 0x60e0));
    if entry != end {
        loop {
            let pointer = vm.u64(entry + 0x118);
            if pointer != 0 {
                env.call(vm, 0x140b0c6d0, CallArgs::ints(&[pointer]));
            }
            entry = entry.wrapping_add(0x120);
            if entry == end {
                break;
            }
        }
    }
    let b = vm.u64(this + 8);
    for offset in [0x7c, 0xb4, 0x84, 0xbc, 0x2b0] {
        vm.set_u64(this + offset, 0);
    }
    let v = vm.u32(b + 0x225c);
    vm.set_u32(this + 0x2b8, v);
    vm.set_u8(this + 0x2bc, u8::from(vm.f32(b + 0x225c) >= 30.0));
    let w = vm.u32(b + 0x2260);
    vm.set_u32(this + 0x2c8, 0);
    vm.set_u32(this + 0x2c4, w);
}

/// `0x14121a690(this, flag)`: resets the control-state object at `F+0xbb30`: the sum `+0x54` of the nine
/// tank heights (`0x141a6ba30`, `mass::tank_position`) times `B+0x3f7c..`, the nine ones at `+0x60..0x80`, the modes `+0x168/+0x16c`
/// (1 or 3 by the input binding queries `0x14123d190`, replayed, the slots' signs `+0x4c` and `B+0x1cb4`,
/// `B+0x1cc0/0x1cc4` and the six words `B+0x1cc8..`), the flag `+0x17c/+0x180` (`flag` unless `B+0x28a4 > 0.1`),
/// the sum `+0x160` of `+0x84..+0xa4` and the engine records' `+0x1d4`.
pub fn reset_flags(vm: &mut Vm, _env: &mut dyn Callees, this: u64, flag: u32) {
    vm.set_u32(this + 0x54, 0);
    let mut b = vm.u64(this + 8);
    for i in 0..9u64 {
        let z = crate::mass::tank_position(vm, b, i as i32)[2];
        b = vm.u64(this + 8);
        let product = z * vm.f32(b + 0x3f7c + 4 * i);
        let sum = product + vm.f32(this + 0x54);
        vm.set_f32(this + 0x54, sum);
    }
    vm.set_u32(this + 0x178, 0);
    vm.set_u64(this + 0x190, 0);
    vm.set_u32(this + 0x15c, 0);
    vm.set_u64(this + 0x188, 0);
    vm.set_u32(this + 0x184, 0);
    for k in 0..9 {
        vm.set_u32(this + 0x60 + 4 * k, 1);
    }
    vm.set_u32(this + 0x16c, 4);
    vm.set_u32(this + 0x168, 4);
    let set = |vm: &mut Vm, a: u32, c: u32| {
        vm.set_u32(this + 0x168, a);
        vm.set_u32(this + 0x16c, c);
    };
    let no_four = |vm: &Vm, b: u64| (0..6).all(|k| vm.i32(b + 0x1cc8 + 4 * k) != 4);
    if vm.i32(b + 0x1cc4) != 0 && vm.i32(b + 0x1cc0) == 0 {
        vm.set_u32(this + 0x16c, 1);
        vm.set_u32(this + 0x168, 1);
    }
    if no_four(vm, b) {
        vm.set_u32(this + 0x16c, 1);
        vm.set_u32(this + 0x168, 1);
    }
    let (mut negative, mut positive) = (false, false);
    let slots = vm.i32(b + 0x91c);
    if slots > 0 {
        let items = vm.u64(b + 0x5ff8);
        for k in 0..slots as u64 {
            let v = vm.f32(items + 0x68 * k + 0x4c);
            negative |= 0.0 > v;
            positive |= v > 0.0;
        }
    }
    let (mut first, mut third) = (false, false);
    for i in 0..9u64 {
        first |= tank_condition(vm, this, i as u32, 1).unwrap_or(false);
        third |= tank_condition(vm, this, i as u32, 3).unwrap_or(false);
    }
    if negative && positive && first && third {
        let b = vm.u64(this + 8);
        if vm.i32(b + 0x1cc4) != 0 && vm.i32(b + 0x1cc0) == 0 {
            set(vm, 1, 3);
        }
        if no_four(vm, b) {
            set(vm, 1, 3);
        }
    }
    let b = vm.u64(this + 8);
    if vm.i32(b + 0x1cb4) != 0 {
        set(vm, 1, 3);
    }
    vm.set_u64(this + 0x170, 0);
    let on = !(0.1 <= f64::from(vm.f32(b + 0x28a4)) || vm.f32(b + 0x28a4).is_nan());
    let value = if on { 1 } else { flag };
    vm.set_u32(this + 0x17c, value);
    vm.set_u32(this + 0x180, value);
    vm.set_u32(this + 0x164, 0);
    let mut sum = vm.f32(this + 0x84) + 0.0;
    for k in 1..9 {
        sum += vm.f32(this + 0x84 + 4 * k);
    }
    vm.set_f32(this + 0x160, sum);
    if vm.i32(b + 0x91c) > 0 {
        let mut k = 0u64;
        loop {
            let r = vm.u64(vm.u64(this) + 0x68b0);
            vm.set_u32(r + 0x2cc * k + 0x1d4, 0);
            k += 1;
            if k as i64 >= i64::from(vm.i32(vm.u64(this + 8) + 0x91c)) {
                break;
            }
        }
    }
}

/// `0x140f5dd10(table)`: clears the flag `+0x50` and four runs of 500 words at `+0xff8`, `+0x5e18`, `+0x65e8`
/// and `+0x6db8` (the table at `0x145783800`).
pub fn clear_table(vm: &mut Vm, table: u64) {
    vm.set_u8(table + 0x50, 0);
    for k in 0..500u64 {
        for offset in [0xff8, 0x5e18, 0x65e8, 0x6db8] {
            vm.set_u32(table + offset + 4 * k, 0);
        }
    }
}

/// `0x1411aedb0(F)`: the first reset of a flight object: `F+0xbdb8` from `B+0xeb8` and `F+0xbdc4 = 1`; the words
/// `+0x68` and `+0x60` of the ten gear animation entries (`[F+0x6958]`, 0x90 bytes; out of range is an `Err`);
/// three 0.85 at `F+0xbe2c..`; the eight copies of `B+0x1b94` of the aircraft object `F+0x28` at `F+0x7a4..`; the
/// four tables of 770 floats at `F+0xdc0` (0), `F+0x19c8` (60), `F+0x25d0` (5), `F+0x3de0` (120), `F+0x49e8`
/// (100); and, on the primary object (`F+0x28 == 0`), the table `0x145783800`, the word `0x14576ce84`, the 16
/// bytes `0x142fe84e8` and six groups of three `u64` of the flight object `0x1426552ac`.
pub fn reset_flight_object(vm: &mut Vm, f: u64) -> Result<(), String> {
    let b = vm.u64(f + 0x20);
    let v = vm.u32(b + 0xeb8);
    vm.set_u32(f + 0xbdb8, v);
    vm.set_u32(f + 0xbdc4, 1);
    let entry = |vm: &Vm, i: u64| -> Result<u64, String> {
        let begin = vm.u64(f + 0x6958);
        let count = (vm.u64(f + 0x6960).wrapping_sub(begin) as i64) / 0x90;
        if count as u64 <= i {
            return Err("gear animation index out of range".into());
        }
        Ok(begin + 0x90 * i)
    };
    for i in 0..10 {
        let e = entry(vm, i)?;
        vm.set_u32(e + 0x68, 0);
    }
    for i in 0..10 {
        let e = entry(vm, i)?;
        vm.set_u32(e + 0x60, 0);
    }
    for offset in [0xbe2c, 0xbe30, 0xbe34] {
        vm.set_u32(f + offset, 0x3f59_999a);
    }
    for k in 0..8u64 {
        let object = aircraft_by_index(vm, vm.i32(f + 0x28));
        let v = vm.u32(object + 0x1b94);
        vm.set_u32(f + 0x7a4 + 4 * k, v);
    }
    for k in 0..0x302u64 {
        vm.set_u32(f + 0xdc0 + 4 * k, 0);
        vm.set_u32(f + 0x19c8 + 4 * k, 0x4270_0000);
        vm.set_u32(f + 0x25d0 + 4 * k, 0x40a0_0000);
        vm.set_u32(f + 0x3de0 + 4 * k, 0x42f0_0000);
        vm.set_u32(f + 0x49e8 + 4 * k, 0x42c8_0000);
    }
    vm.set_u64(f + 0x48, 0);
    vm.set_u32(f + 0x50, 0);
    if vm.i32(f + 0x28) == 0 {
        clear_table(vm, 0x1_4578_3800);
        vm.set_u32(0x1_4576_ce84, 0);
        vm.set_u64(0x1_42fe_84e8, 0);
        vm.set_u64(0x1_42fe_84f0, 0);
        for group in [0x6b14u64, 0x6b34, 0x6b54, 0x6b74, 0x6b94, 0x6bb4] {
            let t = flight_object(vm, vm.i32(INDEX));
            vm.set_u64(t + group, 0);
            vm.set_u64(t + group + 8, 0);
            vm.set_u64(t + group + 16, 0);
        }
    }
    Ok(())
}

/// `0x141a64260(table, index)`: the aircraft object `index`, at `[0x146124398] + index * 0x6668`.
pub fn aircraft_by_index(vm: &Vm, index: i32) -> u64 {
    vm.u64(0x1_4612_4398)
        .wrapping_add((i64::from(index) * 0x6668) as u64)
}

/// `0x14123d190(this, tank, mode)`: a test of the position of fuel tank `tank` (`mass::tank_position` of
/// `[this+8]`): mode 0 never, 1 the `x` below zero, 2 it is zero, 3 it is above zero, 4 always, 5 the height `z`
/// above `0.1 * B+0x64fc + this+0x54`, others never. A tank above 8 or a mode above 6 is the original's
/// fatal-error call (an `Err` here).
pub fn tank_condition(vm: &mut Vm, this: u64, tank: u32, mode: u32) -> Result<bool, String> {
    if tank > 8 || mode > 6 {
        return Err("tank condition argument out of range".into());
    }
    let b = vm.u64(this + 8);
    let [x, _, z] = crate::mass::tank_position(vm, b, tank as i32);
    Ok(match mode {
        1 => 0.0 > x,
        2 => x == 0.0,
        3 => x > 0.0,
        4 => true,
        5 => {
            let limit = f64::from(vm.f32(b + 0x64fc)) * 0.1 + f64::from(vm.f32(this + 0x54));
            f64::from(z) > limit
        }
        _ => false,
    })
}
