//! Widget reparent 0x1412f77a0 and dirty propagation 0x1412e48f0.
//! Stable node identities; focus-owner binding and virtual callbacks remain services.
#[derive(Clone, Debug, Default)]
pub struct Node {
    pub parent: Option<usize>,
    /// Native +0x68 override for the argument passed to 0x1412f6250.
    pub binding_override: u64,
    pub children: Vec<usize>,
    /// Raw native +0xf8 bits; semantic names beyond propagation are unproven.
    pub flags: u32,
}
#[derive(Clone, Debug, Default)]
pub struct Tree {
    pub nodes: Vec<Node>,
}
fn upstream(mut bits: u32) -> u32 {
    if bits & 1 != 0 {
        bits = (bits & !1) | 2;
    }
    if bits & 12 != 0 {
        bits |= 1;
    }
    bits
}
impl Tree {
    fn node(&self, i: usize) -> Result<&Node, &'static str> {
        self.nodes.get(i).ok_or("invalid widget node")
    }
    fn node_mut(&mut self, i: usize) -> Result<&mut Node, &'static str> {
        self.nodes.get_mut(i).ok_or("invalid widget node")
    }
    /// Store all requested bits, propagate only newly introduced bits, transform
    /// bit 1 to bit 2 and introduce bit 1 for bits 4/8 at each ancestor step.
    pub fn dirty(&mut self, mut i: usize, mut bits: u32) -> Result<(), &'static str> {
        for _ in 0..=self.nodes.len() {
            let node = self.node_mut(i)?;
            let added = !node.flags & bits;
            node.flags |= bits;
            if added == 0 {
                return Ok(());
            }
            let Some(parent) = node.parent else {
                return Ok(());
            };
            i = parent;
            bits = upstream(added);
        }
        Err("cyclic dirty propagation")
    }
    fn parent_dirty(&mut self, i: usize, mask: u32) -> Result<(), &'static str> {
        let node = self.node_mut(i)?;
        let added = !node.flags & mask;
        node.flags |= mask;
        if added != 0
            && let Some(parent) = node.parent
        {
            self.dirty(parent, upstream(added))?;
        }
        Ok(())
    }
}
pub trait Services {
    /// Binding endpoint 0x1412f6250; argument identity uses index+1 for nodes, 0 for null.
    fn bind(&mut self, tree: &mut Tree, child: usize, target: u64);
    /// Parent virtual slot +0x188, after child-vector erase and before dirty stores.
    fn removed(&mut self, tree: &mut Tree, parent: usize, child: usize);
    /// Parent virtual slot +0x180, after append and before dirty stores.
    fn added(&mut self, tree: &mut Tree, parent: usize, child: usize);
}
/// Unlike a guarded parent setter, this entry detaches/appends even for equal parents.
/// Spare native vector capacity is the verified profile; allocation failure and
/// callbacks destroying nodes are unsupported. Callback field mutations are retained.
pub fn reparent(
    tree: &mut Tree,
    child: usize,
    requested: Option<usize>,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    if let Some(i) = requested {
        tree.node(i)?;
    }
    let override_target = tree.node(child)?.binding_override;
    let target = if override_target != 0 {
        override_target
    } else {
        requested.map_or(0, |i| i as u64 + 1)
    };
    services.bind(tree, child, target);
    if let Some(old) = tree.node(child)?.parent {
        let position = tree
            .node(old)?
            .children
            .iter()
            .position(|&i| i == child)
            .ok_or("native erase requires child membership")?;
        tree.node_mut(old)?.children.remove(position);
        services.removed(tree, old, child);
        let current = tree
            .node(child)?
            .parent
            .ok_or("native post-remove parent dereference")?;
        tree.parent_dirty(current, 13)?;
    }
    tree.node_mut(child)?.parent = requested;
    if let Some(new) = requested {
        tree.node_mut(new)?.children.push(child);
        // The native call reloads child+0x60, which is still requested before callback.
        services.added(tree, new, child);
        let mask = if tree.node(child)?.flags & 3 != 0 {
            15
        } else {
            13
        };
        let current = tree
            .node(child)?
            .parent
            .ok_or("native post-add parent dereference")?;
        tree.parent_dirty(current, mask)?;
    }
    Ok(())
}
