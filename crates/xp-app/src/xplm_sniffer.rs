//! Windows SDK key-sniffer registration/unregistration on explicit list state.
//! Original allocation/validation/loading/tracking remain services; no live plugin loader.
use crate::{
    xplm_deferred::{self, Kind, Node, Rejection, Tracking},
    xplm_key_hook::Iteration,
};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub callback: u64,
    pub before: i32,
    pub refcon: u64,
}
#[derive(Clone, Copy, Debug)]
pub struct Record {
    pub binding: Binding,
    pub plugin: i32,
}
#[derive(Clone, Debug, Default)]
pub struct State {
    pub list: xplm_deferred::State,
    pub iteration: Iteration,
    pub records: BTreeMap<u64, Record>,
}
pub trait Services: xplm_deferred::Services {
    fn validate_thread(&mut self, list: &mut xplm_deferred::State);
    fn ensure_loaded(&mut self, list: &mut xplm_deferred::State, plugin: i32);
    fn allocate(&mut self, list: &mut xplm_deferred::State, bytes: u64) -> u64;
}
impl State {
    fn find(&self, binding: Binding) -> Result<Option<u64>, &'static str> {
        let mut token = self.list.head;
        for _ in 0..=self.list.nodes.len() {
            if token == 0 {
                return Ok(None);
            }
            if self
                .records
                .get(&token)
                .ok_or("missing sniffer binding")?
                .binding
                == binding
            {
                return Ok(Some(token));
            }
            token = self.list.nodes.get(&token).ok_or("unmapped sniffer")?.next;
        }
        Err("cyclic sniffer list")
    }
    /// Original 0x180008010; ready context and disabled consistency diagnostics.
    pub fn end_iteration(&mut self, services: &mut impl Services) -> Result<(), &'static str> {
        self.iteration.depth = self.iteration.depth.wrapping_sub(1).max(0);
        if self.iteration.depth == 0 && self.iteration.pending != 0 {
            xplm_deferred::drain(Kind::Sniffer, &mut self.list, services)?;
            self.iteration.pending = 0;
        }
        Ok(())
    }
}
/// XPLMRegisterKeySniffer 0x180011230. Successful allocator result required.
/// Duplicate matching includes deleted entries and happens after allocation.
pub fn register(
    state: &mut State,
    binding: Binding,
    services: &mut impl Services,
) -> Result<i32, &'static str> {
    services.validate_thread(&mut state.list);
    services.ensure_loaded(&mut state.list, -1);
    let token = services.allocate(&mut state.list, 0x40);
    if token == 0 || state.list.nodes.contains_key(&token) {
        return Err("invalid allocator token");
    }
    state.list.nodes.insert(
        token,
        Node {
            next: 0,
            previous: 0,
            deferred_next: 0,
            owner: -1,
            deleted: 0,
            text: None,
            tracking: None,
            freed: false,
        },
    );
    // 0x1800206e0 -> 0x18001d4f0 uses inherited ID with assertions disabled.
    state.records.insert(
        token,
        Record {
            binding,
            plugin: state.list.context.current_id(),
        },
    );
    state.iteration.depth = state.iteration.depth.wrapping_add(1);
    let duplicate = state.find(binding)?.is_some();
    if !duplicate {
        let plugin = state.list.context.current_id();
        state.list.nodes.get_mut(&token).unwrap().owner = plugin;
        let elapsed = services.elapsed(&mut state.list);
        services.tracking_entry(&mut state.list, Kind::Sniffer, token);
        state.list.nodes.get_mut(&token).unwrap().tracking = Some(Tracking {
            plugin,
            flag: 1,
            elapsed,
        });
        let head = state.list.head;
        state.list.nodes.get_mut(&token).unwrap().next = head;
        if head != 0 {
            state
                .list
                .nodes
                .get_mut(&head)
                .ok_or("unmapped head")?
                .previous = token;
        }
        state.list.head = token;
    }
    // Native duplicate branch does not request a free of the newly allocated record.
    state.end_iteration(services)?;
    Ok(i32::from(!duplicate))
}
/// XPLMUnregisterKeySniffer 0x1800121d0 + actual remove helper 0x18000da90.
/// Result says a matching tuple was found, even when owner/deleted checks reject removal.
pub fn unregister(
    state: &mut State,
    binding: Binding,
    services: &mut impl Services,
) -> Result<i32, &'static str> {
    services.validate_thread(&mut state.list);
    state.iteration.depth = state.iteration.depth.wrapping_add(1);
    let token = state.find(binding)?;
    if let Some(token) = token {
        let deferred = state.iteration.depth != 0;
        let plugin = state.list.context.current_id();
        if !state.list.contains(token)? || (deferred && state.list.nodes[&token].deleted != 0) {
            state.list.reject(Rejection::MissingBefore, token, services);
        } else if state.list.nodes[&token].owner != plugin && plugin != 0 {
            state.list.reject(Rejection::OwnerBefore, token, services);
        } else if deferred {
            let elapsed = services.elapsed(&mut state.list);
            services.tracking_entry(&mut state.list, Kind::Sniffer, token);
            let node = state.list.nodes.get_mut(&token).unwrap();
            node.tracking = Some(Tracking {
                plugin,
                flag: 0,
                elapsed,
            });
            node.deleted = 1;
            node.deferred_next = state.list.deferred;
            state.list.deferred = token;
        } else {
            state.list.delete_checked(Kind::Sniffer, token, services)?;
        }
        if state.iteration.depth != 0 {
            state.iteration.pending = 1;
        }
    }
    state.end_iteration(services)?;
    Ok(i32::from(token.is_some()))
}
