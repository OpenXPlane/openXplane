use openxplane::{
    ui_callback_links as links,
    ui_callback_list::List,
    ui_dialog_child_context as context, ui_dialog_child_root as root,
    ui_dialog_child_selection as selection, ui_dialog_child_weather as weather,
    ui_dialog_children as factory,
    ui_input_construct::{self, Block},
    ui_interface_chain,
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
fn compare(a: &str, b: &str, at: &str) {
    if a != b {
        let offset = a
            .bytes()
            .zip(b.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(a.len().min(b.len()));
        panic!(
            "{at}: mismatch at {offset}, lengths {}/{}",
            a.len(),
            b.len()
        );
    }
}
fn snapshot(s: &factory::State) -> String {
    format!(
        "{};{}",
        text(&s.bundle),
        if s.blocks.is_empty() {
            "-".to_string()
        } else {
            s.blocks
                .iter()
                .map(|(p, b)| format!("{p}:{}", text(b)))
                .collect::<Vec<_>>()
                .join("/")
        }
    )
}
fn graph(s: &links::State) -> String {
    s.nodes
        .iter()
        .map(|(p, n)| {
            let (k, a, b) = match n.kind {
                links::Kind::Container => ("B", 0, 0),
                links::Kind::Wrapper { first, second } => ("W", first, second),
            };
            format!(
                "{p}:{k}:{a}:{b}:{}",
                if n.peers.is_empty() {
                    "-".to_string()
                } else {
                    n.peers
                        .iter()
                        .map(u64::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                }
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}
struct Replay {
    events: Vec<String>,
    index: usize,
    poison: u8,
}
impl Replay {
    fn event(&mut self) -> Vec<String> {
        let e = self.events[self.index]
            .split('~')
            .map(str::to_string)
            .collect();
        self.index += 1;
        e
    }
    fn effect(&mut self, image: &mut [u8], function: u64, args: &[u64]) -> String {
        let e = self.event();
        assert_eq!(e[0], "I");
        let (f, a) = e[1].split_once(',').unwrap();
        assert_eq!(num(f), function, "event {}", self.index);
        let expected = if a == "-" {
            vec![]
        } else {
            a.split(',').map(num).collect::<Vec<_>>()
        };
        assert_eq!(args, expected);
        compare(
            &text(image),
            &e[2],
            &format!("before {function:x} event{}", self.index),
        );
        let after = hex(&e[3]);
        assert_eq!(image.len(), after.len());
        image.copy_from_slice(&after);
        e[4].clone()
    }
}
impl links::Services for Replay {
    fn allocate(&mut self, n: usize) {
        assert_eq!(self.event(), ["A".to_string(), n.to_string()]);
    }
    fn free(&mut self, n: usize) {
        assert_eq!(self.event(), ["D".to_string(), n.to_string()]);
    }
}
impl openxplane::ui_callback_list::Services for Replay {
    fn allocate_entry(&mut self, n: usize) -> Result<u64, &'static str> {
        let e = self.event();
        assert_eq!(e[0], "L");
        let (m, p) = e[1].split_once(',').unwrap();
        assert_eq!(num(m) as usize, n);
        Ok(num(p))
    }
}
struct Provider<'a> {
    replay: &'a mut Replay,
    id: u64,
}
impl Provider<'_> {
    fn allocate(&mut self, image: &[u8], n: usize) -> Result<Block, &'static str> {
        let e = self.replay.event();
        assert_eq!(e[0], "H");
        let (m, p) = e[1].split_once(',').unwrap();
        assert_eq!(num(m) as usize, n);
        compare(
            &text(image),
            &e[2],
            &format!("inner allocation {}", self.replay.index),
        );
        Ok(Block {
            address: num(p),
            bytes: vec![self.replay.poison; n],
        })
    }
    fn effect(&mut self, image: &mut [u8], function: u64, args: &[u64]) -> String {
        self.replay.effect(image, function, args)
    }
}
impl links::Services for Provider<'_> {
    fn allocate(&mut self, n: usize) {
        links::Services::allocate(self.replay, n);
    }
    fn free(&mut self, n: usize) {
        links::Services::free(self.replay, n);
    }
}
impl openxplane::ui_callback_list::Services for Provider<'_> {
    fn allocate_entry(&mut self, n: usize) -> Result<u64, &'static str> {
        openxplane::ui_callback_list::Services::allocate_entry(self.replay, n)
    }
}
impl ui_input_construct::Services for Provider<'_> {
    fn allocate(&mut self, image: &[u8], n: usize) -> Result<Block, &'static str> {
        Provider::allocate(self, image, n)
    }
}
impl root::Services for Provider<'_> {
    fn allocate(&mut self, image: &[u8], _: &[Block], n: usize) -> Result<Block, &'static str> {
        Provider::allocate(self, image, n)
    }
    fn refresh_nested(
        &mut self,
        image: &mut [u8],
        _: &mut Vec<Block>,
        id: u64,
    ) -> Result<(), &'static str> {
        self.effect(image, 0x14113a300, &[id]);
        Ok(())
    }
    fn refresh_root(
        &mut self,
        image: &mut [u8],
        _: &mut Vec<Block>,
        id: u64,
    ) -> Result<(), &'static str> {
        self.effect(image, 0x141138cf0, &[id]);
        Ok(())
    }
}
impl selection::Services for Provider<'_> {
    fn bind_property(&mut self, s: &mut selection::State, field: u64, enabled: u32, key: &str) {
        assert_eq!(key, "&m_async_apt_filter_done");
        self.effect(
            &mut s.image,
            0x140c8f750,
            &[field, enabled as u64, 0x142643100],
        );
    }
    fn engine(&mut self, s: &mut selection::State) -> u8 {
        num(&self.effect(&mut s.image, 0x1417fb320, &[0x142f2d770])) as u8
    }
    fn resource_present(&mut self, s: &mut selection::State, index: u32) -> bool {
        self.effect(&mut s.image, 0x141a64260, &[0x146124398, index as u64]);
        true
    }
    fn alternate(&mut self, s: &mut selection::State) {
        self.effect(&mut s.image, 0x14111bb90, &[self.id]);
    }
    fn selection_value(&mut self, s: &mut selection::State) -> u64 {
        num(&self.effect(&mut s.image, 0x140c80d50, &[0x145a120c0]))
    }
    fn selected_index(&mut self, s: &mut selection::State, token: u64) -> i32 {
        self.effect(&mut s.image, 0x14163b530, &[token])
            .parse::<i64>()
            .unwrap() as i32
    }
    fn changed(&mut self, s: &mut selection::State) {
        self.effect(&mut s.image, 0x14113ba10, &[self.id]);
    }
    fn reset(&mut self, s: &mut selection::State, arg: u32) {
        self.effect(&mut s.image, 0x14113a460, &[self.id, arg as u64]);
    }
    fn finish(&mut self, s: &mut selection::State) {
        self.effect(&mut s.image, 0x14113d660, &[self.id]);
    }
}
impl context::Services for Provider<'_> {
    fn refresh_context(&mut self, image: &mut [u8; 96]) -> Result<(), &'static str> {
        self.effect(image, 0x1411396d0, &[self.id]);
        Ok(())
    }
}
impl weather::Services for Provider<'_> {
    fn weather(&mut self, image: &mut [u8], off: usize, index: i32, bits: u32) {
        self.effect(
            image,
            0x141b77b60,
            &[off as u64, index as u32 as u64, bits as u64],
        );
    }
    fn altitude(&mut self, image: &mut [u8], parent: u64) -> u32 {
        num(&self.effect(image, 0x14111d6a0, &[parent])) as u32
    }
    fn selection(&mut self, image: &mut [u8], parent: u64) -> [u8; 32] {
        hex(&self.effect(image, 0x141135970, &[parent]))
            .try_into()
            .unwrap()
    }
    fn refresh(&mut self, image: &mut [u8]) {
        self.effect(image, 0x141139fe0, &[self.id]);
    }
    fn special(&mut self, image: &mut [u8]) {
        self.effect(image, 0x141149360, &[self.id]);
    }
}
struct Host {
    replay: Replay,
    callbacks: links::State,
    parents: BTreeMap<u64, List>,
    catalog: u64,
    temporaries: Vec<u64>,
    temporary_index: usize,
    head: u64,
}
impl Host {
    fn keep(s: &mut factory::State, b: Block) {
        s.blocks.insert(b.address, b.bytes);
    }
}
impl factory::Services for Host {
    fn allocate(
        &mut self,
        s: &mut factory::State,
        n: usize,
    ) -> Result<Option<Block>, &'static str> {
        let e = self.replay.event();
        assert_eq!(e[0], "O");
        let (m, p) = e[1].split_once(',').unwrap();
        assert_eq!(num(m) as usize, n);
        compare(
            &snapshot(s),
            &e[2],
            &format!("outer allocation {}", self.replay.index),
        );
        Ok(Some(Block {
            address: num(p),
            bytes: vec![self.replay.poison; n],
        }))
    }
    fn construct(
        &mut self,
        s: &mut factory::State,
        kind: u8,
        p: u64,
        args: &[u64],
    ) -> Result<u64, &'static str> {
        let e = self.replay.event();
        assert_eq!(e[0], "B");
        let f = e[1].split(',').map(num).collect::<Vec<_>>();
        assert_eq!(&f[..2], &[kind as u64, p]);
        assert_eq!(&f[2..], args);
        let mut provider = Provider {
            replay: &mut self.replay,
            id: p,
        };
        match kind {
            0 => {
                let mut state = root::State::new(vec![]);
                root::construct_factory_child(
                    s,
                    p,
                    args[0],
                    &mut state,
                    root::Context {
                        identity: p,
                        callbacks: &mut self.callbacks,
                        parents: &mut self.parents,
                    },
                    &mut provider,
                )?;
                for block in state.blocks.into_iter().filter(|b| b.bytes.len() != 40) {
                    Self::keep(s, block);
                }
            }
            1 => {
                let first = self.temporaries[self.temporary_index];
                self.temporary_index += 1;
                // Presence of a secondary target is controlled by the provider;
                // its second temporary can be absent when target+0xd0 stays null.
                let two = self.temporaries.len() == 5;
                let second = if two {
                    let t = self.temporaries[self.temporary_index];
                    self.temporary_index += 1;
                    t
                } else {
                    0x12340000
                };
                let mut state = selection::State {
                    image: vec![],
                    active_global: 0,
                    catalog_base: self.catalog,
                    catalog_values: (0..3).map(|i| 0x1122334400000000 + i).collect(),
                    interfaces: ui_interface_chain::State {
                        head: None,
                        nodes: vec![Default::default()],
                    },
                };
                let init = selection::construct_factory_child(
                    s,
                    p,
                    args[0],
                    &mut state,
                    selection::Context {
                        identity: p,
                        interface: 0,
                        addresses: &[p + 0x20],
                        temporaries: [first, second],
                        callbacks: &mut self.callbacks,
                        parents: &mut self.parents,
                    },
                    &mut provider,
                )?;
                assert_eq!(state.interfaces.head, Some(0));
                self.head = p + 0x20;
                Self::keep(s, init.base_sentinel);
                Self::keep(s, init.record_sentinel);
            }
            2 => {
                let t = self.temporaries[self.temporary_index];
                self.temporary_index += 1;
                let init = context::construct_factory_child(
                    s,
                    p,
                    args[0],
                    context::Context {
                        identity: p,
                        temporary: t,
                        callbacks: &mut self.callbacks,
                        parents: &mut self.parents,
                    },
                    &mut provider,
                )?;
                Self::keep(s, init.base_sentinel);
            }
            3 => {
                let ts = [
                    self.temporaries[self.temporary_index],
                    self.temporaries[self.temporary_index + 1],
                ];
                self.temporary_index += 2;
                let init = weather::construct_factory_child(
                    s,
                    p,
                    args[0],
                    args[1],
                    weather::Context {
                        identity: p,
                        temporaries: ts,
                        callbacks: &mut self.callbacks,
                        parents: &mut self.parents,
                    },
                    &mut provider,
                )?;
                Self::keep(s, init.base_sentinel);
            }
            _ => return Err("unexpected child kind"),
        }
        factory::publish_callback_links(s, &self.callbacks, &self.parents)?;
        let e = self.replay.event();
        assert_eq!(e[0], "E");
        compare(&snapshot(s), &e[1], &format!("child{kind} return"));
        Ok(p)
    }
    fn resource(&mut self, s: &mut factory::State, index: u32) -> Result<u64, &'static str> {
        let e = self.replay.event();
        assert_eq!(e[0], "R");
        assert_eq!(num(&e[1]), index as u64);
        compare(&snapshot(s), &e[2], "factory resource");
        Ok(num(&e[3]))
    }
    fn refresh(&mut self, s: &mut factory::State, p: u64) -> Result<(), &'static str> {
        self.replay
            .effect(s.blocks.get_mut(&p).unwrap(), 0x141139460, &[p]);
        let head = u64::from_le_bytes(s.blocks[&p][8..16].try_into().unwrap());
        self.parents.insert(p, List::new(head)?);
        Ok(())
    }
}
#[test]
#[ignore = "requires local whole factory corpus via OPENXPLANE_UI_FACTORY_COMPOSED_VECTORS"]
fn native_factory_all_four_constructors_share_receivers_and_callback_graph() {
    let corpus =
        std::fs::read_to_string(std::env::var("OPENXPLANE_UI_FACTORY_COMPOSED_VECTORS").unwrap())
            .unwrap();
    let mut count = 0;
    for row in corpus.lines().filter(|r| r.starts_with("C ")) {
        let v = row.split_whitespace().collect::<Vec<_>>();
        assert_eq!(v.len(), 10);
        assert_eq!(v[4], "->");
        let seed = num(v[1]) as u8;
        let mut host = Host {
            replay: Replay {
                events: v[9].split('|').map(str::to_string).collect(),
                index: 0,
                poison: seed ^ 0x5a,
            },
            callbacks: Default::default(),
            parents: BTreeMap::new(),
            catalog: num(v[2]),
            temporaries: v[3].split(',').map(num).collect(),
            temporary_index: 0,
            head: 0,
        };
        let mut state = factory::State::new([seed; 40]);
        state.construct(&mut host).unwrap();
        compare(&snapshot(&state), v[5], &format!("final case{count}"));
        assert_eq!(graph(&host.callbacks), v[6]);
        assert_eq!(host.head, num(v[8]));
        let expected = v[7]
            .split(';')
            .map(|r| {
                let f = r.split(':').collect::<Vec<_>>();
                (
                    num(f[0]),
                    num(f[1]),
                    if f[2] == "-" {
                        vec![]
                    } else {
                        f[2].split(',').map(num).collect::<Vec<_>>()
                    },
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(host.parents.len(), expected.len());
        for (p, head, entries) in expected {
            assert_eq!(host.parents[&p].sentinel, head);
            assert_eq!(host.parents[&p].entries, entries);
        }
        assert_eq!(host.replay.index, host.replay.events.len());
        assert_eq!(host.temporary_index, host.temporaries.len());
        count += 1;
    }
    assert_eq!(count, 216);
}
