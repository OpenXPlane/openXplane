//! Strict local original-code replay support, reused by whole-factory tests.
#![allow(dead_code)]
use openxplane::{
    ui_fifth_refresh::{self, Services, State},
    ui_payload_move,
};
use std::collections::BTreeMap;
fn num(s: &str) -> u64 {
    s.parse().unwrap()
}
fn hex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}
fn text(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
fn memory(s: &str) -> BTreeMap<u64, Vec<u8>> {
    s.split('/')
        .map(|r| {
            let (p, b) = r.split_once(':').unwrap();
            (num(p), hex(b))
        })
        .collect()
}
fn parse(s: &str, id: u64, stack: u64) -> State {
    let f = s.split(';').collect::<Vec<_>>();
    assert_eq!(f.len(), 5);
    State {
        memory: ui_payload_move::State {
            blocks: memory(f[0]),
        },
        identity: id,
        stack,
        active_global: num(f[1]),
        resource_index: num(f[2]) as u32,
        override_enabled: num(f[3]) as u32,
        override_bits: num(f[4]) as u32,
    }
}
fn snapshot(s: &State) -> String {
    let raw = s
        .memory
        .blocks
        .iter()
        .filter(|(p, b)| !(**p <= s.stack && s.stack < **p + b.len() as u64))
        .map(|(p, b)| format!("{p}:{}", text(b)))
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "{raw};{};{};{};{}",
        s.active_global, s.resource_index, s.override_enabled, s.override_bits
    )
}
fn compare(a: &str, b: &str, label: &str) {
    if a != b {
        let p = a
            .bytes()
            .zip(b.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(a.len().min(b.len()));
        panic!("{label}: mismatch at{p}; lengths {}/{}", a.len(), b.len());
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
    expected_settings_word: Option<u64>,
}
impl Services for Replay {
    fn engine_query(&mut self, s: &mut State) -> Result<u64, &'static str> {
        self.invoke(s, 0x1417fb320, &[0x142f2d770])
    }
    fn resource_record(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        self.invoke(s, 0x141a64260, &[0x146124398, index as u64])
    }
    fn settings_record(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        self.invoke(s, 0x141a64250, &[0x1461243b0, index as u64])
    }

    fn payload_available(&mut self, s: &mut State) -> Result<bool, &'static str> {
        Ok(self.invoke(s, 0x14113b9a0, &[])? as u8 != 0)
    }
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        let f = self.events[self.position].split('~').collect::<Vec<_>>();
        self.position += 1;
        assert_eq!(f.len(), 7);
        assert_eq!(f[0], "I");
        let (name, a) = f[1].split_once(',').unwrap();
        assert_eq!(num(name), function, "event{}", self.position);
        let expected = if a == "-" {
            vec![]
        } else {
            a.split(',').map(num).collect::<Vec<_>>()
        };
        assert_eq!(args, expected);
        compare(
            &snapshot(s),
            f[2],
            &format!("before {function:x} event{}", self.position),
        );
        if f[5] != "-" {
            for (p, b) in memory(f[5]) {
                assert_eq!(
                    s.memory.read(p, b.len()).unwrap(),
                    b,
                    "read{function:x} at{p:x}"
                );
            }
        }
        if function == 0x141a64250
            && let Some(address) = self.expected_settings_word
        {
            assert_eq!(
                num(f[4]) + 0xdac,
                address,
                "settings identity must match root record"
            );
        }
        let scratch = s.memory.blocks.remove(&s.stack).unwrap();
        *s = parse(f[3], s.identity, s.stack);
        s.memory.blocks.insert(s.stack, scratch);
        if f[6] != "-" {
            for (p, b) in memory(f[6]) {
                s.memory.write(p, &b)?;
            }
        }
        Ok(num(f[4]))
    }
}
/// This adapter intentionally inherits the production gate implementation.
struct NativeReplay<'a>(&'a mut Replay);
impl Services for NativeReplay<'_> {
    fn engine_query(&mut self, s: &mut State) -> Result<u64, &'static str> {
        self.0.engine_query(s)
    }
    fn resource_record(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        self.0.resource_record(s, index)
    }
    fn settings_record(&mut self, s: &mut State, index: u32) -> Result<u64, &'static str> {
        self.0.settings_record(s, index)
    }

    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        self.0.invoke(s, function, args)
    }
}
/// Inherits production gate and both indexed lookup methods.
struct TableReplay<'a>(&'a mut Replay);
impl Services for TableReplay<'_> {
    fn engine_query(&mut self, s: &mut State) -> Result<u64, &'static str> {
        self.0.engine_query(s)
    }
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        self.0.invoke(s, function, args)
    }
}
/// Inherits the production query, predicate and both indexed lookups.
struct EngineReplay<'a>(&'a mut Replay);
impl Services for EngineReplay<'_> {
    fn invoke(&mut self, s: &mut State, function: u64, args: &[u64]) -> Result<u64, &'static str> {
        self.0.invoke(s, function, args)
    }
}
pub fn run_engine_row(
    row: &str,
    image: Option<&[u8]>,
    inherited: Option<(u64, u32)>,
    shared: &BTreeMap<u64, Vec<u8>>,
) -> State {
    run_impl(row, image, inherited, true, true, true, shared)
}
pub fn run_tables_row(
    row: &str,
    image: Option<&[u8]>,
    inherited: Option<(u64, u32)>,
    shared: &BTreeMap<u64, Vec<u8>>,
) -> State {
    run_impl(row, image, inherited, true, true, false, shared)
}
pub fn run_row(row: &str, initial_image: Option<&[u8]>, inherited: Option<(u64, u32)>) -> State {
    run_impl(
        row,
        initial_image,
        inherited,
        false,
        false,
        false,
        &BTreeMap::new(),
    )
}
pub fn run_native_row(
    row: &str,
    initial_image: Option<&[u8]>,
    inherited: Option<(u64, u32)>,
    shared: &BTreeMap<u64, Vec<u8>>,
) -> State {
    run_impl(row, initial_image, inherited, true, false, false, shared)
}
fn run_impl(
    row: &str,
    initial_image: Option<&[u8]>,
    inherited: Option<(u64, u32)>,
    native_gate: bool,
    native_tables: bool,
    native_engine: bool,
    shared: &BTreeMap<u64, Vec<u8>>,
) -> State {
    let f = row.split_whitespace().collect::<Vec<_>>();
    assert_eq!(f.len(), 8);
    assert_eq!(f[5], "->");
    let id = num(f[1]);
    let stack = num(f[2]);
    let mut s = parse(f[3], id, stack);
    for (p, bytes) in shared {
        assert_eq!(
            s.memory.read(*p, bytes.len()).unwrap(),
            *bytes,
            "shared settings at{p:x}"
        );
    }
    let mut image = s.memory.read(id, 0xb0).unwrap();
    if let Some(expected) = initial_image {
        assert_eq!(image, expected);
    }
    if let Some((active, index)) = inherited {
        assert_eq!(s.active_global, active);
        assert_eq!(s.resource_index, index);
    }
    s.memory.blocks.insert(stack, hex(f[4]));
    let mut replay = Replay {
        events: if f[7] == "-" {
            vec![]
        } else {
            f[7].split('|').map(str::to_string).collect()
        },
        position: 0,
        expected_settings_word: shared
            .iter()
            .find(|(_, bytes)| bytes.len() == 4)
            .map(|(p, _)| *p),
    };
    if native_engine {
        ui_fifth_refresh::refresh_image(&mut image, &mut s, &mut EngineReplay(&mut replay))
            .unwrap();
    } else if native_tables {
        ui_fifth_refresh::refresh_image(&mut image, &mut s, &mut TableReplay(&mut replay)).unwrap();
    } else if native_gate {
        ui_fifth_refresh::refresh_image(&mut image, &mut s, &mut NativeReplay(&mut replay))
            .unwrap();
    } else {
        ui_fifth_refresh::refresh_image(&mut image, &mut s, &mut replay).unwrap();
    }
    assert_eq!(image, s.memory.read(id, 0xb0).unwrap());
    compare(&snapshot(&s), f[6], "final fifth refresh");
    assert_eq!(replay.position, replay.events.len());
    s
}
