//! Windows SDK deferred sniffer/hotkey deletion on explicit linked records.
//! Tracking maps, diagnostics and allocation are services; initialized TLS ownership is recovered.
use crate::xplm_context;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sniffer,
    Hotkey,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rejection {
    MissingBefore,
    OwnerBefore,
    MissingAfter,
    OwnerAfter,
}
#[derive(Clone, Debug)]
pub struct Text {
    pub length: u64,
    pub capacity: u64,
    pub first: u8,
    pub data: u64,
    /// Header at data-8, read only for capacity+1 >= 0x1000.
    pub allocation_base: Option<u64>,
}
#[derive(Clone, Copy, Debug)]
pub struct Tracking {
    pub plugin: i32,
    pub flag: u8,
    pub elapsed: f32,
}
#[derive(Clone, Debug)]
pub struct Node {
    pub next: u64,
    pub previous: u64,
    pub deferred_next: u64,
    pub owner: i32,
    pub text: Option<Text>,
    pub tracking: Option<Tracking>,
    /// Retained audit record; allocator lifetime is a service, not a Rust object removal.
    pub freed: bool,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub head: u64,
    pub deferred: u64,
    pub nodes: BTreeMap<u64, Node>,
    pub context: xplm_context::State,
}
#[derive(Clone, Copy, Debug)]
pub enum Allocation {
    Node(u64),
    Text { node: u64, pointer: u64 },
}
pub trait Services {
    fn diagnostic(&mut self, state: &mut State, reason: Rejection, node: u64);
    fn notify(&mut self, state: &mut State);
    fn bad_free(&mut self, state: &mut State, node: u64);
    fn elapsed(&mut self, state: &mut State) -> f32;
    fn tracking_entry(&mut self, state: &mut State, kind: Kind, node: u64);
    fn free(&mut self, state: &mut State, allocation: Allocation, bytes: u64);
}
impl State {
    fn contains(&self, token: u64) -> Result<bool, &'static str> {
        let mut current = self.head;
        for _ in 0..=self.nodes.len() {
            if current == 0 {
                return Ok(false);
            }
            if current == token {
                return Ok(true);
            }
            current = self.nodes.get(&current).ok_or("unmapped live node")?.next;
        }
        Err("cyclic live list")
    }
    fn reject(&mut self, reason: Rejection, token: u64, services: &mut impl Services) {
        services.diagnostic(self, reason, token);
        services.notify(self);
        services.bad_free(self, token);
        // Native rescans for optional assertions here; diagnostic assertion mode is not ported.
    }
    fn free_node(
        &mut self,
        kind: Kind,
        token: u64,
        services: &mut impl Services,
    ) -> Result<(), &'static str> {
        if kind == Kind::Hotkey {
            let text = self
                .nodes
                .get(&token)
                .and_then(|n| n.text.as_ref())
                .ok_or("missing hotkey text")?;
            if text.capacity > 15 {
                let mut bytes = text.capacity.wrapping_add(1);
                let mut pointer = text.data;
                if bytes >= 0x1000 {
                    let base = text
                        .allocation_base
                        .ok_or("missing string allocation header")?;
                    bytes = bytes.wrapping_add(0x27);
                    if pointer.wrapping_sub(base).wrapping_sub(8) > 0x1f {
                        return Err("invalid native string allocation header");
                    }
                    pointer = base;
                }
                services.free(
                    self,
                    Allocation::Text {
                        node: token,
                        pointer,
                    },
                    bytes,
                );
            }
            let text = self
                .nodes
                .get_mut(&token)
                .and_then(|n| n.text.as_mut())
                .ok_or("hotkey text invalidated by allocator")?;
            text.length = 0;
            text.capacity = 15;
            text.first = 0;
        }
        services.free(
            self,
            Allocation::Node(token),
            if kind == Kind::Sniffer { 0x40 } else { 0x68 },
        );
        self.nodes
            .get_mut(&token)
            .ok_or("node invalidated by allocator")?
            .freed = true;
        Ok(())
    }
}
/// Recovered sniffer 0x1800094f0 / hotkey 0x1800091a0 deletion flow, with
/// diagnostic assertions disabled and mapped readable records. Queue production,
/// concurrent mutation, uninitialized TLS and actual allocator/tracking bodies are not modeled.
pub fn drain(
    kind: Kind,
    state: &mut State,
    services: &mut impl Services,
) -> Result<(), &'static str> {
    let mut budget = state.nodes.len();
    while state.deferred != 0 {
        if budget == 0 {
            return Err("cyclic or service-expanded deferred queue");
        }
        budget -= 1;
        let token = state.deferred;
        state.deferred = state
            .nodes
            .get(&token)
            .ok_or("unmapped deferred node")?
            .deferred_next;
        let plugin = state.context.current_id();
        if !state.contains(token)? {
            state.reject(Rejection::MissingBefore, token, services);
            continue;
        }
        if state.nodes[&token].owner != plugin && plugin != 0 {
            state.reject(Rejection::OwnerBefore, token, services);
            continue;
        }
        let plugin = state.context.current_id();
        if !state.contains(token)? {
            state.reject(Rejection::MissingAfter, token, services);
        } else if state.nodes[&token].owner != plugin && plugin != 0 {
            state.reject(Rejection::OwnerAfter, token, services);
        } else {
            let elapsed = services.elapsed(state);
            services.tracking_entry(state, kind, token);
            state
                .nodes
                .get_mut(&token)
                .ok_or("missing tracking node")?
                .tracking = Some(Tracking {
                plugin,
                flag: 0,
                elapsed,
            });
            if state.head == token {
                state.head = state.nodes[&token].next;
            }
            let previous = state.nodes[&token].previous;
            if previous != 0 {
                let next = state.nodes[&token].next;
                state
                    .nodes
                    .get_mut(&previous)
                    .ok_or("unmapped previous node")?
                    .next = next;
            }
            let next = state.nodes[&token].next;
            if next != 0 {
                let previous = state.nodes[&token].previous;
                state
                    .nodes
                    .get_mut(&next)
                    .ok_or("unmapped next node")?
                    .previous = previous;
            }
        }
        state.free_node(kind, token, services)?;
    }
    Ok(())
}
