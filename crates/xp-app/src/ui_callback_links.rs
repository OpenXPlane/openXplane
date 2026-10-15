//! Callback-wrapper connection lifecycle used by shared menu dialogs.
//! Original bodies: base1410c7b50, ctor1410c7d20, copy1410c7bf0,
//! symmetric registration1410c8de0, base teardown1410c8430.
//! Ordered link sets model observable registration; native red-black node layout
//! and its internal free order are not represented here.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Container,
    Wrapper { first: u64, second: u64 },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub peers: BTreeSet<u64>,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub nodes: BTreeMap<u64, Node>,
}
/// Allocation/free requests are an observable audit, not a host memory allocator.
pub trait Services {
    fn allocate(&mut self, bytes: usize);
    fn free(&mut self, bytes: usize);
}
impl State {
    fn insert(
        &mut self,
        id: u64,
        kind: Kind,
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        if id == 0 || self.nodes.contains_key(&id) {
            return Err("fresh stable nonnull identity required");
        }
        services.allocate(40);
        self.nodes.insert(
            id,
            Node {
                kind,
                peers: BTreeSet::new(),
            },
        );
        Ok(())
    }
    pub fn container(&mut self, id: u64, services: &mut impl Services) -> Result<(), &'static str> {
        self.insert(id, Kind::Container, services)
    }
    /// Fresh wrapper, null custom callable, R8/R9 stored without narrowing.
    pub fn construct(
        &mut self,
        id: u64,
        parent: Option<u64>,
        first: u64,
        second: u64,
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        if parent.is_some_and(|p| !self.nodes.contains_key(&p)) {
            return Err("missing registration endpoint");
        }
        self.insert(id, Kind::Wrapper { first, second }, services)?;
        if let Some(parent) = parent {
            self.bind(id, parent, services)?;
        }
        Ok(())
    }
    /// Both sets are checked separately; repeated registration is idempotent.
    pub fn bind(
        &mut self,
        left: u64,
        right: u64,
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        if !self.nodes.contains_key(&left) || !self.nodes.contains_key(&right) {
            return Err("missing registration endpoint");
        }
        for (owner, peer) in [(right, left), (left, right)] {
            let set = &mut self.nodes.get_mut(&owner).unwrap().peers;
            if !set.contains(&peer) {
                services.allocate(40);
                set.insert(peer);
            }
        }
        Ok(())
    }
    /// Copies parameters and registers against the source's peers in address order;
    /// the source wrapper itself is not an implicit peer of the new wrapper.
    pub fn copy(
        &mut self,
        id: u64,
        source: u64,
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        let node = self
            .nodes
            .get(&source)
            .ok_or("missing copy source")?
            .clone();
        if !matches!(node.kind, Kind::Wrapper { .. }) {
            return Err("wrapper copy source required");
        }
        self.insert(id, node.kind, services)?;
        for peer in node.peers {
            self.bind(id, peer, services)?;
        }
        Ok(())
    }
    /// Base teardown after any derived callable destruction. Null-callable profile
    /// only: caller must not use this as proof of arbitrary callable lifetime.
    pub fn destroy(&mut self, id: u64, services: &mut impl Services) -> Result<(), &'static str> {
        let peers = self
            .nodes
            .get(&id)
            .ok_or("missing live callback identity")?
            .peers
            .clone();
        if peers.contains(&id) {
            return Err("self-bound teardown unverified");
        }
        for peer in &peers {
            if !self.nodes.contains_key(peer) {
                return Err("dangling peer");
            }
        }
        for peer in &peers {
            if self.nodes.get_mut(peer).unwrap().peers.remove(&id) {
                services.free(40);
            }
        }
        // Native destroys this object's tree nodes and its sentinel. Tree shape
        // determines pointer order; only sizes/counts are modeled here.
        for _ in &peers {
            services.free(40);
        }
        services.free(40);
        self.nodes.remove(&id);
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Heap {
        allocations: usize,
        frees: usize,
    }
    impl Services for Heap {
        fn allocate(&mut self, _: usize) {
            self.allocations += 1;
        }
        fn free(&mut self, _: usize) {
            self.frees += 1;
        }
    }
    #[test]
    fn copied_dialog_callbacks_survive_temporary_destruction_and_source_removal() {
        let mut s = State::default();
        let mut h = Heap::default();
        s.container(100, &mut h).unwrap();
        s.construct(200, Some(100), 3, 0, &mut h).unwrap();
        s.copy(300, 200, &mut h).unwrap();
        s.destroy(200, &mut h).unwrap();
        assert_eq!(s.nodes[&100].peers, BTreeSet::from([300]));
        assert_eq!(
            s.nodes[&300].kind,
            Kind::Wrapper {
                first: 3,
                second: 0
            }
        );
        s.destroy(100, &mut h).unwrap();
        assert!(s.nodes[&300].peers.is_empty());
        s.destroy(300, &mut h).unwrap();
        assert!(s.nodes.is_empty());
        assert_eq!(h.allocations, h.frees);
    }
}
