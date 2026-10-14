//! Direct menu-panel action 0x140f0a2c0: guard, kind-11 close and document/dialog requests.
//! Concrete preparation, controller/window, document, binding and dialog bodies remain services.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub guard_c9: u8,
    pub flag_c8: u8,
    pub ui_mode: i32,
    pub source_token: u64,
    pub flag_7770c6: u8,
}
pub trait Services {
    fn prepare(&mut self, state: &mut State);
    fn controller(&mut self, state: &mut State) -> u64;
    fn close(&mut self, state: &mut State, controller: u64, kind: i32, flag: u8);
    fn notify(&mut self, state: &mut State);
    fn query_a(&mut self, state: &mut State) -> u8;
    fn document(&mut self, state: &mut State) -> Vec<u8>;
    fn validate(&mut self, state: &mut State, document: &[u8]) -> u8;
    fn load(&mut self, state: &mut State, document: &[u8], validation: u8) -> u8;
    fn load_failed(&mut self, state: &mut State);
    fn drop_document(&mut self, state: &mut State, document: Vec<u8>);
    fn allocate(&mut self, state: &mut State, bytes: u64) -> u64;
    fn callback(&mut self, state: &mut State, id: i32) -> u64;
    fn construct(
        &mut self,
        state: &mut State,
        allocation: u64,
        source: u64,
        second: u64,
        first: u64,
    ) -> u64;
    fn drop_callback(&mut self, state: &mut State, callback: u64);
    fn field_c8_action(&mut self, state: &mut State, object: u64);
    fn component_260_action(&mut self, state: &mut State, object: u64);
    fn field_d8_action(&mut self, state: &mut State, object: u64);
    fn destroy(&mut self, state: &mut State, object: u64, flag: u32);
}
pub fn dispatch(
    state: &mut State,
    argument: i32,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if state.guard_c9 != 0 {
        return Ok(());
    }
    state.guard_c9 = 1;
    services.prepare(state);
    let controller = services.controller(state);
    services.close(state, controller, 11, u8::from(state.flag_c8 == 0));
    if argument != 12 && state.ui_mode == 1 && argument <= 0 {
        services.notify(state);
        let mut loaded = false;
        if services.query_a(state) != 0 {
            let document = services.document(state);
            let validation = services.validate(state, &document);
            loaded = services.load(state, &document, validation) != 0;
            if !loaded {
                services.load_failed(state);
            }
            services.drop_document(state, document);
        }
        if !loaded {
            let allocation = services.allocate(state, 0x320);
            if allocation == 0 {
                return Err("native successful dialog allocation required");
            }
            let first = services.callback(state, 0);
            let second = services.callback(state, 3);
            let object = services.construct(state, allocation, state.source_token, second, first);
            if object == 0 {
                return Err("native non-null constructor result required");
            }
            services.drop_callback(state, second);
            services.drop_callback(state, first);
            state.flag_7770c6 = 1;
            services.field_c8_action(state, object);
            services.component_260_action(state, object);
            services.field_d8_action(state, object);
            services.destroy(state, object, 1);
        }
    }
    state.guard_c9 = 0;
    Ok(())
}
