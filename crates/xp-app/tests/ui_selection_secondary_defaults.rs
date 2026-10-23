//! Production reset/finish gate and callback-secondary bridges use the actual bodies.
use openxplane::{
    ui_payload_move, ui_selection_finish as finish, ui_selection_initial as initial,
    ui_selection_reset as reset,
};
const OWNER: u64 = 0x10000;
const RECEIVER: u64 = 0x20000;
const BASE: u64 = 0x30000;
const HEADER: u64 = 0x70000;
const SECOND: u64 = BASE + 0x6668 + 0x6178;
fn input() -> ui_payload_move::State {
    let mut m = ui_payload_move::State::default();
    for (p, n) in [
        (OWNER, 0x2d0),
        (RECEIVER, 0x1c0),
        (0x50000, 40),
        (0x51000, 40),
        (BASE + 0x6178, 32),
        (SECOND, 32),
        (HEADER, 32),
        (0x142640504, 8),
        (0x145899fd0, 16),
        (0x146124398, 8),
        (0x142f2dc95, 1),
    ] {
        m.blocks.insert(p, vec![0; n]);
    }
    for p in [RECEIVER + 0x20, BASE + 0x6178, SECOND, HEADER] {
        m.write(p + 24, &15u64.to_le_bytes()).unwrap();
    }
    for (p, text) in [(SECOND, b"old"), (HEADER, b"new")] {
        m.write(p, text).unwrap();
        m.write(p + 16, &3u64.to_le_bytes()).unwrap();
    }
    m.write(OWNER + 0xc8, &RECEIVER.to_le_bytes()).unwrap();
    m.write(OWNER + 0xc0, &[1]).unwrap();
    m.write(RECEIVER + 0x1a0, &0x50000u64.to_le_bytes())
        .unwrap();
    m.write(0x50008, &0x51000u64.to_le_bytes()).unwrap();
    m.write(0x50019, &[1]).unwrap();
    m.write(0x51000, &0x50000u64.to_le_bytes()).unwrap();
    m.write(0x51010, &0x50000u64.to_le_bytes()).unwrap();
    m.write(0x5101c, &2i32.to_le_bytes()).unwrap();
    m.write(0x146124398, &BASE.to_le_bytes()).unwrap();
    m.write(0x142640508, &1u32.to_le_bytes()).unwrap();
    m.write(0x142f2dc95, &[1]).unwrap();
    m
}
#[derive(Default)]
struct Host {
    singletons: usize,
    queries: usize,
    compares: usize,
    headers: usize,
}
impl Host {
    fn invoke(
        &mut self,
        m: &mut ui_payload_move::State,
        f: u64,
        a: &[u64],
    ) -> Result<u64, &'static str> {
        match f {
            0x1423c0180 => {
                self.compares += 1;
                if a[2] != 0 {
                    assert_eq!(m.read(a[0], a[2] as usize)?, m.read(a[1], a[2] as usize)?);
                }
                m.write(OWNER + 0xc8, &0xdeadbeefu64.to_le_bytes())?;
                m.write(OWNER + 0xc0, &[255])?;
                Ok(0xcafe000000000000)
            }
            0x1414cc7f0 => {
                assert!(a.is_empty());
                self.singletons += 1;
                Ok(0x90000 + self.singletons as u64 * 0x100)
            }
            0x1414cedd0 => {
                self.queries += 1;
                assert_eq!(a, [0x90000 + self.queries as u64 * 0x100]);
                Ok(0x1234567800000000 | u64::from(self.queries != 2))
            }
            0x1411222e0 => {
                self.headers += 1;
                assert_eq!(a, [RECEIVER]);
                m.write(SECOND, b"new")?;
                // The cached resource must survive this table replacement.
                m.write(0x146124398, &0x80000u64.to_le_bytes())?;
                Ok(HEADER)
            }
            _ => Err("opaque secondary/primary/membership/equality endpoint must not be requested"),
        }
    }
    fn checked(&self, m: &ui_payload_move::State, compares: usize, keys: &[u64]) {
        assert_eq!(
            (self.singletons, self.queries, self.headers, self.compares),
            (3, 3, 1, compares)
        );
        assert_eq!(m.blocks.keys().copied().collect::<Vec<_>>(), keys);
        assert_eq!(m.read(OWNER + 0xc0, 1).unwrap(), [255]);
        assert_eq!(
            m.read(OWNER + 0xc8, 8).unwrap(),
            0xdeadbeefu64.to_le_bytes()
        );
    }
}
impl finish::Services for Host {
    fn invoke(&mut self, s: &mut finish::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        self.invoke(&mut s.memory, f, a)
    }
}
impl reset::Services for Host {
    fn invoke(&mut self, s: &mut reset::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        self.invoke(&mut s.memory, f, a)
    }
}
impl initial::Services for Host {
    fn invoke(&mut self, s: &mut initial::State, f: u64, a: &[u64]) -> Result<u64, &'static str> {
        self.invoke(&mut s.memory, f, a)
    }
}
#[test]
fn finish_default_gate_reaches_actual_secondary_and_preserves_live_memory() {
    let memory = input();
    let keys = memory.blocks.keys().copied().collect::<Vec<_>>();
    let mut h = Host::default();
    let mut s = finish::State {
        memory,
        identity: OWNER,
        stack: 0x100000,
    };
    assert!(finish::Services::gate(&mut h, &mut s).unwrap());
    h.checked(&s.memory, 3, &keys);
}
#[test]
fn reset_default_gate_reaches_actual_secondary_and_preserves_live_memory() {
    let memory = input();
    let keys = memory.blocks.keys().copied().collect::<Vec<_>>();
    let mut h = Host::default();
    let mut s = reset::State {
        memory,
        identity: OWNER,
        stack: 0x100000,
    };
    assert!(reset::Services::skip_reset(&mut h, &mut s).unwrap());
    h.checked(&s.memory, 3, &keys);
}
#[test]
fn callback_default_secondary_keeps_full_rax_and_cached_receiver_resource() {
    let memory = input();
    let keys = memory.blocks.keys().copied().collect::<Vec<_>>();
    let mut h = Host::default();
    let mut s = initial::State {
        memory,
        identity: OWNER,
        stack: 0x100000,
    };
    assert_eq!(
        initial::Services::secondary(&mut h, &mut s, RECEIVER).unwrap(),
        0xcafe000000000000
    );
    h.checked(&s.memory, 2, &keys);
}
