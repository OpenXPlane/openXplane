use openxplane::ui_nested_refresh::{self, Services, State, Temporary};
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
fn state(s: &str) -> State {
    let v: Vec<_> = s.split(';').collect();
    State {
        image: hex(v[0]),
        active_global: num(v[1]),
        resource_index: num(v[2]) as u32,
        links: v[3]
            .split(',')
            .map(|r| {
                let (a, b) = r.split_once(':').unwrap();
                (num(a), num(b))
            })
            .collect(),
    }
}
fn snapshot(s: &State) -> String {
    format!(
        "{};{};{};{}",
        text(&s.image),
        s.active_global,
        s.resource_index,
        s.links
            .iter()
            .map(|(p, n)| format!("{p}:{n}"))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn temp(s: &str) -> Temporary {
    let (p, b) = s.split_once(':').unwrap();
    Temporary {
        identity: num(p),
        raw: hex(b),
    }
}
fn temp_snapshot(t: &Temporary) -> String {
    format!("{}:{}", t.identity, text(&t.raw))
}
struct Replay {
    events: Vec<String>,
    position: usize,
    resources: BTreeMap<u64, bool>,
    settings: u64,
    mode: i32,
    native_payload: bool,
    identity: u64,
}
impl Replay {
    fn effect(&mut self, s: &mut State, kind: &str, args: &str) -> String {
        let e: Vec<_> = self.events[self.position]
            .split('~')
            .map(str::to_string)
            .collect();
        self.position += 1;
        assert_eq!(e[0], kind, "event {}", self.position);
        assert_eq!(e[1], args);
        assert_eq!(e[2], snapshot(s), "before {kind}");
        *s = state(&e[3]);
        e[4].clone()
    }
}
impl Services for Replay {
    fn release_item(&mut self, s: &mut State, item: u64) {
        self.effect(s, "X", &item.to_string());
    }
    fn engine(&mut self, s: &mut State) -> u8 {
        num(&self.effect(s, "E", "-")) as u8
    }
    fn resource(&mut self, s: &mut State, index: u32) -> u64 {
        num(&self.effect(s, "R", &index.to_string()))
    }
    fn available(&self, p: u64) -> bool {
        self.resources[&p]
    }
    fn assign_resource(&mut self, s: &mut State, p: u64) {
        self.effect(s, "A", &p.to_string());
    }
    fn settings(&mut self, s: &mut State, index: u32) -> u64 {
        num(&self.effect(s, "S", &index.to_string()))
    }
    fn setting_mode(&self, p: u64) -> i32 {
        assert_eq!(p, self.settings);
        self.mode
    }
    fn temporary(&mut self, s: &mut State, p: u64) -> Temporary {
        temp(&self.effect(s, "T", &p.to_string()))
    }
    fn assign_payload(&mut self, s: &mut State, t: &mut Temporary) {
        if self.native_payload {
            let e: Vec<_> = self.events[self.position]
                .split('~')
                .map(str::to_string)
                .collect();
            self.position += 1;
            assert_eq!(e[0], "P");
            assert_eq!(e[1], temp_snapshot(t));
            assert_eq!(e[2], snapshot(s));
            let mut backing = BTreeMap::new();
            openxplane::ui_payload_move::assign_nested(
                s,
                self.identity,
                t,
                &mut backing,
                &mut InlineOnly,
            )
            .unwrap();
            assert!(backing.is_empty());
            assert_eq!(snapshot(s), e[3]);
            assert_eq!(t.raw, hex(&e[4]));
        } else {
            t.raw = hex(&self.effect(s, "P", &temp_snapshot(t)));
        }
    }
    fn free_temporary_vector(&mut self, s: &mut State, p: u64) {
        self.effect(s, "F", &p.to_string());
    }
    fn destroy_optional(&mut self, s: &mut State, t: &mut Temporary) {
        t.raw = hex(&self.effect(s, "D", &temp_snapshot(t)));
    }
    fn notify(&mut self, s: &mut State, p: u64) {
        self.effect(s, "N", &p.to_string());
    }
}
#[test]
#[ignore = "requires local corpus via OPENXPLANE_UI_NESTED_REFRESH_VECTORS"]
fn whole_native_nested_refresh_mutable_vector_gates_cleanup_and_live_listeners() {
    replay_corpus("OPENXPLANE_UI_NESTED_REFRESH_VECTORS", false);
}
#[test]
#[ignore = "requires local combined native corpus via OPENXPLANE_UI_NESTED_PAYLOAD_VECTORS"]
fn whole_native_nested_refresh_with_actual_payload_assignment() {
    replay_corpus("OPENXPLANE_UI_NESTED_PAYLOAD_VECTORS", true);
}
fn replay_corpus(env: &str, native_payload: bool) {
    let data = std::fs::read_to_string(std::env::var(env).unwrap()).unwrap();
    let identity = data
        .lines()
        .find_map(|l| l.strip_prefix("# object_identity "))
        .map(num)
        .unwrap_or(0);
    let mut cases = 0;
    for row in data.lines().filter(|r| r.starts_with("C ")) {
        let v: Vec<_> = row.split_whitespace().collect();
        assert_eq!(v.len(), 9);
        assert_eq!(v[6], "->");
        let mut s = state(v[1]);
        let mut replay = Replay {
            events: v[8].split('|').map(str::to_string).collect(),
            position: 0,
            resources: v[2].split(',').map(|p| (num(p), num(v[3]) != 0)).collect(),
            settings: num(v[4]),
            mode: v[5].parse().unwrap(),
            native_payload,
            identity,
        };
        ui_nested_refresh::refresh(&mut s, &mut replay).unwrap();
        assert_eq!(snapshot(&s), v[7], "case {cases}");
        assert_eq!(replay.position, replay.events.len());
        cases += 1;
    }
    assert_eq!(cases, 432);
}

struct InlineOnly;
impl openxplane::ui_payload_move::Services for InlineOnly {
    fn allocate_vector(
        &mut self,
        _: &mut openxplane::ui_payload_move::State,
        _: usize,
    ) -> Result<openxplane::ui_input_construct::Block, &'static str> {
        panic!("inline profile has capacity8")
    }
    fn free(
        &mut self,
        _: &mut openxplane::ui_payload_move::State,
        _: u64,
        _: Option<usize>,
    ) -> Result<(), &'static str> {
        panic!("inline profile owns no heap")
    }
}
