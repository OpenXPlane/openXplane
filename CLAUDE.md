# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

openXplane (crate `openxplane`, Rust edition 2024): an independent open engine aiming at gradual compatibility with X-Plane content. Approach: reverse-engineer the reference X-Plane 12 build, implement subsystems independently, compare against the reference. No flight simulation yet. README.md and `research/*.md` are written in Russian.

## Commands

All cargo commands use `--offline` (dependencies are vendored/cached; no network).

```sh
cargo build --offline
cargo test --offline                      # all tests
cargo test --offline <name>               # single test by name filter
cargo run --offline -- inspect Xplane12   # scan content dir: ACF refs to OBJ/AFL, parses AFL (exit 0 ok, 2 missing deps/AFL errors, 1 read/format/arg errors)
cargo run --offline -- view "Xplane12/Cessna 172 SP/Cessna_172SP.acf"   # wgpu window (Metal on macOS)
cargo run --offline -- aircraft-info <acf>
cargo run --offline -- airfoil-info <afl> <alpha_deg>
cargo run --offline -- airfoil-eval <afl> <alpha> <mul> <div>
cargo run --offline -- airfoil-mix <afl> <alpha> <mul> <div> <param>
cargo run --offline -- airfoil-replay <afl> examples/profile-replay.csv <noise.f32le|-> <...>
cargo run --offline -- buffet-eval <table.f32le> x y phase time cl cd cm
python3 tools/inspect_pe.py Xplane12/X-Plane.exe   # PE analysis helpers in tools/
```

See README.md for full CLI argument semantics.

## Layout and gitignored inputs

- `Xplane12/` (the reference install, including `X-Plane.exe`) and `research/local/` (disassembly dumps, captured tables) are gitignored and not distributed. Commands and some tests need `Xplane12/` present locally.
- `src/lib.rs` holds the ACF parser (`Aircraft`, `Property`) and declares modules; `src/main.rs` is the CLI dispatcher.
- Pipeline modules (aerodynamics, ported from the original EXE): `airfoil` (AFL 1110 reader) → `aero` (angle correction + fixed-grid table lookup) → `stall` (normalized angle, hysteresis: on above 1, reset below 0.75) → `buffet` (noise perturbation of Cl/Cd in active stall) → `regimes` (table-pair selection and blending) → `profile` (unified evaluator tying these together, shared stall-flag state across the table pair). `runtime` models the double-stored / float32-dataref time source.
- `dataref`: typed registry; `register_aircraft` registers the ACF-backed datarefs whose type/writability were confirmed from the EXE's registration table (`research/DATAREFS.md`, `tools/extract_datarefs.py`).
- `commands`: command registry (phases, handler chains) loading a catalog produced by `tools/extract_commands.py` (`research/COMMANDS.md`); handler order and misuse rules are openXplane policy.
- `apt`: streaming apt.dat (format 1200) reader for one airport at a time (`airport-info`); derived from observing the data file, not from the EXE (see `research/APT_DAT.md`).
- Rendering: `obj8` (OBJ8 geometry reader), `scene`, `gpu`, `viewer`, `shader.wgsl` (wgpu + winit).

## Conventions specific to this repo

- Behavior is only implemented where confirmed in the original EXE; each ported stage is documented in `research/*.md` with function addresses (e.g. `0x141a412c0..0x141a41a2a`) and evidence. Update the matching research doc when adding or changing confirmed behavior, and state clearly what is not yet established.
- Do not guess unknown semantics: the ACF loader preserves unknown properties, order, duplicates, string values and line numbers; typed reads (`unique_property`, `float_property`) reject missing/duplicate/non-finite values rather than choosing a precedence.
- Preserve float32 semantics of the reference (unit-conversion constants, float32 dataref access) rather than "cleaning up" to f64.
- Inputs not yet recovered (angle multiplier/divisor, mode parameter, noise table, time advance) are passed in explicitly by callers, not defaulted.
