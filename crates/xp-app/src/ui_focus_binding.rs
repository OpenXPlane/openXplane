//! Native owner binding 0x1412f6250, distinct from widget parent+0x60.
//! Owner/selected identities model native +0x08/+0x10; callback bodies are services.
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub owner: Option<usize>,
    pub selected: Option<usize>,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub nodes: Vec<Node>,
    pub focused: Option<usize>,
}
pub trait Services {
    fn blur(&mut self, state: &mut State, leaf: usize);
    /// Native virtual +0x10; any nonzero low byte stops ancestor clearing.
    fn stop(&mut self, state: &mut State, node: usize) -> u8;
}
impl State {
    fn node(&self, i: usize) -> Result<&Node, &'static str> {
        self.nodes.get(i).ok_or("invalid binding node")
    }
    fn node_mut(&mut self, i: usize) -> Result<&mut Node, &'static str> {
        self.nodes.get_mut(i).ok_or("invalid binding node")
    }
    fn selected_leaf(&self, mut i: usize) -> Result<usize, &'static str> {
        for _ in 0..=self.nodes.len() {
            let Some(next) = self.node(i)?.selected else {
                return Ok(i);
            };
            i = next;
        }
        Err("cyclic selection chain")
    }
}
pub fn bind(
    state: &mut State,
    node: usize,
    requested: Option<usize>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if let Some(i) = requested {
        state.node(i)?;
    }
    let old = state.node(node)?.owner;
    if old == requested {
        return Ok(());
    }
    if let Some(old) = old {
        let mut current = old;
        let mut child = node;
        let mut focused_chain = false;
        let mut completed = false;
        for _ in 0..=state.nodes.len() {
            let parent = state.node(current)?;
            if parent.selected != Some(child) {
                completed = true;
                break;
            }
            if let Some(owner) = parent.owner {
                child = current;
                current = owner;
            } else {
                focused_chain = state.focused == Some(current);
                completed = true;
                break;
            }
        }
        if !completed {
            return Err("cyclic binding owner chain");
        }
        if focused_chain {
            let leaf = state.selected_leaf(current)?;
            services.blur(state, leaf);
            // Native reloads node+8 after blur and each cleared ancestor+8 after callback.
            let mut parent = state.node(node)?.owner;
            let mut finished = false;
            for _ in 0..=state.nodes.len() {
                let Some(i) = parent else {
                    finished = true;
                    break;
                };
                state.node_mut(i)?.selected = None;
                if services.stop(state, i) != 0 {
                    finished = true;
                    break;
                }
                parent = state.node(i)?.owner;
            }
            if !finished {
                return Err("cyclic binding clear chain");
            }
        } else if state.node(old)?.selected == Some(node) {
            state.node_mut(old)?.selected = None;
        }
    }
    state.node_mut(node)?.owner = requested;
    Ok(())
}
