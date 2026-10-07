//! Airfoil tables and the evaluation chain recovered from the reference build: the AFL reader, the angle
//! correction and lookup, the stall state, the buffet perturbation, regime selection and blending, the
//! profile function, the runtime clock and the wing element function. Verified against the original machine
//! code in `crates/xp-app/tests/original_vectors.rs` (research/VERIFICATION.md).
pub mod aero;
pub mod airflow;
pub mod airfoil;
pub mod atmosphere;
pub mod attitude;
pub mod body;
pub mod buffet;
pub mod callees;
pub mod controls;
pub mod element_force;
pub mod engine;
pub mod engine_env;
pub mod flight_state;
pub mod flight_step;
pub mod forces;
pub mod fuel;
pub mod matrix;
pub mod piston;
pub mod profile;
pub mod prop;
pub mod regimes;
pub mod runtime;
pub mod scalar;
pub mod shadow;
pub mod stall;
pub mod transform;
pub mod vm;
pub mod wash;
pub mod wing_element;
