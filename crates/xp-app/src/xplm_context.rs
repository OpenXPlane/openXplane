//! Windows XPLM plugin-context stack and activation query on initialized TLS.
//! OS TLS initialization, allocation failures and actual module load/unload bodies remain unported.
#[derive(Clone, Debug)]
pub struct State {
    pub stack: Vec<i32>,
    /// Native plugin records have stride 0x110 and enabled byte +0x88.
    pub enabled: Vec<u8>,
    /// Global 0x180068290; -1 means no pending unload.
    pub pending_unload: i32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: Vec::new(),
            enabled: Vec::new(),
            pending_unload: -1,
        }
    }
}

pub trait Services {
    fn validate_enabled(&mut self, state: &mut State);
    fn ensure_loaded(&mut self, state: &mut State, plugin: i32);
    fn unload(&mut self, state: &mut State, plugin: i32);
    fn notify_unload(&mut self, state: &mut State);
}
impl State {
    /// Constructor 0x18001cc90 appends the full signed plugin word.
    pub fn enter(&mut self, plugin: i32) {
        self.stack.push(plugin);
    }
    /// Pure result lookup of XPLMIsPluginEnabled (caller validation is a service).
    /// Preserve the raw byte; negative/out-of-range IDs return 0.
    pub fn enabled_byte(&self, plugin: i32) -> u8 {
        usize::try_from(plugin)
            .ok()
            .and_then(|i| self.enabled.get(i))
            .copied()
            .unwrap_or(0)
    }
    /// 0x18001d1d0: plugin 0 is active even when its enabled record is zero.
    /// Other IDs call the lazy-load service, then query the captured ID, not a changed top.
    pub fn active(&mut self, services: &mut impl Services) -> Result<bool, &'static str> {
        let plugin = *self.stack.last().ok_or("empty native context stack")?;
        if plugin == 0 {
            return Ok(true);
        }
        services.ensure_loaded(self, plugin);
        services.validate_enabled(self);
        Ok(self.enabled_byte(plugin) != 0)
    }
    /// Destructor 0x18001cf60 checks pending ID against the entire current stack.
    /// If absent it calls unload/notification before popping the then-current final frame.
    pub fn leave(&mut self, services: &mut impl Services) -> Result<i32, &'static str> {
        if self.stack.is_empty() {
            return Err("empty native context stack");
        }
        let pending = self.pending_unload;
        if pending != -1 && !self.stack.contains(&pending) {
            services.unload(self, pending);
            services.notify_unload(self);
        }
        self.stack
            .pop()
            .ok_or("context stack invalidated by unload service")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Effects {
        change_top: bool,
        calls: Vec<String>,
    }
    impl Services for Effects {
        fn validate_enabled(&mut self, _: &mut State) {
            self.calls.push("validate".into());
        }
        fn ensure_loaded(&mut self, state: &mut State, plugin: i32) {
            self.calls.push(format!("load{plugin}"));
            if self.change_top {
                *state.stack.last_mut().unwrap() = 0;
            }
        }
        fn unload(&mut self, _: &mut State, plugin: i32) {
            self.calls.push(format!("unload{plugin}"));
        }
        fn notify_unload(&mut self, _: &mut State) {
            self.calls.push("notify".into());
        }
    }
    #[test]
    fn system_context_bypasses_enabled_and_other_context_uses_captured_id() {
        let mut state = State {
            stack: vec![0],
            enabled: vec![0, 0],
            ..State::default()
        };
        let mut effects = Effects {
            change_top: true,
            ..Effects::default()
        };
        assert!(state.active(&mut effects).unwrap());
        assert!(effects.calls.is_empty());
        state.stack[0] = 1;
        assert!(!state.active(&mut effects).unwrap());
        assert_eq!(state.stack, [0]);
        assert_eq!(effects.calls, ["load1", "validate"]);
    }
    #[test]
    fn pending_unload_waits_until_no_frame_references_plugin() {
        let mut state = State {
            stack: vec![7],
            pending_unload: 7,
            ..State::default()
        };
        let mut effects = Effects::default();
        state.enter(0);
        state.leave(&mut effects).unwrap();
        assert_eq!(state.stack, [7]);
        assert!(effects.calls.is_empty());
        state.leave(&mut effects).unwrap();
        assert!(effects.calls.is_empty());
        state.enter(0);
        state.leave(&mut effects).unwrap();
        assert_eq!(effects.calls, ["unload7", "notify"]);
        assert!(state.stack.is_empty());
    }
}
