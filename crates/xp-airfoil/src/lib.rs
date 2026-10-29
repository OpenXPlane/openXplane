//! Airfoil tables and the evaluation chain recovered from the reference build: the AFL reader, the angle
//! correction and lookup, the stall state, the buffet perturbation, regime selection and blending, the
//! profile function, the runtime clock and the wing element function. Verified against the original machine
//! code in `crates/xp-app/tests/original_vectors.rs` (research/VERIFICATION.md).
pub mod aero;
pub mod airflow;
pub mod airfoil;
pub mod atmosphere;
pub mod attitude;
pub mod binding;
pub mod blend;
pub mod body;
pub mod buffet;
pub mod callees;
pub mod controls;
pub mod element_force;
pub mod engine;
pub mod engine_env;
pub mod engine_small;
pub mod flight_init;
pub mod flight_state;
pub mod flight_step;
pub mod forces;
pub mod fuel;
pub mod gear;
pub mod input;
pub mod limits;
pub mod mass;
pub mod matrix;
pub mod piston;
pub mod piston_native;
pub mod profile;
pub mod prop;
pub mod regimes;
pub mod runtime;
pub mod scalar;
pub mod shadow;
pub mod stall;
pub mod strut;
pub mod terrain;
pub mod tire;
pub mod transform;
pub mod vm;
pub mod wash;
pub mod weather;
pub mod wheel;
pub mod wing_element;

pub mod aircraft_bounds;
pub mod wing_dimensions;

pub mod wing_boundary;
