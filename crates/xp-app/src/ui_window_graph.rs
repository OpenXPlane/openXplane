//! Composes native window visibility with widget reparent on shared ancestry.
//! Widget alpha/parents are authoritative; registered opacity paths are derived.
use crate::{
    ui_state::cumulative_opacity,
    ui_widget_tree::{self, Tree},
    ui_window_visibility as visibility,
};
#[derive(Clone, Debug, Default)]
pub struct State {
    pub windows: visibility::State,
    pub tree: Tree,
    /// Raw widget +0x58 values, aligned with tree nodes.
    pub alpha: Vec<f32>,
    /// Registered entry index -> widget node index.
    pub widgets: Vec<usize>,
    pub implementation_node: usize,
}
fn path(tree: &Tree, alpha: &[f32], mut i: usize) -> Result<Vec<f32>, &'static str> {
    let mut out = Vec::new();
    for _ in 0..=tree.nodes.len() {
        let node = tree.nodes.get(i).ok_or("invalid opacity node")?;
        out.push(*alpha.get(i).ok_or("missing widget alpha")?);
        let Some(parent) = node.parent else {
            out.reverse();
            return Ok(out);
        };
        i = parent;
    }
    Err("cyclic opacity ancestry")
}
fn sync(
    windows: &mut visibility::State,
    tree: &Tree,
    alpha: &[f32],
    widgets: &[usize],
) -> Result<(), &'static str> {
    if widgets.len() != windows.registry.entries.len() {
        return Err("widget/entry mapping mismatch");
    }
    for (entry, &widget) in windows.registry.entries.iter_mut().zip(widgets) {
        entry.opacity_root_to_leaf = path(tree, alpha, widget)?;
    }
    Ok(())
}
impl State {
    pub fn visible(&self, token: u64) -> Result<bool, &'static str> {
        let entry = self
            .windows
            .registry
            .order
            .iter()
            .copied()
            .find(|&i| {
                self.windows
                    .registry
                    .entries
                    .get(i)
                    .is_some_and(|w| w.token == token)
            })
            .ok_or("unregistered graph window")?;
        let widget = *self
            .widgets
            .get(entry)
            .ok_or("missing graph window widget")?;
        Ok(cumulative_opacity(&path(&self.tree, &self.alpha, widget)?)? > 0.0)
    }
}
/// Remaining window endpoints do not mutate graph fields in this verified profile.
/// Widget callbacks may mutate the tree through the inherited native callback API.
pub trait Services: ui_widget_tree::Services {
    fn owner(&mut self, entry: usize, attached: bool);
    fn prepare_hide(&mut self, entry: usize);
    fn activate(&mut self, entry: usize, show: bool);
    fn refresh(&mut self);
    fn event(&mut self, token: u64);
}
struct Adapter<'a, S> {
    tree: &'a mut Tree,
    alpha: &'a mut [f32],
    widgets: &'a [usize],
    implementation: usize,
    services: &'a mut S,
}
impl<S: Services> visibility::Services for Adapter<'_, S> {
    fn owner(&mut self, _: &mut visibility::State, i: usize, attached: bool) {
        self.services.owner(i, attached);
    }
    fn prepare_hide(&mut self, _: &mut visibility::State, i: usize) {
        self.services.prepare_hide(i);
    }
    fn activate(&mut self, _: &mut visibility::State, i: usize, show: bool) {
        self.services.activate(i, show);
    }
    fn reparent(
        &mut self,
        windows: &mut visibility::State,
        i: usize,
        show: bool,
    ) -> Result<(), &'static str> {
        let widget = *self.widgets.get(i).ok_or("missing graph widget")?;
        // The native visibility body writes leaf alpha immediately before reparent.
        *self.alpha.get_mut(widget).ok_or("missing graph alpha")? = *windows.registry.entries[i]
            .opacity_root_to_leaf
            .last()
            .ok_or("missing graph opacity path")?;
        ui_widget_tree::reparent(
            self.tree,
            widget,
            show.then_some(self.implementation),
            self.services,
        )?;
        sync(windows, self.tree, self.alpha, self.widgets)
    }
    fn refresh(&mut self, _: &mut visibility::State) {
        self.services.refresh();
    }
    fn event(&mut self, _: &mut visibility::State, token: u64) {
        self.services.event(token);
    }
}
pub fn dispatch(
    state: &mut State,
    token: u64,
    show: u32,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    sync(
        &mut state.windows,
        &state.tree,
        &state.alpha,
        &state.widgets,
    )?;
    let mut adapter = Adapter {
        tree: &mut state.tree,
        alpha: &mut state.alpha,
        widgets: &state.widgets,
        implementation: state.implementation_node,
        services,
    };
    visibility::dispatch(&mut state.windows, token, show, 0, 0.0, &mut adapter)
}
pub fn toggle(
    state: &mut State,
    token: u64,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let show = u32::from(!state.visible(token)?);
    dispatch(state, token, show, services)
}
