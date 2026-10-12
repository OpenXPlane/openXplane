//! Independent flight configuration state; does not claim original UI semantics.
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FlightSetup {
    pub root: PathBuf,
    pub aircraft: Vec<PathBuf>,
    pub selected: usize,
    pub airport: String,
    pub editing_airport: bool,
    pub error: Option<String>,
}
impl FlightSetup {
    pub fn new(root: &Path, aircraft: &Path, airport: &str) -> Result<Self, String> {
        let mut choices = openxplane::install::Install::open(root)?.find_acf()?;
        if !choices.iter().any(|p| p == aircraft) {
            choices.push(aircraft.to_path_buf());
        }
        let selected = choices.iter().position(|p| p == aircraft).unwrap();
        Ok(Self {
            root: root.to_path_buf(),
            aircraft: choices,
            selected,
            airport: airport.to_ascii_uppercase(),
            editing_airport: false,
            error: None,
        })
    }
    pub fn next_aircraft(&mut self) {
        self.selected = (self.selected + 1) % self.aircraft.len();
        self.error = None;
    }
    pub fn key(&mut self, key: &str) {
        match key {
            "Backspace" => {
                self.airport.pop();
            }
            "Return" | "Enter" | "Tab" => self.editing_airport = false,
            _ if key.len() == 1
                && key.as_bytes()[0].is_ascii_alphanumeric()
                && self.airport.len() < 8 =>
            {
                self.airport.push_str(&key.to_ascii_uppercase())
            }
            _ => {}
        }
        self.error = None;
    }
    pub fn valid(&self) -> bool {
        !self.airport.is_empty() && self.airport.bytes().all(|c| c.is_ascii_alphanumeric())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn editing_is_bounded_and_rejects_path_characters() {
        let mut setup = FlightSetup {
            root: PathBuf::new(),
            aircraft: vec!["a.acf".into(), "b.acf".into()],
            selected: 0,
            airport: String::new(),
            editing_airport: true,
            error: None,
        };
        for key in ["k", "s", "e", "a", "/", "."] {
            setup.key(key);
        }
        assert_eq!(setup.airport, "KSEA");
        assert!(setup.valid());
        setup.next_aircraft();
        assert_eq!(setup.selected, 1);
        setup.next_aircraft();
        assert_eq!(setup.selected, 0);
        for _ in 0..20 {
            setup.key("1");
        }
        assert_eq!(setup.airport.len(), 8);
        setup.key("Backspace");
        assert_eq!(setup.airport.len(), 7);
        setup.key("Return");
        assert!(!setup.editing_airport);
    }
}
