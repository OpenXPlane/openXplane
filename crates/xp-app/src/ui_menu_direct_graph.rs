//! Direct panel action composed with registered controller close on the shared graph.
//! Remaining direct/window effect services keep their existing explicit boundaries.
use crate::{ui_menu_close_graph as close, ui_menu_direct as direct};
struct Bridge<'a, S> {
    controller: &'a mut close::State,
    identity: u64,
    effects: &'a mut S,
}
impl<S: direct::Services + close::Services> direct::Services for Bridge<'_, S> {
    fn prepare(&mut self, state: &mut direct::State) {
        direct::Services::prepare(self.effects, state)
    }
    fn controller(&mut self, state: &mut direct::State) -> u64 {
        direct::Services::controller(self.effects, state)
    }
    fn notify(&mut self, state: &mut direct::State) {
        direct::Services::notify(self.effects, state)
    }
    fn query_a(&mut self, state: &mut direct::State) -> u8 {
        direct::Services::query_a(self.effects, state)
    }
    fn document(&mut self, state: &mut direct::State) -> Vec<u8> {
        direct::Services::document(self.effects, state)
    }
    fn validate(&mut self, state: &mut direct::State, document: &[u8]) -> u8 {
        direct::Services::validate(self.effects, state, document)
    }
    fn load(&mut self, state: &mut direct::State, document: &[u8], validation: u8) -> u8 {
        direct::Services::load(self.effects, state, document, validation)
    }
    fn load_failed(&mut self, state: &mut direct::State) {
        direct::Services::load_failed(self.effects, state)
    }
    fn drop_document(&mut self, state: &mut direct::State, document: Vec<u8>) {
        direct::Services::drop_document(self.effects, state, document)
    }
    fn allocate(&mut self, state: &mut direct::State, bytes: u64) -> u64 {
        direct::Services::allocate(self.effects, state, bytes)
    }
    fn callback(&mut self, state: &mut direct::State, id: i32) -> u64 {
        direct::Services::callback(self.effects, state, id)
    }
    fn construct(
        &mut self,
        state: &mut direct::State,
        allocation: u64,
        source: u64,
        second: u64,
        first: u64,
    ) -> u64 {
        direct::Services::construct(self.effects, state, allocation, source, second, first)
    }
    fn drop_callback(&mut self, state: &mut direct::State, callback: u64) {
        direct::Services::drop_callback(self.effects, state, callback)
    }
    fn field_c8_action(&mut self, state: &mut direct::State, object: u64) {
        direct::Services::field_c8_action(self.effects, state, object)
    }
    fn component_260_action(&mut self, state: &mut direct::State, object: u64) {
        direct::Services::component_260_action(self.effects, state, object)
    }
    fn field_d8_action(&mut self, state: &mut direct::State, object: u64) {
        direct::Services::field_d8_action(self.effects, state, object)
    }
    fn destroy(&mut self, state: &mut direct::State, object: u64, flag: u32) {
        direct::Services::destroy(self.effects, state, object, flag)
    }
    fn close(
        &mut self,
        _: &mut direct::State,
        controller: u64,
        kind: i32,
        flag: u8,
    ) -> Result<(), &'static str> {
        if controller != self.identity {
            return Err("direct controller identity does not match graph controller");
        }
        close::dispatch(self.controller, kind, flag, self.effects)
    }
}
/// Overrides the legacy direct close endpoint with the actual recovered graph-backed close.
/// Other services delegate unchanged. Controller identity must be supplied by the caller.
pub fn dispatch(
    panel: &mut direct::State,
    controller: &mut close::State,
    identity: u64,
    argument: i32,
    effects: &mut (impl direct::Services + close::Services),
) -> Result<(), &'static str> {
    direct::dispatch(
        panel,
        argument,
        &mut Bridge {
            controller,
            identity,
            effects,
        },
    )
}
