//! The default keyboard map of the reference build: command name, key and XPLM virtual key code.
//! The table is `assets/keymap/default_keys.tsv`, extracted by `tools/extract_keymap.py` from the
//! command table of X-Plane.exe (research/KEYMAP.md); modifiers are not part of that record.
use std::{collections::HashMap, sync::OnceLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub command: String,
    /// Key name as written by the extraction tool: `F2`, `P`, `Space`, `Numpad0`, `[`...
    pub key: String,
    /// XPLM virtual key code.
    pub code: u8,
    pub description: String,
}

const TABLE: &str = include_str!("../assets/keymap/default_keys.tsv");

/// Parses `command<TAB>key<TAB>0xCC<TAB>description` rows. Fails on a malformed row or a key that
/// two commands share.
pub fn parse(tsv: &str) -> Result<Vec<Binding>, String> {
    let mut out: Vec<Binding> = Vec::new();
    for (i, line) in tsv
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
    {
        let f: Vec<&str> = line.splitn(4, '\t').collect();
        if f.len() != 4 {
            return Err(format!(
                "keymap line {}: expected 4 tab-separated fields",
                i + 1
            ));
        }
        let code = u8::from_str_radix(f[2].trim_start_matches("0x"), 16)
            .map_err(|_| format!("keymap line {}: invalid code '{}'", i + 1, f[2]))?;
        if out.iter().any(|b| b.code == code) {
            return Err(format!(
                "keymap line {}: code {} is bound twice",
                i + 1,
                f[2]
            ));
        }
        out.push(Binding {
            command: f[0].to_string(),
            key: f[1].to_string(),
            code,
            description: f[3].to_string(),
        });
    }
    Ok(out)
}

pub fn defaults() -> &'static [Binding] {
    static BINDINGS: OnceLock<Vec<Binding>> = OnceLock::new();
    BINDINGS.get_or_init(|| parse(TABLE).expect("the embedded keymap is valid"))
}

/// The default binding of a key name (`"F2"`, `"P"`, `"Numpad3"`...).
pub fn command_for_key(key: &str) -> Option<&'static Binding> {
    static INDEX: OnceLock<HashMap<&'static str, &'static Binding>> = OnceLock::new();
    INDEX
        .get_or_init(|| defaults().iter().map(|b| (b.key.as_str(), b)).collect())
        .get(key)
        .copied()
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
        assert_eq!(defaults().len(), 71);
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
    fn parser_rejects_bad_rows_and_duplicate_keys() {
        assert!(parse("sim/a/b\tP\t0x50").is_err());
        assert!(parse("sim/a/b\tP\tzz\tx").is_err());
        assert!(parse("sim/a/b\tP\t0x50\tx\nsim/c/d\tQ\t0x50\ty").is_err());
        assert_eq!(parse("sim/a/b\tP\t0x50\tx\n").unwrap().len(), 1);
    }
}
