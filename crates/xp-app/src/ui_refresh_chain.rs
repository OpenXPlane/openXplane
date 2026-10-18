//! Compose root refresh services with recovered nested refresh and payload move.
use crate::{
    ui_input_construct::Block, ui_nested_refresh as nested, ui_payload_move as payload,
    ui_root_refresh as root,
};
use std::collections::BTreeMap;
fn q(s: &root::State, p: u64) -> Result<u64, &'static str> {
    Ok(u64::from_le_bytes(s.memory.read(p, 8)?.try_into().unwrap()))
}
fn links(s: &root::State, base: u64) -> Result<BTreeMap<u64, u64>, &'static str> {
    let mut out = BTreeMap::new();
    for (p, b) in &s.memory.blocks {
        if *p == s.identity || (*p <= s.stack && s.stack < *p + b.len() as u64) {
            continue;
        }
        for off in (0..b.len().saturating_sub(7)).step_by(8) {
            out.insert(
                *p + off as u64,
                u64::from_le_bytes(b[off..off + 8].try_into().unwrap()),
            );
        }
    }
    let begin = q(s, base + 0x140)?;
    let end = q(s, base + 0x148)?;
    if let Some(n) = end.checked_sub(begin)
        && n % 8 == 0
        && n / 8 <= 100000
    {
        for i in 0..n / 8 {
            out.insert(begin + i * 8, q(s, begin + i * 8)?);
        }
    }
    Ok(out)
}
struct Adapter<'a, S> {
    root: &'a mut root::State,
    host: &'a mut S,
    base: u64,
    temporary: u64,
    error: Option<&'static str>,
}
impl<S: root::Services> Adapter<'_, S> {
    fn publish(&mut self, s: &nested::State) -> Result<(), &'static str> {
        self.root.memory.write(self.base, &s.image)?;
        self.root.active_global = s.active_global;
        self.root.resource_index = s.resource_index;
        for (p, n) in &s.links {
            self.root.memory.write(*p, &n.to_le_bytes())?;
        }
        Ok(())
    }
    fn capture(&mut self, s: &mut nested::State) -> Result<(), &'static str> {
        s.image = self.root.memory.read(self.base, 0x158)?;
        s.active_global = self.root.active_global;
        s.resource_index = self.root.resource_index;
        s.links = links(self.root, self.base)?;
        Ok(())
    }
    fn attempt(&mut self, r: Result<(), &'static str>) {
        if let Err(e) = r {
            self.error.get_or_insert(e);
        }
    }
    fn call(&mut self, s: &mut nested::State, f: u64, a: &[u64]) -> u64 {
        if self.error.is_some() {
            return 0;
        }
        let result = self.publish(s);
        self.attempt(result);
        if self.error.is_some() {
            return 0;
        }
        let value = match self.host.invoke(self.root, f, a) {
            Ok(v) => v,
            Err(e) => {
                self.error = Some(e);
                0
            }
        };
        let result = self.capture(s);
        self.attempt(result);
        value
    }
}
struct PayloadHost<'a, S> {
    root: &'a mut root::State,
    host: &'a mut S,
}
impl<S: root::Services> PayloadHost<'_, S> {
    fn call(&mut self, m: &mut payload::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        std::mem::swap(&mut self.root.memory, m);
        let result = self.host.invoke(self.root, f, a);
        std::mem::swap(&mut self.root.memory, m);
        result
    }
}
impl<S: root::Services> payload::Services for PayloadHost<'_, S> {
    fn allocate_vector(&mut self, m: &mut payload::State, n: usize) -> Result<Block, &'static str> {
        let p = self.call(m, 0x142301130, &[n as u64])?;
        let bytes = m
            .blocks
            .remove(&p)
            .ok_or("allocator must publish fresh mapped backing")?;
        Ok(Block { address: p, bytes })
    }
    fn free(
        &mut self,
        m: &mut payload::State,
        p: u64,
        n: Option<usize>,
    ) -> Result<(), &'static str> {
        if let Some(n) = n {
            self.call(m, 0x1422e7c2c, &[p, n as u64])?;
        } else {
            self.call(m, 0x142301120, &[p])?;
        }
        Ok(())
    }
}
impl<S: root::Services> nested::Services for Adapter<'_, S> {
    fn release_item(&mut self, s: &mut nested::State, p: u64) {
        self.call(s, 0x140f59550, &[p]);
    }
    fn engine(&mut self, s: &mut nested::State) -> u8 {
        self.call(s, 0x1417fb320, &[0x142f2d770]) as u8
    }
    fn resource(&mut self, s: &mut nested::State, i: u32) -> u64 {
        self.call(s, 0x141a64260, &[0x146124398, i as u64])
    }
    fn available(&self, p: u64) -> bool {
        self.error.is_none() && q(self.root, p + 0x6188).is_ok_and(|x| x != 0)
    }
    fn assign_resource(&mut self, s: &mut nested::State, p: u64) {
        self.call(s, 0x141142ce0, &[self.base, p]);
    }
    fn settings(&mut self, s: &mut nested::State, i: u32) -> u64 {
        self.call(s, 0x141a64250, &[0x1461243b0, i as u64])
    }
    fn setting_mode(&self, p: u64) -> i32 {
        if self.error.is_some() {
            return 1;
        }
        self.root
            .memory
            .read(p + 0xdac, 4)
            .map(|b| i32::from_le_bytes(b.try_into().unwrap()))
            .unwrap_or(1)
    }
    fn temporary(&mut self, s: &mut nested::State, p: u64) -> nested::Temporary {
        self.call(s, 0x1417f7610, &[self.temporary, p]);
        let raw = match self.root.memory.read(self.temporary, 0xe0) {
            Ok(b) => b,
            Err(e) => {
                self.error.get_or_insert(e);
                vec![0; 0xe0]
            }
        };
        nested::Temporary {
            identity: self.temporary,
            raw,
        }
    }
    fn assign_payload(&mut self, s: &mut nested::State, t: &mut nested::Temporary) {
        if self.error.is_some() {
            return;
        }
        let result = self.publish(s);
        self.attempt(result);
        let result = self.root.memory.write(t.identity, &t.raw);
        self.attempt(result);
        if self.error.is_some() {
            return;
        }
        let mut memory = std::mem::take(&mut self.root.memory);
        let result = payload::assign(
            &mut memory,
            self.base + 0x40,
            t.identity,
            &mut PayloadHost {
                root: self.root,
                host: self.host,
            },
        );
        self.root.memory = memory;
        self.attempt(result);
        match self.root.memory.read(t.identity, 0xe0) {
            Ok(raw) => t.raw = raw,
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }
        let result = self.capture(s);
        self.attempt(result);
    }
    fn free_temporary_vector(&mut self, s: &mut nested::State, p: u64) {
        self.call(s, 0x142301120, &[p]);
    }
    fn destroy_optional(&mut self, s: &mut nested::State, t: &mut nested::Temporary) {
        if self.error.is_some() {
            return;
        }
        let result = self.root.memory.write(t.identity, &t.raw);
        self.attempt(result);
        self.call(s, 0x1408ab620, &[t.identity + 0x30]);
        match self.root.memory.read(t.identity, 0xe0) {
            Ok(raw) => t.raw = raw,
            Err(e) => {
                self.error.get_or_insert(e);
            }
        }
    }
    fn notify(&mut self, s: &mut nested::State, p: u64) {
        self.call(s, 0x1410c9d10, &[p]);
    }
}
/// Called from root::Services::invoke for14113a300. Scratch temporary identity
/// is caller-supplied mapped storage, not a host stack address. Valid provider
/// computation is still supplied by the host. Provider errors return partial
/// mapped state; native exception recovery is not claimed.
pub fn refresh_nested(
    root: &mut root::State,
    host: &mut impl root::Services,
    temporary: u64,
) -> Result<(), &'static str> {
    let base = root.identity + 0x268;
    let mut state = nested::State {
        image: root.memory.read(base, 0x158)?,
        active_global: root.active_global,
        resource_index: root.resource_index,
        links: links(root, base)?,
    };
    root.memory.read(temporary, 0xe0)?;
    let mut adapter = Adapter {
        root,
        host,
        base,
        temporary,
        error: None,
    };
    let result = nested::refresh(&mut state, &mut adapter);
    let publish = adapter.publish(&state);
    if let Some(e) = adapter.error {
        return Err(e);
    }
    result.and(publish)
}
