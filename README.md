<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/logos/wordmark-light.svg">
    <img alt="openXplane" src="assets/logos/wordmark-dark.svg" width="380">
  </picture>
</p>

**openXplane** is an independent open engine with gradual compatibility with X-Plane content, written in Rust.
Approach: study the original, implement each subsystem independently, and compare the results with the
reference build. The first reference is a provided X-Plane 12.

> [!IMPORTANT]
> openXplane contains no game content. It needs an original copy of X-Plane 12 and only reads from it.

![Cessna 172 SP in the openXplane viewer](site/assets/cessna.png)

Architecture and milestones, modelled on [openOMSI](https://github.com/openOMSI-Project/openOMSI):
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Current state

The ACF property loader, OBJ8 geometry reading and a static-view window of the stock Cessna on wgpu all work.
The Metal backend is verified on this Mac. There is no flight simulation yet.

From the analysis of the original EXE, typed reading of mass, maximum fuel, centre-of-gravity coordinates and
engine count has been added. Unit conversion factors keep the float32 values of the reference build.

```sh
cargo run --offline -- aircraft-info "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

Results and the limits of the confirmed behaviour: [research/ACF_LOADING.md](research/ACF_LOADING.md).

AFL 1110 is read: header, shape points, parameters and several Cl/Cd/Cm tables of 721 rows each. All 34 provided
airfoils have been checked. To print values at an angle of attack of 5°:

```sh
cargo run --offline -- airfoil-info "Xplane12/Airfoils/NACA 2412 (popular).afl" 5
```

Sampling is linear inside each table, with no regime blending and no flight-model corrections.
Research: [research/AIRFOILS.md](research/AIRFOILS.md).

The section of angle correction and fixed-grid lookup confirmed in the EXE is implemented separately.
Arguments: the angle after the previous stages, and the correction multiplier and divisor. For example:

```sh
cargo run --offline -- airfoil-eval "Xplane12/Airfoils/NACA 2412 (popular).afl" 10 1 2
```

The command shows the effective angle, the indices, the weights and the coefficients of each table. The multiplier
and divisor are still given by hand: how they are obtained for a wing, the later corrections and the integration
of forces are not implemented yet. Formulas and addresses: [research/AERO_LOOKUP.md](research/AERO_LOOKUP.md).

`airfoil-eval` also prints the normalised angle and the stall state without memory of the previous state. The
library has a mode with the original's hysteresis: it switches on above 1 and resets below 0.75 of the absolute
normalised angle. The Cl/Cd changes of the later perturbation function are not applied yet.
Evidence: [research/STALL_STATE.md](research/STALL_STATE.md).

The separate `buffet` module implements the perturbation generator and the active-stall Cl/Cd corrections. The
shared noise table and the time in seconds are passed in explicitly. The time source is established:
`sim/time/total_running_time_sec`. The `runtime` module reproduces the double storage and access through a float32
dataref. Noise initialisation and time advance are not recovered yet, so this module is not wired into
`airfoil-eval` automatically. Check with a given table:

```sh
cargo run --offline -- buffet-eval table.f32le 0 0 0 0 2 0.25 -0.1
```

The table is 262144 little-endian float32 numbers. Arguments after it: x, y, phase, time in seconds (the internal
double), Cl, Cd, Cm. Research: [research/BUFFET.md](research/BUFFET.md),
[research/RUNTIME_TIME.md](research/RUNTIME_TIME.md).

Choosing a pair of tables and blending the results by an input parameter:

```sh
cargo run --offline -- airfoil-mix "Xplane12/Airfoils/NACA 2412 (popular).afl" 5 1 1 0.75
```

The last argument is given in the scale of the AFL file's `parameters[0]`. The units of this input and its
computation in the flight model are not established yet. The command blends the implemented lookup stage; the
original blends the outputs of a fuller evaluator. Research: [research/AFL_REGIMES.md](research/AFL_REGIMES.md).

The loader keeps unknown properties, their order, repeats, string values and line numbers. It reads the
`PROPERTIES_BEGIN` / `PROPERTIES_END` section; the other sections, including `PANEL_3D`, are not interpreted yet.
The file version is kept as metadata, not as a guarantee that a whole X-Plane version is supported.

```sh
cargo run --offline -- inspect Xplane12
cargo test --offline
python3 tools/inspect_pe.py Xplane12/X-Plane.exe
python3 tools/find_acf_xrefs.py Xplane12/X-Plane.exe
```

`inspect` takes a content directory, finds ACF files recursively, checks the references to models and airfoils and
parses the AFL files it finds. Exit code: 0 - the checked references and AFL files are fine, 2 - dependencies are
missing or there are AFL errors, 1 - an ACF read or format error, or bad arguments. Textures, panels, Lua, sound
and scenery are not checked by this command yet.

## Unified profile evaluator

`profile` combines angle correction, AFL lookup, the shared stall memory for the chosen table pair, perturbations
and result blending. The `airfoil-replay` command carries the state between the rows of an input CSV:

```sh
cargo run --offline -- airfoil-replay "Xplane12/Airfoils/NACA 2412 (popular).afl" examples/profile-replay.csv research/local/runtime-snapshot/noise.f32le 0
```

An active stall needs an explicitly passed noise table. The `tools/capture_xplane_runtime.py` tool is prepared to
read it from the original on Windows; its Windows branch is not verified yet. Without an active stall, `-` can be
passed instead of the table. The evaluator creates no forces and moves no aircraft; flight-model inputs are given
explicitly. Instructions and confirmation of the state order:
[research/PROFILE_PIPELINE.md](research/PROFILE_PIPELINE.md).

## Cessna viewer

```sh
cargo run --offline -- view "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

Left mouse button and movement rotate the camera; the wheel zooms; `R` resets; `Esc` closes. The window redraws
on camera and size changes, not continuously in the background.

To save an image without a window:

```sh
cargo run --offline -- render "Xplane12/Cessna 172 SP/Cessna_172SP.acf" /tmp/cessna.png
```

Diagnostics of all attached OBJ files:

```sh
cargo run --offline -- mesh-info "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

`mesh-info` continues after errors in individual objects. It returns 2 if a file could not be read or parsed or a
base texture is missing. Unsupported commands are listed separately and do not by themselves mean a geometry error.

The viewer uses seven objects: fuselage, landing gear, propeller, wings, front and rear seats, and the outer glass.
All animation datarefs are 0. This is the chosen set of geometry for the first view, not a reproduction of the
aircraft state. VT/IDX/IDX10/TRIS, the nested transform stack, rotation and translation keys, show/hide, a DDS
lookup in place of the declared PNG, base colour, lighting and approximate glass transparency work. 4x MSAA is
supported.

Limitations: no normal/lit maps, PBR, mipmaps, mirrors, instrument screens, clickable controls or simulation.
Geometry is still double-sided; glass is sorted per object; LOD is fixed to the first set; non-zero ACF attachment
poses are rejected. All attached OBJ files of the three provided Cessna variants pass geometry parsing; this does
not mean all their commands or the full display of the aircraft are supported.
Protocol: [research/VIEWER.md](research/VIEWER.md).

Installation layout. A standard X-Plane folder (`Aircraft/`, `Airfoils/`, `Custom Data/`, `Custom Scenery/`,
`Global Scenery/`, `Resources/`) is supported with its folder precedence: `Custom Data/` before
`Resources/default data/`, scenery packs from `scenery_packs.ini` before Global Airports before the default
`apt.dat`, an aircraft's own `airfoils/` before the shared `Airfoils/`. Evidence and which rules are only
documentation-based: [research/INSTALL_LAYOUT.md](research/INSTALL_LAYOUT.md).

The minimal layout used while only a few files were provided is still accepted:

```text
Xplane12/
├── X-Plane.exe
├── Cessna 172 SP/
├── Airfoils/
└── Earth nav data/
    ├── apt.dat
    └── earth_nav.dat
```

The content directory and local reports are excluded from Git. The engine must read user content separately from
its own code. The research tools do not modify the original files.

## Datarefs

```sh
cargo run --offline -- datarefs "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
python3 tools/extract_datarefs.py Xplane12/X-Plane.exe --prefix sim/aircraft/weight/
```

`datarefs` registers the datarefs whose backing ACF fields are confirmed in the original (empty, maximum and fuel
mass, CG Y/Z, engine count) with the type and writability of the reference build. `extract_datarefs.py` lists all
5503 registration records of the original's table. Layout, evidence and limits:
[research/DATAREFS.md](research/DATAREFS.md).

## ACF loader schema

```sh
python3 tools/extract_acf_schema.py Xplane12/X-Plane.exe > research/local/acf-schema.tsv
```

Lists the 1117 ACF property reads the original's loader performs in a recognisable shape, with the field offset,
reader type and unit coefficient (wing geometry, part positions, engine fields). Evidence and limits:
[research/ACF_SCHEMA.md](research/ACF_SCHEMA.md).

## Commands

```sh
python3 tools/extract_commands.py Xplane12/X-Plane.exe > research/local/commands.tsv
cargo run --offline -- commands research/local/commands.tsv sim/flight_controls/flaps
```

The original's command table (3012 built-in commands) is enumerated by the tool, and `CommandRegistry` loads it
and runs begin/continue/end phases through handler chains. Layout, evidence and limits:
[research/COMMANDS.md](research/COMMANDS.md).

## Airports (apt.dat)

```sh
cargo run --offline -- airport-info "Xplane12/Earth nav data/apt.dat" KSEA
cargo run --offline -- airport-info Xplane12 KSEA   # whole installation, priority order
```

Reads one airport as a stream: the header, the metadata (1302) and the land runways (100). The other rows are only
counted. Exit code 2 if the ICAO code is not found. Limits and verification:
[research/APT_DAT.md](research/APT_DAT.md).

## Discord

While the viewer is open, Discord Rich Presence can show what you are looking at. It is off until you set a
Discord application id in `OPENXPLANE_DISCORD_APP_ID`; setup and limits: [docs/DISCORD.md](docs/DISCORD.md). The
website gets a Discord button as soon as an invite link is set in `site/app.js`.

## Roadmap

1. Continue analysing the original EXE's loaders and confirm the ACF/OBJ rules.
2. Extend OBJ8: instruments, materials, normal/lit maps, dynamic animations.
3. Datarefs, commands and the runtime of the stock Cessna's Lua scripts.
4. Application of the AFL tables, ACF parameters, engines, landing gear and aerodynamics, with measurements in
   the original.
5. apt.dat, DSF and the library dependencies of one test airport.

Support for X-Plane 10/11 and third-party binary plugins comes in later stages. There is no timeline for full
compatibility yet.

Results of the first investigation: [research/BASELINE.md](research/BASELINE.md).

## License

MIT, see [LICENSE](LICENSE). X-Plane is a trademark of its respective owner. openXplane is an independent
project and is not affiliated with it.
