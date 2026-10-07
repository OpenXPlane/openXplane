//! The `openxplane` facade over the workspace crates: one place to name everything the application and the
//! tests use.
pub use xp_acf::{Aircraft, Property, aircraft, reference_candidates, wing};
pub use xp_airfoil::{
    aero, airflow, airfoil, atmosphere, attitude, body, buffet, callees, controls, element_force,
    engine, engine_env, flight_state, flight_step, forces, fuel, input, matrix, piston, profile,
    prop, regimes, runtime, scalar, shadow, stall, transform, vm, wash, wing_element,
};
pub use xp_dataref::{commands, dataref, flight_map, keymap};
pub use xp_discord as discord;
pub use xp_obj as obj8;
pub use xp_scenery::{apt, install, world};
pub use xp_sim::{flight, pilot};
