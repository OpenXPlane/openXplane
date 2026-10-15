//! Callback receiver141149460: append a120-byte node containing a wrapper copy.
//! Native list links/count are represented exactly by stable identities; wrapper
//! connection semantics reuse ui_callback_links. Internal RB tree bytes remain opaque.
use crate::ui_callback_links;
#[derive(Clone, Debug)]
pub struct List {
    pub sentinel: u64,
    pub entries: Vec<u64>,
}
pub trait Services: ui_callback_links::Services {
    fn allocate_entry(&mut self, bytes: usize) -> Result<u64, &'static str>;
}
impl List {
    pub fn new(sentinel: u64) -> Result<Self, &'static str> {
        if sentinel == 0 {
            return Err("nonnull list sentinel required");
        }
        Ok(Self {
            sentinel,
            entries: vec![],
        })
    }
    /// Ordered (identity,next,previous) images for the sentinel and all nodes.
    pub fn links(&self) -> Vec<(u64, u64, u64)> {
        let mut result = vec![(
            self.sentinel,
            *self.entries.first().unwrap_or(&self.sentinel),
            *self.entries.last().unwrap_or(&self.sentinel),
        )];
        for (i, id) in self.entries.iter().enumerate() {
            result.push((
                *id,
                *self.entries.get(i + 1).unwrap_or(&self.sentinel),
                if i == 0 {
                    self.sentinel
                } else {
                    self.entries[i - 1]
                },
            ));
        }
        result
    }
    /// The copied wrapper lives at entry+16. Copy and peer registration complete
    /// before the count/list publication. Source custom callable must be null.
    pub fn append(
        &mut self,
        source: u64,
        callbacks: &mut ui_callback_links::State,
        services: &mut impl Services,
    ) -> Result<u64, &'static str> {
        if self.entries.len() as u64 == 0x222222222222222 {
            return Err("native list length fatal path unverified");
        }
        if !matches!(
            callbacks.nodes.get(&source).map(|n| &n.kind),
            Some(ui_callback_links::Kind::Wrapper { .. })
        ) {
            return Err("live wrapper source required");
        }
        let entry = services.allocate_entry(120)?;
        if entry == 0 || entry == self.sentinel || self.entries.contains(&entry) {
            return Err("fresh nonnull list entry required");
        }
        let wrapper = entry.checked_add(16).ok_or("wrapper identity overflow")?;
        callbacks.copy(wrapper, source, services)?;
        self.entries.push(entry);
        Ok(entry)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    struct Heap {
        next: u64,
    }
    impl ui_callback_links::Services for Heap {
        fn allocate(&mut self, _: usize) {}
        fn free(&mut self, _: usize) {}
    }
    impl Services for Heap {
        fn allocate_entry(&mut self, _: usize) -> Result<u64, &'static str> {
            let p = self.next;
            self.next += 0x100;
            Ok(p)
        }
    }
    #[test]
    fn retained_list_copy_remains_registered_after_temporary_destruction() {
        let mut h = Heap { next: 0x1000 };
        let mut c = ui_callback_links::State::default();
        c.container(100, &mut h).unwrap();
        c.construct(200, Some(100), 3, 0, &mut h).unwrap();
        let mut list = List::new(0x800).unwrap();
        let entry = list.append(200, &mut c, &mut h).unwrap();
        c.destroy(200, &mut h).unwrap();
        assert_eq!(
            list.links(),
            vec![(0x800, entry, entry), (entry, 0x800, 0x800)]
        );
        assert_eq!(
            c.nodes[&100].peers.iter().copied().collect::<Vec<_>>(),
            vec![entry + 16]
        );
        assert_eq!(
            c.nodes[&(entry + 16)].kind,
            ui_callback_links::Kind::Wrapper {
                first: 3,
                second: 0
            }
        );
    }
}
