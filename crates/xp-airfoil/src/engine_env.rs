//! The engine helpers of [`crate::engine`] on the original's objects ([`Vm`]): the environment queries are the
//! callees of the original (frame time, input bindings, atmosphere, random generator, fuel draw), replayed in the
//! tests, and the engine record is a window of `0x2cc` bytes of memory.
use crate::engine::{EngineEnv, Record, starter_ready, starter_timer, thrust_term};
use crate::vm::{CallArgs, Callees, Vm};

const FRAME_TIME_OWNER: u64 = 0x142f_018b8;
const ENGINE_FLAG_OWNER: u64 = 0x1424_f5648;
const THRESHOLD_GLOBAL: u64 = 0x142f_01918;

/// An [`EngineEnv`] backed by the original's callees. The callees' memory effects are collected in `effects` (the
/// memory they would write is not the one the port is reading while the environment is borrowed).
pub struct CalleeEnv<'a> {
    env: &'a mut dyn Callees,
    f: u64,
    threshold: f64,
    pub effects: Vm,
}

impl<'a> CalleeEnv<'a> {
    pub fn new(env: &'a mut dyn Callees, vm: &Vm, f: u64) -> Self {
        CalleeEnv {
            env,
            f,
            threshold: vm.f64(THRESHOLD_GLOBAL),
            effects: Vm::default(),
        }
    }

    fn call(&mut self, address: u64, args: CallArgs) -> crate::vm::Reply {
        self.env.call(&mut self.effects, address, args)
    }

    fn float_args(&self, owner: u64, values: &[f32]) -> CallArgs {
        let mut a = CallArgs::ints(&[owner]);
        for (i, v) in values.iter().enumerate() {
            a.xmm[i + 1] = Some(v.to_bits());
        }
        a
    }
}

impl EngineEnv for CalleeEnv<'_> {
    fn atmosphere_a(&mut self, time: f32) -> f32 {
        let args = self.float_args(self.f + 0xbfa8, &[time]);
        f32::from_bits(self.call(0x141ba6750, args).xmm0 as u32)
    }
    fn atmosphere_b(&mut self, time: f32, value: f32) -> f32 {
        let args = self.float_args(self.f + 0xbfa8, &[time, value]);
        f32::from_bits(self.call(0x141ba64e0, args).xmm0 as u32)
    }
    fn engine_flag(&mut self) -> bool {
        let args = CallArgs::ints(&[ENGINE_FLAG_OWNER]);
        self.call(0x1417f12c0, args).rax as u8 != 0
    }
    fn frame_time(&mut self) -> f64 {
        let args = CallArgs::ints(&[FRAME_TIME_OWNER]);
        f64::from_bits(self.call(0x140c448c0, args).xmm0)
    }
    fn binding(&mut self, id: u32, index: i32) -> bool {
        // the second argument (the query mode: 2 for the first fuel cut-off test of `0x141238c20`, otherwise 1) is
        // not part of `EngineEnv::binding`, so it is not checked here
        let mut args = CallArgs::ints(&[self.f, 1, u64::from(id), index as u32 as u64]);
        args.int[1] = None;
        self.call(0x1407ace10, args).rax as u32 != 0
    }
    fn thrust_threshold(&mut self) -> f64 {
        self.threshold
    }
    fn fuel_draw(&mut self, amount: f32, interval: f32, mode: i32) {
        // (tanks, amount, mode, interval): the mode is the third argument, the interval the fourth
        let mut args = CallArgs::ints(&[self.f + 0xbd00]);
        args.xmm[1] = Some(amount.to_bits());
        args.int[2] = Some(mode as u32 as u64);
        args.xmm[3] = Some(interval.to_bits());
        self.call(0x14117c380, args);
    }
    fn random_unit(&mut self) -> f32 {
        let generator = self.call(0x1410c9c60, CallArgs::default()).rax;
        let args = CallArgs::ints(&[generator]);
        f32::from_bits(self.call(0x14067b2f0, args).xmm0 as u32)
    }
}

fn load_record(vm: &Vm, base: u64) -> Record {
    Record(
        (0..Record::WORDS as u64)
            .map(|i| vm.u32(base + 4 * i))
            .collect(),
    )
}

fn store_record(vm: &mut Vm, base: u64, before: &Record, after: &Record) {
    for (i, (b, a)) in before.0.iter().zip(&after.0).enumerate() {
        if b != a {
            vm.set_u32(base + 4 * i as u64, *a);
        }
    }
}

fn apply_effects(vm: &mut Vm, effects: &Vm) {
    for (address, word) in effects.words() {
        vm.set_u32(*address, *word);
    }
}

/// `0x141238c20(F, n)`: the thrust term of engine `n`; see [`thrust_term`].
pub fn thrust_term_of(vm: &mut Vm, env: &mut dyn Callees, f: u64, n: i32) -> f32 {
    let b = vm.u64(f + 0x20);
    let engine = vm.u64(b + 0x5ff8) + (i64::from(n) * 0x68) as u64;
    let part = vm.u64(b + 0x6010) + (i64::from(n) * 0x3770) as u64;
    let state = vm.u64(f + 0x68b0) + (i64::from(n) * 0x2cc) as u64;
    let before = load_record(vm, state);
    let mut record = before.clone();
    let mut callees = CalleeEnv::new(env, vm, f);
    let level = thrust_term(
        &(&*vm, f),
        &(&*vm, b),
        &(&*vm, engine),
        &(&*vm, part),
        &mut record,
        n,
        &mut callees,
    );
    let effects = callees.effects;
    store_record(vm, state, &before, &record);
    apply_effects(vm, &effects);
    level
}

/// `0x1411924e0(M, start)`: the starter delay of the engine state record at `state`; see [`starter_timer`].
pub fn starter_delay(vm: &mut Vm, env: &mut dyn Callees, f: u64, state: u64, start: bool) {
    let before = load_record(vm, state);
    let mut record = before.clone();
    let mut callees = CalleeEnv::new(env, vm, f);
    starter_timer(&mut record, start, &mut callees);
    let effects = callees.effects;
    store_record(vm, state, &before, &record);
    apply_effects(vm, &effects);
}

/// `0x1411a2d90(F, n)`: whether engine `n` can be started; see [`starter_ready`].
pub fn starter_ready_of(vm: &mut Vm, env: &mut dyn Callees, f: u64, n: i32) -> bool {
    let b = vm.u64(f + 0x20);
    let state = vm.u64(f + 0x68b0) + (i64::from(n) * 0x2cc) as u64;
    let record = load_record(vm, state);
    let mut callees = CalleeEnv::new(env, vm, f);
    let ready = starter_ready(&(&*vm, b), &record, n, &mut callees);
    let effects = callees.effects;
    apply_effects(vm, &effects);
    ready
}
