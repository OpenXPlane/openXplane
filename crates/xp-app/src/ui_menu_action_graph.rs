//! Concrete fallback -> complete direct action -> actual graph-backed controller close.
use crate::{
    ui_menu_action as action, ui_menu_close_graph as close, ui_menu_direct as direct,
    ui_menu_direct_graph,
};
struct Bridge<'a, S> {
    panel: &'a mut direct::State,
    controller: &'a mut close::State,
    identity: u64,
    effects: &'a mut S,
}
impl<S: action::Services + direct::Services + close::Services> action::Services for Bridge<'_, S> {
    fn query_a(&mut self, state: &mut action::State) -> u8 {
        action::Services::query_a(self.effects, state)
    }
    fn query_b(&mut self, state: &mut action::State) -> u8 {
        action::Services::query_b(self.effects, state)
    }
    fn allocate(&mut self, state: &mut action::State, bytes: u64) -> u64 {
        action::Services::allocate(self.effects, state, bytes)
    }
    fn callback(&mut self, state: &mut action::State, index: u8) -> u64 {
        action::Services::callback(self.effects, state, index)
    }
    fn construct(
        &mut self,
        state: &mut action::State,
        allocation: u64,
        second: u64,
        first: u64,
    ) -> u64 {
        action::Services::construct(self.effects, state, allocation, second, first)
    }
    fn drop_callback(&mut self, state: &mut action::State, callback: u64) {
        action::Services::drop_callback(self.effects, state, callback)
    }
    fn manager(&mut self, state: &mut action::State) -> u64 {
        action::Services::manager(self.effects, state)
    }
    fn registry_input(&mut self, state: &mut action::State, object: u64) -> u64 {
        action::Services::registry_input(self.effects, state, object)
    }
    fn resolve(&mut self, state: &mut action::State, manager: u64, input: u64) -> u64 {
        action::Services::resolve(self.effects, state, manager, input)
    }
    fn destroy(&mut self, state: &mut action::State, object: u64, flag: u32) {
        action::Services::destroy(self.effects, state, object, flag)
    }
    fn submit(&mut self, state: &mut action::State, word: i32, result: u64) {
        action::Services::submit(self.effects, state, word, result)
    }
    fn direct(&mut self, state: &mut action::State, argument: i32) -> Result<(), &'static str> {
        // Both recovered handlers read the same native UI-mode field.
        self.panel.ui_mode = state.ui_mode;
        let result = ui_menu_direct_graph::dispatch(
            self.panel,
            self.controller,
            self.identity,
            argument,
            self.effects,
        );
        state.ui_mode = self.panel.ui_mode;
        result
    }
}
pub fn dispatch(
    state: &mut action::State,
    panel: &mut direct::State,
    controller: &mut close::State,
    identity: u64,
    effects: &mut (impl action::Services + direct::Services + close::Services),
) -> Result<(), &'static str> {
    action::dispatch(
        state,
        &mut Bridge {
            panel,
            controller,
            identity,
            effects,
        },
    )
}
