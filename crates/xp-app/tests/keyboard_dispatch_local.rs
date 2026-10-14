use openxplane::key_intercept::{self, State as InterceptorState};
use openxplane::ui_state::{RegisteredWindow, WindowState};
use openxplane::xplm_key_hook::{
    self, List as SdkList, Services as SdkServices, Sniffer, State as SdkState,
};
use openxplane::{
    keyboard_dispatch::{self, InterceptInput, Services},
    ui_input::{self, KeyInput},
};
use xp_dataref::key_event::{Action, Entry};

struct Replay {
    v: Vec<i64>,
    mode: i32,
    block: u8,
    trace: Vec<String>,
    windows: WindowState,
}
impl Replay {
    fn mutate(&mut self) {
        if self.v[12] >= 0 {
            self.mode = self.v[12] as i32;
        }
        if self.v[13] >= 0 {
            self.block = self.v[13] as u8;
        }
    }
}
impl Services for Replay {
    fn has_visible_capture(&mut self) -> bool {
        self.trace.push("Q".into());
        self.windows.visible_classification(2).unwrap()
    }
    fn intercept(&mut self, i: InterceptInput) -> i32 {
        self.trace.push(format!(
            "P:{}:{}:{}:{}",
            i.character, i.virtual_key, i.flags, i.phase
        ));
        let mut state = InterceptorState {
            registered: true,
            export_present: true,
            mode: self.mode,
            ..InterceptorState::default()
        };
        key_intercept::intercept(&mut state, i, 0, &mut HookReplay(self))
    }
    fn focused(&mut self, i: KeyInput) -> u8 {
        self.trace.push(format!(
            "U:{}:{}:{}",
            i.character as u32, i.virtual_key, i.flags
        ));
        let mask = self.v[8] as u32;
        ui_input::dispatch_focused(
            self.v[10] != 0,
            || mask,
            i,
            None,
            |_| {
                self.trace.push("O".into());
                self.mutate();
                self.v[3] as u8
            },
        )
        .unwrap()
    }
    fn command_gate(&mut self) -> (i32, u8) {
        (self.mode, self.block)
    }
}

struct HookReplay<'a>(&'a mut Replay);
impl key_intercept::Services for HookReplay<'_> {
    fn clock(&mut self, _: &mut InterceptorState) -> f64 {
        0.0
    }
    fn enter_scope(&mut self, _: &mut InterceptorState, _: u64) -> u64 {
        panic!("scope disabled in combined replay")
    }
    fn leave_scope(&mut self, _: &mut InterceptorState, _: u64, _: u64) {
        panic!("scope disabled in combined replay")
    }
    fn enter_hook(&mut self, _: &mut InterceptorState) -> i32 {
        0
    }
    fn leave_hook(&mut self, _: &mut InterceptorState, _: i32) {}
    fn call_hook(&mut self, _: &mut InterceptorState, i: InterceptInput, refcon: u64) -> i32 {
        assert_eq!(refcon, 0);
        self.0.trace.push(format!("D:{}", i.phase));
        let mut sdk = SdkState {
            sniffers: vec![
                Sniffer {
                    deleted: 0,
                    before: 1,
                    plugin: 0,
                    refcon: 0,
                },
                Sniffer {
                    deleted: 0,
                    before: 0,
                    plugin: 0,
                    refcon: 2,
                },
            ],
            ..SdkState::default()
        };
        let mut services = SdkReplay {
            replay: self.0,
            context: openxplane::xplm_context::State {
                stack: vec![-1],
                ..openxplane::xplm_context::State::default()
            },
        };
        let result = xplm_key_hook::dispatch(&mut sdk, i, refcon, &mut services);
        assert_eq!(services.context.stack, [-1]);
        assert_eq!(
            (sdk.sniffer_iteration.depth, sdk.sniffer_iteration.pending),
            (0, 0)
        );
        assert_eq!(
            (sdk.hotkey_iteration.depth, sdk.hotkey_iteration.pending),
            (0, 0)
        );
        result as i32
    }
}

struct SdkReplay<'a> {
    replay: &'a mut Replay,
    context: openxplane::xplm_context::State,
}
impl SdkServices for SdkReplay<'_> {
    fn validate(&mut self, _: u8) {}
    fn enter(&mut self, _: &mut SdkState, plugin: i32) -> bool {
        assert_eq!(plugin, 0);
        self.context.enter(plugin);
        self.context.active(&mut ContextBoundary).unwrap()
    }
    fn leave(&mut self, _: &mut SdkState, _: i32) {
        self.context.leave(&mut ContextBoundary).unwrap();
    }
    fn sniff(&mut self, _: &mut SdkState, _: usize, i: InterceptInput, refcon: u64) -> i32 {
        assert!(refcon == 0 || refcon == 2);
        self.replay.trace.push(format!(
            "N:{}:{}:{}:{refcon}",
            i.character, i.virtual_key, i.flags
        ));
        self.replay.mutate();
        self.replay.v[if refcon == 0 { 1 } else { 2 }] as i32
    }
    fn hotkey(&mut self, _: &mut SdkState, _: usize, _: u64) {
        panic!("no hotkeys in paired replay")
    }
    fn window(&mut self, _: &mut SdkState, _: u64, _: InterceptInput, _: u64) {
        panic!("no plugin window in paired replay")
    }
    fn flush(&mut self, _: &mut SdkState, _: SdkList, _: u8) {
        panic!("no deferred list changes in paired replay")
    }
}

struct ContextBoundary;
impl openxplane::xplm_context::Services for ContextBoundary {
    fn validate_enabled(&mut self, _: &mut openxplane::xplm_context::State) {
        panic!("plugin zero bypasses enabled export")
    }
    fn ensure_loaded(&mut self, _: &mut openxplane::xplm_context::State, _: i32) {
        panic!("plugin zero bypasses lazy load")
    }
    fn unload(&mut self, _: &mut openxplane::xplm_context::State, _: i32) {
        panic!("no pending unload")
    }
    fn notify_unload(&mut self, _: &mut openxplane::xplm_context::State) {
        panic!("no pending unload")
    }
}

#[test]
#[ignore = "requires local paired EXE/DLL keyboard replay; set OPENXPLANE_KEYBOARD_DISPATCH_VECTORS"]
fn native_keyboard_gates_ui_and_binding_transitions_match() {
    let text = std::fs::read_to_string(
        std::env::var_os("OPENXPLANE_KEYBOARD_DISPATCH_VECTORS")
            .expect("set OPENXPLANE_KEYBOARD_DISPATCH_VECTORS"),
    )
    .unwrap();
    assert!(
        text.contains(
            "# sdk_sha256 706c6f1fd856c1a8c121edac2c8df12e65603e821e3f786ee8bde72543c1401f"
        ),
        "requires paired EXE/DLL corpus"
    );
    assert!(
        text.contains(
            "# reference_sha256 2ca099b7c6b218e78d02bdbe298be2aebf670b67c5adff827449b065e1107936"
        ),
        "requires baseline EXE corpus"
    );
    let mut count = 0;
    for line in text.lines().filter(|l| l.starts_with("K ")) {
        let (input, output) = line.split_once(" -> ").unwrap();
        let v: Vec<i64> = input
            .split_whitespace()
            .skip(1)
            .map(|s| s.parse().unwrap())
            .collect();
        let event = KeyInput {
            character: v[6] as i32,
            virtual_key: v[7] as u32,
            flags: v[9] as u32,
        };
        let mut suppression = v[11] as i32;
        let mut entries = [0, 1, -1]
            .into_iter()
            .zip([3, 29, 30])
            .map(|(active, command)| Entry {
                key: event.virtual_key,
                modifiers: event.flags & 7,
                active,
                command,
            })
            .collect::<Vec<_>>();
        let window = |token, classification, opacity| {
            Some(RegisteredWindow {
                token,
                classification,
                opacity_root_to_leaf: vec![opacity],
            })
        };
        let windows = WindowState {
            registered: if v[0] != 0 {
                vec![None, window(9, 2, 1.0)]
            } else {
                vec![window(9, 1, 1.0), window(10, 2, 0.0)]
            },
            ..WindowState::default()
        };
        let mut r = Replay {
            windows,
            mode: v[4] as i32,
            block: v[5] as u8,
            v,
            trace: Vec::new(),
        };
        let actions = keyboard_dispatch::dispatch(&mut entries, &mut suppression, event, &mut r);
        for action in actions {
            r.trace.push(match action {
                Action::Begin(c) => format!("B:{}", c as u32),
                Action::End(c) => format!("E:{}", c as u32),
            });
        }
        let mut fields = output.split_whitespace();
        let actual = [
            suppression as i64,
            entries[0].active as i64,
            entries[1].active as i64,
            entries[2].active as i64,
            r.mode as u32 as i64,
            r.block as i64,
        ];
        let expected: Vec<i64> = (0..6)
            .map(|_| fields.next().unwrap().parse().unwrap())
            .collect();
        assert_eq!(actual.as_slice(), expected, "case {count}");
        assert_eq!(
            r.trace,
            fields.map(str::to_string).collect::<Vec<_>>(),
            "case {count}"
        );
        count += 1;
    }
    assert_eq!(count, 2152);
}
