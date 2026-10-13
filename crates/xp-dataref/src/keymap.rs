//! The default keyboard map of the reference build: command name, key and XPLM virtual key code.
//! The table is `assets/keymap/default_keys.tsv`, extracted by `tools/extract_keymap.py` from the
//! command table of X-Plane.exe (research/KEYMAP.md), including its modifier mask.
use std::sync::OnceLock;

/// Native modifier bits: Shift=1, Alt=2, Ctrl=4. Super/Command is not mapped to Ctrl.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modifiers(u8);

impl Modifiers {
    pub const NONE: Self = Self(0);
    pub fn from_keys(shift: bool, alt: bool, ctrl: bool) -> Self {
        Self(u8::from(shift) | (u8::from(alt) << 1) | (u8::from(ctrl) << 2))
    }
    pub fn bits(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub command: String,
    /// Key name as written by the extraction tool: `F2`, `P`, `Space`, `Numpad0`, `[`...
    pub key: String,
    /// XPLM virtual key code.
    pub code: u8,
    pub modifiers: Modifiers,
    pub description: String,
}

const TABLE: &str = include_str!("../../../assets/keymap/default_keys.tsv");

/// Parses command/key/code/description/optional-modifier-mask TSV rows.
/// Original conflicting chords are preserved. Duplicate commands are rejected.
pub fn parse(tsv: &str) -> Result<Vec<Binding>, String> {
    let mut out: Vec<Binding> = Vec::new();
    for (i, line) in tsv
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let f: Vec<&str> = line.split('\t').collect();
        if !(4..=5).contains(&f.len()) {
            return Err(format!(
                "keymap line {}: expected 4 or 5 tab-separated fields",
                i + 1
            ));
        }
        let code = u8::from_str_radix(f[2].trim_start_matches("0x"), 16)
            .map_err(|_| format!("keymap line {}: invalid code '{}'", i + 1, f[2]))?;
        let mask = if f.len() == 5 {
            u8::from_str_radix(f[4].trim_start_matches("0x"), 16)
                .map_err(|_| format!("keymap line {}: invalid modifiers", i + 1))?
        } else {
            0
        };
        if mask > 7 {
            return Err(format!("keymap line {}: unknown modifier bits", i + 1));
        }
        if out.iter().any(|b| b.command == f[0]) {
            return Err(format!(
                "keymap line {}: command {} is bound twice",
                i + 1,
                f[0]
            ));
        }
        out.push(Binding {
            command: f[0].to_string(),
            key: f[1].to_string(),
            code,
            modifiers: Modifiers(mask),
            description: f[3].to_string(),
        });
    }
    Ok(out)
}

pub fn defaults() -> &'static [Binding] {
    static BINDINGS: OnceLock<Vec<Binding>> = OnceLock::new();
    BINDINGS.get_or_init(|| parse(TABLE).expect("the embedded keymap is valid"))
}

/// Every original binding for a chord, preserving native catalogue order.
/// Four chords conflict in the original table; native conflict resolution is not recovered.
pub fn bindings_for_chord(
    key: &str,
    modifiers: Modifiers,
) -> impl Iterator<Item = &'static Binding> {
    defaults()
        .iter()
        .filter(move |b| b.key == key && b.modifiers == modifiers)
}

/// A uniquely bound chord. Ambiguous chords return None rather than inventing precedence.
pub fn command_for_chord(key: &str, modifiers: Modifiers) -> Option<&'static Binding> {
    let mut found = bindings_for_chord(key, modifiers);
    let binding = found.next()?;
    if found.next().is_some() {
        None
    } else {
        Some(binding)
    }
}

/// The unique unmodified binding of a key name.
pub fn command_for_key(key: &str) -> Option<&'static Binding> {
    command_for_chord(key, Modifiers::NONE)
}

/// The default key of a command.
pub fn key_for_command(command: &str) -> Option<&'static Binding> {
    defaults().iter().find(|b| b.command == command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_embedded_table_is_valid_and_complete() {
        assert_eq!(defaults().len(), 182);
        let p = command_for_key("P").unwrap();
        assert_eq!(
            (p.command.as_str(), p.code),
            ("sim/operation/pause_toggle", 0x50)
        );
        let f2 = key_for_command("sim/engines/throttle_up").unwrap();
        assert_eq!((f2.key.as_str(), f2.code), ("F2", 0x71));
        assert_eq!(
            command_for_key("1").unwrap().command,
            "sim/flight_controls/flaps_up"
        );
        assert_eq!(
            command_for_key("[").unwrap().command,
            "sim/flight_controls/pitch_trim_down"
        );
        assert!(command_for_key("A").is_none());
    }

    #[test]
    fn function_keys_follow_the_virtual_key_sequence() {
        // F1.. are consecutive codes from 0x70, as the XPLM virtual keys are
        for n in 1..=19u8 {
            let b = command_for_key(&format!("F{n}")).unwrap_or_else(|| panic!("F{n} unbound"));
            assert_eq!(b.code, 0x6F + n);
        }
    }

    #[test]
    fn parser_rejects_bad_rows_and_duplicate_commands() {
        assert!(parse("sim/a/b\tP\t0x50").is_err());
        assert!(parse("sim/a/b\tP\tzz\tx").is_err());
        assert!(parse("sim/a/b\tP\t0x50\tx\nsim/a/b\tQ\t0x51\ty").is_err());
        assert_eq!(parse("sim/a/b\tP\t0x50\tx\n").unwrap().len(), 1);
    }
    #[test]
    fn modifiers_distinguish_native_chords_and_preserve_conflicts() {
        let shift = Modifiers::from_keys(true, false, false);
        let ctrl = Modifiers::from_keys(false, false, true);
        let alt = Modifiers::from_keys(false, true, false);
        assert_eq!(shift.bits(), 1);
        assert_eq!(alt.bits(), 2);
        assert_eq!(ctrl.bits(), 4);
        assert_eq!(
            command_for_chord("Space", shift).unwrap().command,
            "sim/operation/screenshot"
        );
        assert_eq!(
            command_for_chord("`", ctrl).unwrap().command,
            "sim/operation/toggle_settings_window"
        );
        assert_eq!(
            command_for_chord("R", alt).unwrap().command,
            "sim/replay/replay_toggle"
        );
        assert!(command_for_chord("F", ctrl).is_none());
        assert_eq!(bindings_for_chord("F", ctrl).count(), 2);
        assert_eq!(
            defaults()
                .iter()
                .filter(|b| b.modifiers == Modifiers::NONE)
                .count(),
            71
        );
        assert!(parse("sim/a/b\tP\t0x50\tx\t0x8").is_err());
        assert_eq!(
            parse("sim/a/b\tP\t0x50\tx\t0x0\nsim/c/d\tP\t0x50\ty\t0x1")
                .unwrap()
                .len(),
            2
        );
    }
}
