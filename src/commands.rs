//! Command registry: named commands with begin/continue/end phases and handler chains.
//! The catalog (names, descriptions) comes from tools/extract_commands.py output; this crate
//! carries no original names. Handler order and double-begin handling are openXplane policy,
//! not behaviour confirmed in the reference build (see research/COMMANDS.md).
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Begin,
    Continue,
    End,
}

/// Returns true to let the next handler run, false to stop the chain.
pub type Handler = Box<dyn FnMut(Phase) -> bool>;

struct Command {
    description: String,
    /// High dword of the catalog's flag word; looks like a default key code, not established.
    flag_hint: u32,
    active: bool,
    handlers: Vec<Handler>,
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: BTreeMap<String, Command>,
}

fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty() || !name.contains('/') || name.chars().any(char::is_whitespace) {
        return Err(format!("invalid command name '{name}'"));
    }
    Ok(())
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        name: &str,
        description: &str,
        flag_hint: u32,
    ) -> Result<(), String> {
        check_name(name)?;
        if self.commands.contains_key(name) {
            return Err(format!("duplicate command: {name}"));
        }
        self.commands.insert(
            name.to_string(),
            Command {
                description: description.to_string(),
                flag_hint,
                active: false,
                handlers: Vec::new(),
            },
        );
        Ok(())
    }

    /// Loads `name<TAB>internal<TAB>flag<TAB>description` rows (tools/extract_commands.py).
    /// All or nothing: a malformed row or a duplicate leaves the registry unchanged.
    pub fn load_catalog(&mut self, tsv: &str) -> Result<usize, String> {
        let mut rows = Vec::new();
        for (i, line) in tsv
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let f: Vec<&str> = line.splitn(4, '\t').collect();
            if f.len() != 4 {
                return Err(format!(
                    "catalog line {}: expected 4 tab-separated fields",
                    i + 1
                ));
            }
            let flag = u32::from_str_radix(f[2].trim_start_matches("0x"), 16)
                .map_err(|_| format!("catalog line {}: invalid flag '{}'", i + 1, f[2]))?;
            check_name(f[0]).map_err(|e| format!("catalog line {}: {e}", i + 1))?;
            if self.commands.contains_key(f[0])
                || rows.iter().any(|(n, _, _): &(&str, &str, u32)| *n == f[0])
            {
                return Err(format!(
                    "catalog line {}: duplicate command {}",
                    i + 1,
                    f[0]
                ));
            }
            rows.push((f[0], f[3], flag));
        }
        let count = rows.len();
        for (name, desc, flag) in rows {
            self.register(name, desc, flag)?;
        }
        Ok(count)
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.commands.keys().map(String::as_str)
    }

    pub fn description(&self, name: &str) -> Result<&str, String> {
        Ok(&self.command(name)?.description)
    }

    pub fn flag_hint(&self, name: &str) -> Result<u32, String> {
        Ok(self.command(name)?.flag_hint)
    }

    pub fn is_active(&self, name: &str) -> Result<bool, String> {
        Ok(self.command(name)?.active)
    }

    fn command(&self, name: &str) -> Result<&Command, String> {
        self.commands
            .get(name)
            .ok_or_else(|| format!("unknown command: {name}"))
    }

    /// Handlers run in registration order.
    pub fn add_handler(&mut self, name: &str, handler: Handler) -> Result<(), String> {
        self.commands
            .get_mut(name)
            .ok_or_else(|| format!("unknown command: {name}"))?
            .handlers
            .push(handler);
        Ok(())
    }

    fn dispatch(&mut self, name: &str, phase: Phase) -> Result<(), String> {
        let c = self
            .commands
            .get_mut(name)
            .ok_or_else(|| format!("unknown command: {name}"))?;
        for h in &mut c.handlers {
            if !h(phase) {
                break;
            }
        }
        Ok(())
    }

    pub fn begin(&mut self, name: &str) -> Result<(), String> {
        if self.is_active(name)? {
            return Err(format!("command already active: {name}"));
        }
        self.dispatch(name, Phase::Begin)?;
        self.commands.get_mut(name).unwrap().active = true;
        Ok(())
    }

    pub fn continue_(&mut self, name: &str) -> Result<(), String> {
        if !self.is_active(name)? {
            return Err(format!("command not active: {name}"));
        }
        self.dispatch(name, Phase::Continue)
    }

    pub fn end(&mut self, name: &str) -> Result<(), String> {
        if !self.is_active(name)? {
            return Err(format!("command not active: {name}"));
        }
        self.dispatch(name, Phase::End)?;
        self.commands.get_mut(name).unwrap().active = false;
        Ok(())
    }

    /// Begin immediately followed by end.
    pub fn once(&mut self, name: &str) -> Result<(), String> {
        self.begin(name)?;
        self.end(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::RefCell, rc::Rc};

    fn recorder(
        log: &Rc<RefCell<Vec<(&'static str, Phase)>>>,
        tag: &'static str,
        pass: bool,
    ) -> Handler {
        let log = Rc::clone(log);
        Box::new(move |p| {
            log.borrow_mut().push((tag, p));
            pass
        })
    }

    #[test]
    fn phases_follow_begin_continue_end_and_reject_misuse() {
        let mut r = CommandRegistry::new();
        r.register("sim/test/cmd", "Test.", 0).unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        r.add_handler("sim/test/cmd", recorder(&log, "a", true))
            .unwrap();
        assert!(r.continue_("sim/test/cmd").is_err());
        assert!(r.end("sim/test/cmd").is_err());
        r.begin("sim/test/cmd").unwrap();
        assert_eq!(r.is_active("sim/test/cmd"), Ok(true));
        assert!(
            r.begin("sim/test/cmd")
                .unwrap_err()
                .contains("already active")
        );
        r.continue_("sim/test/cmd").unwrap();
        r.end("sim/test/cmd").unwrap();
        assert_eq!(r.is_active("sim/test/cmd"), Ok(false));
        assert_eq!(
            *log.borrow(),
            vec![
                ("a", Phase::Begin),
                ("a", Phase::Continue),
                ("a", Phase::End)
            ]
        );
    }

    #[test]
    fn once_runs_begin_then_end_and_a_handler_can_stop_the_chain() {
        let mut r = CommandRegistry::new();
        r.register("sim/test/cmd", "Test.", 0).unwrap();
        let log = Rc::new(RefCell::new(Vec::new()));
        r.add_handler("sim/test/cmd", recorder(&log, "first", false))
            .unwrap();
        r.add_handler("sim/test/cmd", recorder(&log, "second", true))
            .unwrap();
        r.once("sim/test/cmd").unwrap();
        assert_eq!(
            *log.borrow(),
            vec![("first", Phase::Begin), ("first", Phase::End)]
        );
        assert_eq!(r.is_active("sim/test/cmd"), Ok(false));
    }

    #[test]
    fn catalog_loading_is_all_or_nothing_and_validates() {
        let mut r = CommandRegistry::new();
        let ok = "sim/a/one\tcmnd_one\t0x0\tFirst.\nsim/a/two\tcmnd_two\t0x50\tSecond one.\n";
        assert_eq!(r.load_catalog(ok), Ok(2));
        assert_eq!(r.description("sim/a/two"), Ok("Second one."));
        assert_eq!(r.flag_hint("sim/a/two"), Ok(0x50));
        let dup = "sim/b/x\tcmnd_x\t0x0\tX.\nsim/a/one\tcmnd_one\t0x0\tFirst.\n";
        assert!(r.load_catalog(dup).unwrap_err().contains("duplicate"));
        assert_eq!(r.len(), 2);
        assert!(r.load_catalog("sim/c/y\tcmnd_y\tzz\tY.").is_err());
        assert!(r.load_catalog("sim/c/y\tcmnd_y\t0x0").is_err());
        assert!(r.load_catalog("noslash\tcmnd_n\t0x0\tN.").is_err());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn unknown_command_is_an_error() {
        let mut r = CommandRegistry::new();
        assert!(r.once("sim/nope/x").unwrap_err().contains("unknown"));
        assert!(r.add_handler("sim/nope/x", Box::new(|_| true)).is_err());
    }
}
