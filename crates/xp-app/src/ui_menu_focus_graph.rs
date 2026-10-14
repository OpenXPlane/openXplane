//! Menu graph with recovered owner/selected binding, rather than a fixed focus flag.
use crate::{
    ui_focus_binding as binding, ui_input::KeyInput, ui_menu_action as action,
    ui_menu_close_graph as close, ui_menu_direct as direct, ui_menu_key_graph as keys,
    ui_widget_tree as tree, ui_window_graph as graph,
};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default)]
pub struct State {
    pub menu: keys::State,
    pub binding: binding::State,
    pub entry_bindings: Vec<usize>,
    pub widget_bindings: Vec<usize>,
    /// Opaque tree binding target token -> owner/selected node identity.
    pub targets: BTreeMap<u64, usize>,
    pub panel_binding: usize,
}
pub trait Services:
    action::Services + direct::Services + close::Services + binding::Services
{
    /// Focused child dispatch remains an explicit virtual callback endpoint.
    fn child(&mut self, state: &mut binding::State, node: usize, input: KeyInput) -> u8;
}
struct Bridge<'a, S> {
    binding: &'a mut binding::State,
    entries: &'a [usize],
    widgets: &'a [usize],
    targets: &'a BTreeMap<u64, usize>,
    implementation: usize,
    effects: &'a mut S,
    error: Option<&'static str>,
}
impl<S: Services> action::Services for Bridge<'_, S> {
    fn query_a(&mut self, state: &mut action::State) -> u8 {
        action::Services::query_a(self.effects, state)
    }
    fn query_b(&mut self, state: &mut action::State) -> u8 {
        action::Services::query_b(self.effects, state)
    }
    fn direct(&mut self, state: &mut action::State, argument: i32) -> Result<(), &'static str> {
        action::Services::direct(self.effects, state, argument)
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
}
impl<S: Services> direct::Services for Bridge<'_, S> {
    fn prepare(&mut self, state: &mut direct::State) {
        direct::Services::prepare(self.effects, state)
    }
    fn controller(&mut self, state: &mut direct::State) -> u64 {
        direct::Services::controller(self.effects, state)
    }
    fn close(
        &mut self,
        state: &mut direct::State,
        controller: u64,
        kind: i32,
        flag: u8,
    ) -> Result<(), &'static str> {
        direct::Services::close(self.effects, state, controller, kind, flag)
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
}
impl<S: Services> close::Services for Bridge<'_, S> {
    fn update(&mut self, state: &mut close::State, kind: i32, flag: u8) {
        close::Services::update(self.effects, state, kind, flag)
    }
    fn reset(&mut self, state: &mut close::State, token: u64, mode: i32, extra: i32) {
        close::Services::reset(self.effects, state, token, mode, extra)
    }
    fn delete(&mut self, state: &mut close::State, token: u64, flag: u8) {
        close::Services::delete(self.effects, state, token, flag)
    }
    fn environment(&mut self, state: &mut close::State) -> u8 {
        close::Services::environment(self.effects, state)
    }
    fn command_once(&mut self, state: &mut close::State) {
        close::Services::command_once(self.effects, state)
    }
    fn controller_refresh(&mut self, state: &mut close::State) {
        close::Services::controller_refresh(self.effects, state)
    }
    fn final_update(&mut self, state: &mut close::State, kind: i32) {
        close::Services::final_update(self.effects, state, kind)
    }
}
impl<S: Services> graph::Services for Bridge<'_, S> {
    fn prepare_hide(&mut self, entry: usize) {
        graph::Services::prepare_hide(self.effects, entry)
    }
    fn activate(&mut self, entry: usize, show: bool) {
        graph::Services::activate(self.effects, entry, show)
    }
    fn refresh(&mut self) {
        graph::Services::refresh(self.effects)
    }
    fn event(&mut self, token: u64) {
        graph::Services::event(self.effects, token)
    }
    fn owner(&mut self, entry: usize, attached: bool) {
        let result = (|| {
            let i = *self.entries.get(entry).ok_or("missing entry binding")?;
            binding::bind(
                self.binding,
                i,
                attached.then_some(self.implementation),
                self.effects,
            )
        })();
        if let Err(e) = result {
            self.error = Some(e);
        }
    }
}
impl<S: Services> tree::Services for Bridge<'_, S> {
    fn removed(&mut self, tree: &mut tree::Tree, parent: usize, child: usize) {
        tree::Services::removed(self.effects, tree, parent, child)
    }
    fn added(&mut self, tree: &mut tree::Tree, parent: usize, child: usize) {
        tree::Services::added(self.effects, tree, parent, child)
    }
    fn bind(&mut self, _: &mut tree::Tree, child: usize, target: u64) {
        let result = (|| {
            let i = *self.widgets.get(child).ok_or("missing widget binding")?;
            let requested = if target == 0 {
                None
            } else {
                Some(
                    *self
                        .targets
                        .get(&target)
                        .ok_or("unknown binding target identity")?,
                )
            };
            binding::bind(self.binding, i, requested, self.effects)
        })();
        if let Err(e) = result {
            self.error = Some(e);
        }
    }
}
pub fn dispatch(
    state: &mut State,
    input: KeyInput,
    effects: &mut impl Services,
) -> Result<u8, &'static str> {
    let implementation = *state
        .widget_bindings
        .get(state.menu.controller.graph.implementation_node)
        .ok_or("missing implementation binding")?;
    let mut bridge = Bridge {
        binding: &mut state.binding,
        entries: &state.entry_bindings,
        widgets: &state.widget_bindings,
        targets: &state.targets,
        implementation,
        effects,
        error: None,
    };
    let result = keys::dispatch(&mut state.menu, input, &mut bridge)?;
    if let Some(e) = bridge.error {
        return Err(e);
    }
    Ok(result)
}
/// Captures the current focus before child dispatch, like native 0x1412c6300.
/// This executor supports a null focus or this panel; other widget classes need their own executor.
pub fn dispatch_focused(
    state: &mut State,
    mask: u32,
    input: KeyInput,
    effects: &mut impl Services,
) -> Result<u8, &'static str> {
    let Some(focused) = state.binding.focused else {
        return Ok(0);
    };
    if focused != state.panel_binding {
        return Err("focused widget has no menu executor");
    }
    let child = state
        .binding
        .nodes
        .get(focused)
        .ok_or("invalid focused binding")?
        .selected;
    if let Some(child) = child
        && effects.child(&mut state.binding, child, input) != 0
    {
        return Ok(1);
    }
    if !crate::ui_input::accepts_key(mask, input)? {
        return Ok(0);
    }
    dispatch(state, input, effects)
}
