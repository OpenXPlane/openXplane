//! Airfoil tables and the evaluation chain recovered from the reference build: the AFL reader, the angle
//! correction and lookup, the stall state, the buffet perturbation, regime selection and blending, the
//! profile function, the runtime clock and the wing element function. Verified against the original machine
//! code in `crates/xp-app/tests/original_vectors.rs` (research/VERIFICATION.md).
pub mod aero;
pub mod airfoil;
pub mod buffet;
pub mod profile;
pub mod regimes;
pub mod runtime;
pub mod stall;
pub mod wing_element;
