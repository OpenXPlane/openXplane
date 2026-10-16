use openxplane::{
    ui_context_refresh::{self, Key, Services, State},
    ui_input_construct::Block,
    ui_native_string::NativeString,
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
    b.iter().map(|v| format!("{v:02x}")).collect()
}
fn links(s: &State) -> String {
    s.next
        .iter()
        .map(|(p, n)| format!("{p}={n}"))
        .collect::<Vec<_>>()
        .join(",")
}
fn state(s: &State) -> String {
    format!("{};{};{}", text(&s.image), s.active_global, links(s))
}
fn index(k: Key) -> usize {
    match k {
        Key::Zulu => 0,
        Key::Day => 1,
    }
}
struct Replay {
    events: Vec<String>,
    position: usize,
    v: Vec<u64>,
    alt: [u8; 32],
    default: [u8; 32],
    context: [u8; 16],
    numbers: [u64; 2],
    strings: [NativeString; 2],
    code: i32,
    hour: u32,
    key: usize,
    head2: u64,
    first: u64,
    last: u64,
}
impl Replay {
    fn check(&mut self, kind: &str, args: String, s: &State) {
        let v: Vec<_> = self.events[self.position].split('~').collect();
        assert_eq!(v[0], kind, "event {}", self.position);
        assert_eq!(v[1], args);
        assert_eq!(v[2], state(s), "{kind} event {}", self.position);
        self.position += 1;
    }
}
impl Services for Replay {
    fn engine(&mut self, s: &mut State) -> u8 {
        self.check("E", "-".to_string(), s);
        s.active_global = self.v[2];
        self.v[0] as u8
    }
    fn resource_present(&mut self, s: &mut State, i: u32) -> bool {
        self.check("R", i.to_string(), s);
        self.v[3] != 0
    }
    fn alternate(&mut self, s: &mut State) -> [u8; 32] {
        self.check("A", "-".to_string(), s);
        self.alt
    }
    fn manager(&mut self, s: &mut State) -> u64 {
        self.check("M", "-".to_string(), s);
        12345
    }
    fn default_context(&mut self, s: &mut State, m: u64) -> [u8; 32] {
        self.check("B", m.to_string(), s);
        self.default
    }
    fn has(&mut self, s: &mut State, k: Key) -> u8 {
        let i = index(k);
        self.check("H", i.to_string(), s);
        self.v[4 + i] as u8
    }
    fn parent_context(&mut self, s: &mut State, p: u64) -> [u8; 16] {
        self.check("C", p.to_string(), s);
        self.context
    }
    fn context_day(&mut self, s: &mut State, p: [u8; 32], c: [u8; 16]) -> i32 {
        self.check("D", format!("{},{}", text(&p), text(&c)), s);
        self.code
    }
    fn option(&mut self, s: &mut State, k: Key) -> Result<NativeString, &'static str> {
        let i = index(k);
        self.check("G", i.to_string(), s);
        self.key = i;
        Ok(self.strings[i].clone())
    }
    fn parse(&mut self, s: &mut State, k: Key, t: &NativeString) -> f64 {
        let i = index(k);
        self.check("P", format!("{i},{}", text(&t.raw)), s);
        f64::from_bits(self.numbers[i])
    }
    fn release(&mut self, s: &mut State, b: Block) -> Result<(), &'static str> {
        self.check("F", format!("{},{}", b.address, b.bytes.len()), s);
        let expected = self.strings[self.key].storage.as_ref().unwrap();
        assert_eq!(b.address, expected.address);
        assert_eq!(b.bytes, expected.bytes);
        if self.key == 1 && self.v[10] != 0 {
            s.image[0x50] = if self.v[10] == 1 { 1 } else { 255 };
            if self.v[10] == 2 {
                let p = u64::from_le_bytes(s.image[0x58..].try_into().unwrap());
                s.image[0x58..].copy_from_slice(&(p + 1).to_le_bytes());
            }
        }
        Ok(())
    }
    fn context_hour(&mut self, s: &mut State, p: [u8; 32], c: [u8; 16]) -> u32 {
        self.check("V", format!("{},{}", text(&p), text(&c)), s);
        self.hour
    }
    fn notify(&mut self, s: &mut State, wrapper: u64) {
        self.check("N", wrapper.to_string(), s);
        let p = wrapper - 16;
        if p == self.first {
            match self.v[9] {
                1 => {
                    let head = u64::from_le_bytes(s.image[8..16].try_into().unwrap());
                    s.next.insert(p, head);
                }
                2 => {
                    s.image[8..16].copy_from_slice(&self.head2.to_le_bytes());
                    s.next.insert(p, self.head2);
                }
                3 => {
                    s.next.insert(p, self.last);
                }
                _ => {}
            }
        }
        s.image[0x18] = 0xa5;
    }
}
fn parse_links(v: &str) -> BTreeMap<u64, u64> {
    v.split(',')
        .map(|r| {
            let (p, n) = r.split_once('=').unwrap();
            (num(p), num(n))
        })
        .collect()
}
fn parse_string(v: &str) -> NativeString {
    let f: Vec<_> = v.split(':').collect();
    NativeString {
        raw: hex(f[0]).try_into().unwrap(),
        storage: if f[2] == "-" {
            None
        } else {
            Some(Block {
                address: num(f[1]),
                bytes: hex(f[2]),
            })
        },
    }
}
#[test]
#[ignore = "requires original local corpus via OPENXPLANE_UI_CONTEXT_REFRESH_VECTORS"]
fn native_whole_refresh_context_cli_numeric_edges_and_mutable_listeners() {
    let data = std::fs::read_to_string(
        std::env::var("OPENXPLANE_UI_CONTEXT_REFRESH_VECTORS").expect("corpus path"),
    )
    .unwrap();
    let mut cases = 0;
    for line in data.lines().filter(|l| l.starts_with("C ")) {
        let v: Vec<_> = line.split_whitespace().collect();
        assert_eq!(v[14], "->");
        let inputs: Vec<_> = v[2].split(',').map(num).collect();
        let next = parse_links(v[8]);
        let pointers: Vec<_> = next.keys().copied().collect();
        let mut s = State {
            image: hex(v[7]).try_into().unwrap(),
            active_global: inputs[1],
            next,
        };
        let mut replay = Replay {
            events: v[18].split('|').map(str::to_string).collect(),
            position: 0,
            v: inputs,
            alt: hex(v[9]).try_into().unwrap(),
            default: hex(v[10]).try_into().unwrap(),
            context: hex(v[11]).try_into().unwrap(),
            numbers: [num(v[3]), num(v[4])],
            strings: [parse_string(v[12]), parse_string(v[13])],
            code: v[5].parse().unwrap(),
            hour: num(v[6]) as u32,
            key: 0,
            head2: pointers[1],
            first: pointers[2],
            last: pointers[5],
        };
        ui_context_refresh::refresh(&mut s, &mut replay).unwrap();
        assert_eq!(s.image.to_vec(), hex(v[15]), "case {cases}");
        assert_eq!(s.active_global, num(v[16]));
        assert_eq!(links(&s), v[17]);
        assert_eq!(replay.position, replay.events.len());
        cases += 1;
    }
    assert_eq!(cases, 972);
}
