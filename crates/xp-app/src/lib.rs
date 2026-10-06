//! The `openxplane` facade over the workspace crates: one place to name everything the application and the
//! tests use.
pub use xp_acf::{Aircraft, Property, aircraft, reference_candidates, wing};
pub use xp_airfoil::{aero, airfoil, buffet, profile, regimes, runtime, stall, wing_element};
pub use xp_dataref::{commands, dataref, keymap};
pub use xp_discord as discord;
pub use xp_obj as obj8;
pub use xp_scenery::{apt, install, world};
pub use xp_sim::{flight, pilot};
