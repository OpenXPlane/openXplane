//! Concrete menu-panel fallback 0x140f00310 with recovered property-2 gate.
//! Window construction/resolution/submission remain explicit service bodies.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub engine_byte: u8,
    pub ui_mode: i32,
}
impl State {
    /// Original panel interface 0x140efa740 for property 2 and predicate 0x1414ca5d0.
    pub fn property2(&self) -> f32 {
        if self.engine_byte == 0 && self.ui_mode == 2 {
            0.0
        } else {
            1.0
        }
    }
}
pub trait Services {
    /// 0x140f0a1b0; called independently twice where reached. Full meaning/body unported.
    fn query_a(&mut self, state: &mut State) -> u8;
    /// 0x1417e2640 on global 0x1460fc940. Full meaning/body unported.
    fn query_b(&mut self, state: &mut State) -> u8;
    /// Slot +0x170 interface +0x08 -> thunk 0x140ef6f10 -> body 0x140f0a2c0, argument 0.
    fn direct(&mut self, state: &mut State, argument: i32);
    fn allocate(&mut self, state: &mut State, bytes: u64) -> u64;
    /// Original empty wrapper ctor 0x1410c7d20; two independent temporaries.
    fn callback(&mut self, state: &mut State, index: u8) -> u64;
    fn construct(&mut self, state: &mut State, allocation: u64, second: u64, first: u64) -> u64;
    fn drop_callback(&mut self, state: &mut State, callback: u64);
    fn manager(&mut self, state: &mut State) -> u64;
    fn registry_input(&mut self, state: &mut State, object: u64) -> u64;
    fn resolve(&mut self, state: &mut State, manager: u64, input: u64) -> u64;
    fn destroy(&mut self, state: &mut State, object: u64, flag: u32);
    /// 0x140ef67c0 receives pointer to word 0 and result/null at panel+0x170.
    fn submit(&mut self, state: &mut State, word: i32, result: u64);
}
pub fn dispatch(state: &mut State, services: &mut impl Services) -> Result<(), &'static str> {
    if state.property2() == 0.0 {
        return Ok(());
    }
    if state.engine_byte != 0 {
        services.direct(state, 0);
        return Ok(());
    }
    if services.query_a(state) != 0 && services.query_b(state) != 0 {
        services.direct(state, 0);
        return Ok(());
    }
    let result = if services.query_a(state) != 0 {
        0
    } else {
        let allocation = services.allocate(state, 0x320);
        if allocation == 0 {
            return Err("native non-null construction allocation required");
        }
        let first = services.callback(state, 0);
        let second = services.callback(state, 1);
        let object = services.construct(state, allocation, second, first);
        if object == 0 {
            return Err("native non-null constructor result required");
        }
        services.drop_callback(state, second);
        services.drop_callback(state, first);
        let manager = services.manager(state);
        let input = services.registry_input(state, object);
        let result = services.resolve(state, manager, input);
        services.destroy(state, object, 1);
        result
    };
    services.submit(state, 0, result);
    Ok(())
}
