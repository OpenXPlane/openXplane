# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

openXplane (crate `openxplane`, Rust edition 2024): an independent open engine aiming at gradual compatibility with X-Plane content. Approach: reverse-engineer the reference X-Plane 12 build, implement subsystems independently, compare against the reference. An approximate Cessna flight model and airport flight viewer run; they are not the original flight model. README.md and `research/*.md` are written in Russian.

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
- `install`: the folder layout of an X-Plane installation and folder precedence (`Custom Data` over `Resources/default data`, scenery packs before Global Airports, aircraft `airfoils/` before `Airfoils/`); the flat provided layout is still accepted. Which rules are build-confirmed vs documentation-based is in `research/INSTALL_LAYOUT.md`.
- `discord`: Rich Presence over Discord's local IPC (std only), enabled by `OPENXPLANE_DISCORD_APP_ID`; used by `view`. Tested against a mock socket only (`docs/DISCORD.md`).
- `world`: airport ground geometry from apt.dat (projection, bezier flattening, ear-clip triangulation, layered triangles); `scene::airport` in the binary places the aircraft on a runway for `airport-render`/`airport-view`. Rules are apt.dat conventions, not build-confirmed (`research/WORLD_GEOMETRY.md`).
- `wing`, `flight`: typed wing surfaces from the ACF and an approximate rigid-body flight model (`fly-test`, `tests/flight_local.rs`). Not the original's model; `research/FLIGHT_MODEL.md` lists which parts are original and which are assumed.
- `apt`: streaming apt.dat (format 1200) reader for one airport at a time (`airport-info`); derived from observing the data file, not from the EXE (see `research/APT_DAT.md`).
- Rendering: `obj8` (OBJ8 geometry reader), `scene`, `gpu`, `viewer`, `shader.wgsl` (wgpu + winit).

## Verifying against the original

`tools/emulate_xp.py` (Unicorn) executes original functions on synthetic inputs; `tools/gen_profile_vectors.py`
writes `tests/data/profile_stage.txt` and `tests/original_vectors.rs` requires bit-identical output from the
port. Prefer extending this over hand-reading disassembly when porting a new function (`research/VERIFICATION.md`).

## Conventions specific to this repo

- Behavior is only implemented where confirmed in the original EXE; each ported stage is documented in `research/*.md` with function addresses (e.g. `0x141a412c0..0x141a41a2a`) and evidence. Update the matching research doc when adding or changing confirmed behavior, and state clearly what is not yet established.
- Do not guess unknown semantics: the ACF loader preserves unknown properties, order, duplicates, string values and line numbers; typed reads (`unique_property`, `float_property`) reject missing/duplicate/non-finite values rather than choosing a precedence.
- Preserve float32 semantics of the reference (unit-conversion constants, float32 dataref access) rather than "cleaning up" to f64.
- Inputs not yet recovered (angle multiplier/divisor, mode parameter, noise table, time advance) are passed in explicitly by callers, not defaulted.

## Interactive input

Use `pilot::CommandInput` for viewer commands: it separates held regular brakes from the maximum-brake latch
and uses the loaded model's flap detent count. `apply_command` is the stateless single-command helper.
Viewer releases are matched by physical key; focus loss/reset clears held input. These are openXplane
policies, not recovered original command handlers (`research/INPUT_HANDLING.md`).

## Native UI

`ui` rasterizes independent Alt/Esc menus into a wgpu overlay (system TTF via ab_glyph).
`flight_setup` uses the existing Install ACF catalog and an editable airport identifier. In `fly`,
StartFlight loads a complete new airport_flight, uploads replacement GPU meshes on the same device,
then commits scene/Flight; errors leave the current session active. UI artwork/layout and selection
semantics are openXplane policy, not original handlers; evidence/limits are in `research/UI.md`.
`OPENXPLANE_UI_RELOAD_SMOKE=<airport>` with `--smoke` exercises the same reload path using the next ACF.
