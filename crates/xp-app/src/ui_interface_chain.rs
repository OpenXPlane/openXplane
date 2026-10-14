//! Global interface ctor 0x1410c7bb0 / scalar destructor 0x1410c8d30.
//! Native global0x14588f558, +8 newer and +10 older; links remain in detached nodes.
pub const BASE_TABLE: u64 = 0x1426401f8;
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub table: u64,
    pub newer: Option<usize>,
    pub older: Option<usize>,
    pub free_requested: bool,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub head: Option<usize>,
    pub nodes: Vec<Node>,
}
pub trait Services {
    fn free(&mut self, state: &mut State, node: usize, bytes: usize);
}
impl State {
    fn node(&self, i: usize) -> Result<&Node, &'static str> {
        self.nodes.get(i).ok_or("invalid interface identity")
    }
    fn node_mut(&mut self, i: usize) -> Result<&mut Node, &'static str> {
        self.nodes.get_mut(i).ok_or("invalid interface identity")
    }
    pub fn construct(&mut self, i: usize) -> Result<usize, &'static str> {
        let old = self.head;
        let node = self.node_mut(i)?;
        node.table = BASE_TABLE;
        node.newer = None;
        node.older = old;
        node.free_requested = false;
        self.head = Some(i);
        if let Some(old) = old {
            self.node_mut(old)?.newer = Some(i);
        }
        Ok(i)
    }
    pub fn destroy(
        &mut self,
        i: usize,
        flags: u32,
        services: &mut impl Services,
    ) -> Result<usize, &'static str> {
        let newer = self.node(i)?.newer;
        let older = self.node(i)?.older;
        self.node_mut(i)?.table = BASE_TABLE;
        if self.head == Some(i) {
            self.head = older;
        }
        if let Some(newer) = newer {
            self.node_mut(newer)?.older = older;
        }
        if let Some(older) = older {
            self.node_mut(older)?.newer = newer;
        }
        if flags & 1 != 0 {
            self.node_mut(i)?.free_requested = true;
            services.free(self, i, 24);
        }
        Ok(i)
    }
}
