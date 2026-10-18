#[path = "support/ui_selection_alternate_replay.rs"]
mod alternate_replay;
#[path = "support/ui_selection_changed_replay.rs"]
mod change_replay;
#[path = "support/context_refresh_replay.rs"]
mod context_replay;
#[path = "support/ui_fifth_replay.rs"]
mod fifth_replay;
#[path = "support/ui_root_replay.rs"]
mod root_replay;
#[path = "support/ui_weather_replay.rs"]
mod weather_replay;
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
    native_child_tables: bool,
    native_weather: bool,
    native_changed: bool,
    native_alternate: bool,
    resources: openxplane::ui_payload_move::State,
    replay: &'a mut Replay,
    id: u64,
    resource_available: bool,
    active_global: u64,
    resource_index: u32,
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
    fn resource_present(
        &mut self,
        s: &mut selection::State,
        index: u32,
    ) -> Result<bool, &'static str> {
        if self.native_child_tables {
            let event = self.replay.event();
            assert_eq!(event[0], "J");
            assert_eq!(num(&event[1]), index as u64);
            assert_eq!(hex(&event[2]), s.image);
            assert_eq!(root_replay::parse_memory(&event[3]), self.resources.blocks);
            return openxplane::ui_indexed_records::resource_present(&self.resources, index);
        }
        self.effect(&mut s.image, 0x141a64260, &[0x146124398, index as u64]);
        Ok(self.resource_available)
    }
    fn alternate(&mut self, s: &mut selection::State) {
        self.effect(&mut s.image, 0x14111bb90, &[self.id]);
    }
    fn alternate_checked(&mut self, s: &mut selection::State) -> Result<(), &'static str> {
        if !self.native_alternate {
            self.alternate(s);
            return Ok(());
        }
        let event = self.replay.event();
        assert_eq!(event[0], "P");
        let row = event[1..].join("~").replace('!', " ").replace('^', "|");
        assert_eq!(num(row.split_whitespace().nth(1).unwrap()), self.id);
        assert_eq!(self.resources.blocks.len(), 20);
        let runtime = alternate_replay::run_row(
            &row,
            Some(&s.image),
            Some((s.active_global, s.resource_index)),
            &self.resources.blocks,
        );
        s.image = runtime.memory.read(self.id, 0x2d0)?;
        s.active_global = runtime.active_global;
        s.resource_index = runtime.resource_index;
        self.active_global = runtime.active_global;
        self.resource_index = runtime.resource_index;
        for (p, b) in &mut self.resources.blocks {
            *b = runtime.memory.read(*p, b.len())?;
        }
        Ok(())
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
    fn changed_checked(
        &mut self,
        s: &mut selection::State,
        base: &mut Block,
    ) -> Result<(), &'static str> {
        if !self.native_changed {
            self.changed(s);
            return Ok(());
        }
        let event = self.replay.event();
        assert_eq!(event[0], "N");
        let row = event[1..].join("~").replace('!', " ").replace('^', "|");
        assert_eq!(num(row.split_whitespace().nth(1).unwrap()), self.id);
        let runtime = change_replay::run_row(&row, Some(&s.image), Some(base));
        s.image = runtime.memory.read(self.id, 0x2d0)?;
        base.bytes = runtime.memory.read(base.address, base.bytes.len())?;
        Ok(())
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
        if self.replay.events[self.replay.index].starts_with("U~") {
            let event = self.replay.event();
            let row = event[1..].join("~").replace('!', " ").replace('^', "|");
            let fields = row.split_whitespace().collect::<Vec<_>>();
            assert_eq!(fields.len(), if self.native_child_tables { 22 } else { 21 });
            assert_eq!(fields[14], "->");
            assert_eq!(image.as_slice(), hex(fields[7]));
            assert_eq!(self.resource_index as u64, num(fields[19]));
            let head = u64::from_le_bytes(image[8..16].try_into().unwrap());
            let mut next = context_replay::parse_links(fields[8]);
            assert_eq!(next, BTreeMap::from([(head, head)]));
            let mut replay = context_replay::Replay::from_fields(&fields, fields[18], &next);
            assert_eq!(self.active_global, replay.v[1]);
            if self.native_child_tables {
                assert_eq!(self.resources.blocks.len(), 20);
                assert_eq!(root_replay::parse_memory(fields[21]), self.resources.blocks);
                replay.resources = Some(self.resources.clone());
            }
            openxplane::ui_context_refresh::refresh_image_with_index(
                image,
                &mut self.active_global,
                &mut self.resource_index,
                &mut next,
                &mut replay,
            )?;
            assert_eq!(image.as_slice(), hex(fields[15]));
            assert_eq!(self.active_global, num(fields[16]));
            assert_eq!(next, context_replay::parse_links(fields[17]));
            assert_eq!(self.resource_index as u64, num(fields[20]));
            assert_eq!(replay.position, replay.event_count());
        } else {
            self.effect(image, 0x1411396d0, &[self.id]);
        }
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
    fn refresh_checked(&mut self, image: &mut [u8]) -> Result<(), &'static str> {
        if !self.native_weather {
            self.refresh(image);
            return Ok(());
        }
        let event = self.replay.event();
        assert_eq!(event[0], "T");
        let row = event[1..].join("~").replace('!', " ").replace('^', "|");
        assert_eq!(num(row.split_whitespace().nth(1).unwrap()), self.id);
        assert_eq!(self.resources.blocks.len(), 20);
        let runtime = weather_replay::run_row(
            &row,
            Some(image),
            Some((self.active_global, self.resource_index)),
            &self.resources.blocks,
        );
        image.copy_from_slice(&runtime.memory.read(self.id, 0x3e8)?);
        self.active_global = runtime.active_global;
        self.resource_index = runtime.resource_index;
        for (p, b) in &mut self.resources.blocks {
            *b = runtime.memory.read(*p, b.len())?;
        }
        Ok(())
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
    active_global: u64,
    resource_index: u32,
    resource_available: bool,
    native_predicate: bool,
    native_tables: bool,
    native_engine: bool,
    native_root_tables: bool,
    native_child_tables: bool,
    native_weather: bool,
    native_changed: bool,
    native_alternate: bool,
    shared_settings: BTreeMap<u64, Vec<u8>>,
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
            native_child_tables: self.native_child_tables,
            native_weather: self.native_weather,
            native_changed: self.native_changed,
            native_alternate: self.native_alternate,
            resources: openxplane::ui_payload_move::State {
                blocks: self.shared_settings.clone(),
            },
            replay: &mut self.replay,
            id: p,
            resource_available: self.resource_available,
            active_global: self.active_global,
            resource_index: self.resource_index,
        };
        match kind {
            0 if provider.replay.events[provider.replay.index].starts_with("Q~") => {
                let event = provider.replay.event();
                let row = event[1..].join("~").replace('!', " ").replace('^', "|");
                let fields = row.split_whitespace().collect::<Vec<_>>();
                assert_eq!(num(fields[1]), p);
                assert_eq!(num(fields[13]), args[0]);
                let initial = root_replay::parse_snapshot(fields[3], p, num(fields[2]));
                assert_eq!(s.blocks[&p], initial.memory.read(p, 0x3e0).unwrap());
                let (state, runtime) = if self.native_root_tables {
                    root_replay::run_constructor_tables_row(
                        &row,
                        &mut self.callbacks,
                        &mut self.parents,
                    )
                } else {
                    root_replay::run_constructor_row(&row, &mut self.callbacks, &mut self.parents)
                };
                s.blocks.insert(p, state.image);
                for block in state.blocks.into_iter().filter(|b| b.bytes.len() != 40) {
                    Self::keep(s, block);
                }
                self.active_global = runtime.active_global;
                self.resource_index = runtime.resource_index;
                self.shared_settings = runtime
                    .memory
                    .blocks
                    .iter()
                    .filter(|(_, bytes)| bytes.len() == 4)
                    .map(|(p, bytes)| (*p, bytes.clone()))
                    .collect();
                if self.native_root_tables {
                    self.shared_settings.clear();
                    use openxplane::ui_indexed_records::{self, Kind};
                    for index in [-1i32, 0, 1, 5, 20, 77, 78, 88, 89] {
                        let resource = ui_indexed_records::lookup(
                            &runtime.memory,
                            Kind::Resource,
                            index as u32 as u64,
                        )?;
                        let settings = ui_indexed_records::lookup(
                            &runtime.memory,
                            Kind::Settings,
                            index as u32 as u64,
                        )?;
                        self.shared_settings.insert(
                            resource + 0x6178,
                            runtime.memory.read(resource + 0x6178, 32)?,
                        );
                        self.shared_settings
                            .insert(settings + 0xdac, runtime.memory.read(settings + 0xdac, 4)?);
                        if index == 88 {
                            self.resource_available = u64::from_le_bytes(
                                runtime
                                    .memory
                                    .read(resource + 0x6188, 8)?
                                    .try_into()
                                    .unwrap(),
                            ) != 0;
                        }
                    }
                }
                if self.native_tables {
                    for table in [0x146124398, 0x1461243b0] {
                        self.shared_settings
                            .insert(table, runtime.memory.read(table, 8)?);
                    }
                }
                for event in fields[12].split('|') {
                    let e = event.split('~').collect::<Vec<_>>();
                    if e[0] == "I" && num(e[1].split(',').next().unwrap()) == 0x141a64250 {
                        assert!(
                            self.shared_settings.contains_key(&(num(e[4]) + 0xdac)),
                            "root settings getter must return the shared record"
                        );
                    }
                }
                for event in fields[12].split('|') {
                    let e = event.split('~').collect::<Vec<_>>();
                    if e[0] == "I" && num(e[1].split(',').next().unwrap()) == 0x141a64260 {
                        let resource = num(e[4]);
                        self.resource_available = u64::from_le_bytes(
                            runtime
                                .memory
                                .read(resource + 0x6188, 8)
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ) != 0;
                        break;
                    }
                }
            }
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
                    active_global: self.active_global,
                    resource_index: self.resource_index,
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
                if self.native_alternate {
                    self.active_global = state.active_global;
                    self.resource_index = state.resource_index;
                    self.shared_settings = provider.resources.blocks.clone();
                }
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
                self.active_global = provider.active_global;
                self.resource_index = provider.resource_index;
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
                if self.native_weather {
                    self.active_global = provider.active_global;
                    self.resource_index = provider.resource_index;
                    self.shared_settings = provider.resources.blocks.clone();
                }
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
        if self.native_tables {
            let memory = openxplane::ui_payload_move::State {
                blocks: self.shared_settings.clone(),
            };
            return openxplane::ui_indexed_records::lookup(
                &memory,
                openxplane::ui_indexed_records::Kind::Resource,
                index as u64,
            );
        }
        let e = self.replay.event();
        assert_eq!(e[0], "R");
        assert_eq!(num(&e[1]), index as u64);
        compare(&snapshot(s), &e[2], "factory resource");
        Ok(num(&e[3]))
    }
    fn refresh(&mut self, s: &mut factory::State, p: u64) -> Result<(), &'static str> {
        if self.replay.events[self.replay.index].starts_with("W~") {
            let event = self.replay.event();
            let row = event[1..].join("~").replace('!', " ").replace('^', "|");
            assert_eq!(num(row.split_whitespace().nth(1).unwrap()), p);
            let runtime = if self.native_engine {
                assert_eq!(
                    self.shared_settings.len(),
                    if self.native_root_tables { 20 } else { 3 }
                );
                fifth_replay::run_engine_row(
                    &row,
                    Some(&s.blocks[&p]),
                    Some((self.active_global, self.resource_index)),
                    &self.shared_settings,
                )
            } else if self.native_tables {
                assert_eq!(
                    self.shared_settings.len(),
                    3,
                    "root word and two table descriptors required"
                );
                fifth_replay::run_tables_row(
                    &row,
                    Some(&s.blocks[&p]),
                    Some((self.active_global, self.resource_index)),
                    &self.shared_settings,
                )
            } else if self.native_predicate {
                assert_eq!(
                    self.shared_settings.len(),
                    1,
                    "one root settings field required"
                );
                fifth_replay::run_native_row(
                    &row,
                    Some(&s.blocks[&p]),
                    Some((self.active_global, self.resource_index)),
                    &self.shared_settings,
                )
            } else {
                fifth_replay::run_row(
                    &row,
                    Some(&s.blocks[&p]),
                    Some((self.active_global, self.resource_index)),
                )
            };
            for (address, bytes) in &mut s.blocks {
                *bytes = runtime.memory.read(*address, bytes.len())?;
            }
            self.active_global = runtime.active_global;
            self.resource_index = runtime.resource_index;
        } else {
            self.replay
                .effect(s.blocks.get_mut(&p).unwrap(), 0x141139460, &[p]);
        }
        let head = u64::from_le_bytes(s.blocks[&p][8..16].try_into().unwrap());
        self.parents.insert(p, List::new(head)?);
        Ok(())
    }
}
#[test]
#[ignore = "requires local whole factory corpus via OPENXPLANE_UI_FACTORY_COMPOSED_VECTORS"]
fn native_factory_all_four_constructors_share_receivers_and_callback_graph() {
    replay_factory("OPENXPLANE_UI_FACTORY_COMPOSED_VECTORS", 216);
}
#[test]
#[ignore = "requires local factory+root corpus via OPENXPLANE_UI_FACTORY_ROOT_CHAIN_VECTORS"]
fn native_factory_with_actual_root_nested_and_payload_refresh_chain() {
    replay_factory("OPENXPLANE_UI_FACTORY_ROOT_CHAIN_VECTORS", 118);
}
#[test]
#[ignore = "requires local factory+root+context corpus via OPENXPLANE_UI_FACTORY_ROOT_CONTEXT_VECTORS"]
fn native_factory_with_actual_root_and_context_refreshes() {
    replay_factory("OPENXPLANE_UI_FACTORY_ROOT_CONTEXT_VECTORS", 118);
}
#[test]
#[ignore = "requires local complete refresh chain via OPENXPLANE_UI_FACTORY_FIFTH_CHAIN_VECTORS"]
fn native_factory_with_actual_root_context_and_fifth_refreshes() {
    replay_factory("OPENXPLANE_UI_FACTORY_FIFTH_CHAIN_VECTORS", 118);
}
#[test]
#[ignore = "requires native shared predicate corpus via OPENXPLANE_UI_FACTORY_SHARED_GATE_VECTORS"]
fn native_factory_with_actual_fifth_gate_and_shared_root_settings() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_SHARED_GATE_VECTORS",
        118,
        ReplayMode::Predicate,
    );
}
fn replay_factory(env: &str, expected_cases: usize) {
    replay_factory_mode(env, expected_cases, ReplayMode::Controlled);
}
#[test]
#[ignore = "requires native fifth table chain via OPENXPLANE_UI_FACTORY_FIFTH_TABLES_VECTORS"]
fn native_factory_with_real_fifth_record_tables() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_FIFTH_TABLES_VECTORS",
        118,
        ReplayMode::Tables,
    );
}
#[test]
#[ignore = "requires actual fifth query/table factory corpus via OPENXPLANE_UI_FACTORY_FIFTH_ENGINE_VECTORS"]
fn native_factory_with_actual_fifth_query_and_record_tables() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_FIFTH_ENGINE_VECTORS",
        118,
        ReplayMode::Engine,
    );
}
#[test]
#[ignore = "requires actual root/fifth tables via OPENXPLANE_UI_FACTORY_ROOT_TABLES_VECTORS"]
fn native_factory_with_real_root_nested_and_fifth_record_tables() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_ROOT_TABLES_VECTORS",
        118,
        ReplayMode::RootTables,
    );
}
#[test]
#[ignore = "requires actual selection/context getters via OPENXPLANE_UI_FACTORY_CHILD_TABLES_VECTORS"]
fn native_factory_with_actual_selection_and_context_resource_tables() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_CHILD_TABLES_VECTORS",
        118,
        ReplayMode::ChildTables,
    );
}
#[test]
#[ignore = "requires actual weather factory corpus via OPENXPLANE_UI_FACTORY_WEATHER_CHAIN_VECTORS"]
fn native_factory_with_actual_weather_refresh_and_all_record_tables() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_WEATHER_CHAIN_VECTORS",
        118,
        ReplayMode::Weather,
    );
}
#[test]
#[ignore = "requires actual selection changed factory corpus via OPENXPLANE_UI_FACTORY_SELECTION_CHANGED_VECTORS"]
fn native_factory_with_actual_selection_change_and_weather_chain() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_SELECTION_CHANGED_VECTORS",
        118,
        ReplayMode::Changed,
    );
}
#[test]
#[ignore = "requires whole alternate factory corpus via OPENXPLANE_UI_FACTORY_SELECTION_ALTERNATE_VECTORS"]
fn native_factory_with_actual_selection_alternate_changed_and_weather_chain() {
    replay_factory_mode(
        "OPENXPLANE_UI_FACTORY_SELECTION_ALTERNATE_VECTORS",
        118,
        ReplayMode::Alternate,
    );
}
#[derive(Clone, Copy, PartialEq, PartialOrd)]
enum ReplayMode {
    Controlled,
    Predicate,
    Tables,
    Engine,
    RootTables,
    ChildTables,
    Weather,
    Changed,
    Alternate,
}
fn replay_factory_mode(env: &str, expected_cases: usize, mode: ReplayMode) {
    let native_predicate = mode >= ReplayMode::Predicate;
    let native_tables = mode >= ReplayMode::Tables;
    let native_engine = mode >= ReplayMode::Engine;
    let native_root_tables = mode >= ReplayMode::RootTables;
    let native_child_tables = mode >= ReplayMode::ChildTables;
    let native_weather = mode >= ReplayMode::Weather;
    let native_changed = mode >= ReplayMode::Changed;
    let native_alternate = mode >= ReplayMode::Alternate;
    let corpus = std::fs::read_to_string(std::env::var(env).unwrap()).unwrap();
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
            active_global: 0,
            resource_index: 0,
            resource_available: true,
            native_predicate,
            native_tables,
            native_engine,
            native_root_tables,
            native_child_tables,
            native_weather,
            native_changed,
            native_alternate,
            shared_settings: BTreeMap::new(),
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
    assert_eq!(count, expected_cases);
}
