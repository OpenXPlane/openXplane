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

## Download

Every change to `main` is built by GitHub Actions and published as the rolling
[nightly release](https://github.com/OpenXPlane/openXplane/releases/tag/nightly):

| Platform | File |
| --- | --- |
| Windows x64 | `openXplane-<version>-windows-x64.zip`, run `openxplane.exe` |
| macOS (Apple silicon) | `openXplane-<version>-macos-arm64.zip`, run `openxplane` |
| Linux x64 | `openXplane-<version>-linux-x64.zip`, run `openxplane` |

There is no launcher yet: run it from a terminal and point it at your X-Plane 12 folder (see the commands below).

## Goals

1. **1:1 behaviour.** The formats and algorithms of X-Plane 12 behave as in the original, confirmed in the
   reference build and verified against its machine code.
2. **No original code or assets.** Nothing is copied; formats are described in the research notes.
3. **A better engine.** Rust, wgpu (Metal, Vulkan, DirectX 12), 64-bit, multithreaded loading.

## Documentation

| Document | What is in it |
| --- | --- |
| [Keyboard](docs/KEYBOARD.md) | the original's default keys and openXplane's own |
| [User guide](docs/USER_GUIDE.md) | install, first flights, data output |
| [Building](docs/BUILDING.md) | toolchain, tests, regenerating vectors |
| [FAQ](docs/FAQ.md) | common questions |
| [Releasing](docs/RELEASING.md) | versions, nightly builds |
| [Architecture](docs/ARCHITECTURE.md) | crates and roadmap |
| [Plan](docs/PLAN.md) | phased plan for a full implementation |
| [Compatibility](docs/COMPATIBILITY.md) | how close it is to the original, and how that is estimated |
| [Discord](docs/DISCORD.md) | Rich Presence |
| [Research notes](research/) | every recovered format and algorithm, with evidence |
| [Verification](research/VERIFICATION.md) | comparing ports against the original machine code |
| [Changelog](CHANGELOG.md) | what changed in each version |

## Repository layout

```
openXplane/
├── VERSION            MAJOR.MINOR of the next release (edited by hand)
├── crates/            the engine, one crate per subsystem
│   ├── xp-acf/          ACF aircraft files, typed parameters, lifting surfaces
│   ├── xp-airfoil/      AFL tables, the verified evaluation chain, the wing element function
│   ├── xp-obj/          OBJ8 model reader
│   ├── xp-scenery/      installation layout, apt.dat, airport ground geometry
│   ├── xp-dataref/      dataref and command registries, default keyboard map
│   ├── xp-sim/          the approximate flight model and pilot commands
│   ├── xp-discord/      Discord Rich Presence
│   └── xp-app/          the `openxplane` binary (viewer, flight, tools) and the tests against the original
├── tools/             research tools: emulator, extractors, vector generators, logo, scoring
├── scripts/           build scripts for every platform, version.sh
├── assets/            logos, screenshots, the keymap table
├── docs/              user documentation (also published on the website)
├── research/          research notes
├── site/              the GitHub Pages website
└── .github/workflows/ CI, nightly builds, the website
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security problems: [SECURITY.md](SECURITY.md).

<!-- compat:start -->
## How close to the original is it?

**About 41% implemented, about 28% verified identical to the original.** Early stage: it loads
the original content, draws it, and flies an approximation; it does not yet behave like X-Plane.

- *Implemented* counts work that exists in any form, including approximations (the flight model is one).
- *Verified identical* is an estimate of recovered behaviour supported by comparisons with the original code. Evidence distinguishes exact matches from controlled inputs and platform tolerances; it does not establish whole-game equivalence.

This is a self-assessed, weighted rubric, not a measurement (method and per-subsystem numbers in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md); data in [docs/compat.json](docs/compat.json), updated with
`python3 tools/compat_score.py --write`).

| Subsystem | Weight | Implemented | Verified identical | Notes |
|---|---:|---:|---:|---|
| Aircraft files (ACF) | 8 | 32% | 12% | All properties are preserved; typed: mass, CG, wings, gear, engine and controls subset. Loader schema recovered from the original; focused property subsets have strict original-code comparisons. Three original-loader replays now reach the return sentinel (13690, 13718 and 15525 tracked words). Previous fixtures were partial; two contain 424 corrected words each. These are reference results, not a count of matching ported fields. The full-object diagnostic is explicitly ignored by default, requires local reference files when requested, and reports remaining differences without asserting parity. The wing chord unit-conversion loop is enforced bit for bit on 312 synthetic original-code cases (3432 output words). Typed wings read all 48 slots and use supplied active rib chords; missing rib arrays retain a declared linear approximation. Active `_c_off_for_RIB` values are now typed without unit conversion and passed to the recovered boundary writer. The complete 20x18x3 body-grid feet conversion is recovered: 69120 synthetic SSE outputs and all 15120 supplied Cessna body-coordinate fields match exactly in debug/release. These local tests are explicitly ignored in ordinary CI; remaining full-object mismatches are still reported. |
| 3D models (OBJ8) | 7 | 35% | 5% | Geometry, nested transforms, rotation/translation animation, textures. No normal/lit maps, instruments, panels or most dynamic animation. |
| Airfoil evaluation (AFL) | 6 | 90% | 85% | All 34 airfoils; the whole profile function (selection, stall, buffet, blending, Mach factor) matches the original bit for bit on 1800 emulator cases. |
| Flight model | 28 | 81% | 72% | An approximate rigid-body model that takes off and flies. Verified against the original machine code in an emulator: the airfoil and per-element aerodynamics, the whole engine update and the propeller force function, the engine controls for every engine kind, the airflow wash, the terrain mesh probes (vertical ray and segment with moving cells), the strut, tire, wheel and body blend contact models, the weight and balance, the input binding query, and the whole flight step block by block: the wing, body and part loops, the landing gear (targets, steering, brakes, drag, strut contact), the wing and body ground contact chain, the hook, tow, float and wave forces, the plug-in force sink, the equations of motion and the integration, the position setter, the attitude, the geographic state, the 4 x 4 matrices and the Euler angles, the flight angles and the end of the step. Remaining recovery and integration work includes scenery-backed terrain and geoid queries, shared-pointer object lookups, per-strip airflow (`0x14121a9b0`), radiator update and runtime caller state; many ported blocks are not yet connected into a running model. The viewer still flies an approximate model, but now runs the recovered positive-delta coefficient branch and ordered wing grouping with recovered aspect/centroid parameters. Since the previous review, controlled original-code comparisons cover delta parameter curves, noise, target coefficients, full-element and force composition, aircraft bounds point emission, wing enable/join predicates and the 49-pass group scan. Full-element delta output matches 2099/2100 words exactly on macOS, with one platform-trigonometry difference checked by tolerance; some corpora use recorded CRT results or synthetic noise. Live wing chord inputs now use recovered ACF rib conversions and the typed reader scans 48 slots. The live boundary coordinates now use the recovered writer for up to ten elements, with ACF chord offsets and CG-relative origins. Dynamic wing rotation/retraction and aircraft curvature are fixed to zero; element force arms/normals and much of the running flight model remain approximate. Controlled whole-writer comparison covers 240 cases and 1431 points: all 4293 words match with recorded original CRT results, 4292 match using platform trigonometry (one bounded difference). Original geometry update scheduling, reference state and noise initialization are not yet connected. Curvature setup is recovered from acf/_ball_ang_def_deg: a local 521-case original-code comparison matches 1562/1563 words exactly, with one bounded platform-math difference. Live curvature remains zero until complete original reference bounds are supplied; the setup comparison is explicitly opt-in and its corpus is local. No whole-model parity is established. Body quad emission matches 47844 coordinate words across 120 local original-code cases; the ten-slot gear point pass matches 23952 words across 500 cases, both without tolerances. Inputs are explicit transformed body grids and stored gear fields, not proven live runtime state. Propeller point production now matches 28632 words across 400 local original-code cases, without tolerance. Gear, wing, body and propeller point producers are recovered individually; combined reference-vector assembly now matches 22350 point words and 800 final bounds words across 80 single-pass original-code cases. These checks include original auxiliary numerical helpers with only new/delete replaced; their auxiliary output fields are not compared. Body-grid/propeller runtime preparation and live reference integration remain unfinished. |
| Datarefs and commands | 8 | 34% | 20% | Registries and catalogs of 5503 datarefs and 3138 native command records from the original, its default keyboard map (182 bindings, 178 distinct chords, including Shift/Alt/Ctrl), values for 6 datarefs; 30 unmodified key commands act in the flight viewer; 2388 datarefs mapped to the field of the flight object or the simulation object that their getter reads (decoded from the getter code). All 182 key/mask pairs match a local original-code copy-block replay; four conflicting original chords are preserved and not dispatched. OS event lifecycle and UI handlers remain unverified. A separate native key-vector transition port matches 2728 local replay cases (active fields, suppression and ordered callback requests); callback bodies are logged, and live wiring is pending. |
| Scenery and world | 14 | 10% | 0% | Airport ground from apt.dat (runways, taxiways, aprons). No DSF, terrain, objects, lights or weather. |
| Cockpit, instruments, Lua systems | 10 | 0% | 0% | Not started. |
| Rendering | 6 | 20% | 0% | wgpu viewer with lighting and glass. No PBR, shadows, sky, clouds or instrument screens. Original Alt/Esc UI is not ported in the active viewer: Esc exits, and its HUD uses Roboto Mono. No paired reference visual or motion comparison establishes UI/graphics fidelity. Original local UI resources now decode through ui-resources: 25 PNG/DXT5 DDS textures at their stored dimensions and 12 intact font files. The viewer is not yet connected to them; local decoding is not visual parity. Native controller/window-registry queries and cumulative parent opacity are recovered: 1033 local contexts match query results, finite opacity bits and 2066 command traces. Four UI action bodies remain logged; explicit state services are not yet live viewer UI. |
| Audio, weather, multiplayer, plugins | 7 | 0% | 0% | Not started. |
| Platforms and distribution | 6 | 50% | 0% | CI on Linux, macOS and Windows, nightly builds, standard folder layout. No launcher or settings UI. |

## Roadmap

1. **Content without simulation** (mostly done): ACF, OBJ8, AFL, apt.dat and the viewer work. Left: PANEL_3D, normal/lit maps, instruments, an apt.dat index. Verification work: turn the diagnostic ACF object comparison into enforced checks over explicitly supported fields, keep local reference diagnostics explicitly opt-in, and resolve differences against completed loader fixtures.
2. **Datarefs and commands** (in progress): Registries and original catalogs exist. Left: values for the other 5497 datarefs, command handlers, wiring to the aircraft state. Complete command block-mask query now has 6418 local comparisons, including all 3138 IDs; recover mask lifecycle and full callback dispatch next. Full Begin/End executor now has 1768 native comparisons, including observer mutations and primary replacement; real callback construction/effects and live integration remain unfinished. Native Continue extends the comparison to 2152 combined phase cases; held scheduling and actual callback effects remain unfinished.
3. **Flight model** (in progress): Recovered profile, delta-wing, engine, propeller, contact and flight-step kernels exist with original-code comparisons. The approximate viewer now uses the delta branch and ordered wing grouping. The complete boundary coordinate loop and point transform now have original-code comparisons and run with static live inputs. Next: recover dynamic geometry state and full reference bounds, then connect the recovered curvature setup, original noise initialization and per-strip airflow; connect the remaining recovered force, propulsion and contact blocks into the runtime, then compare complete trajectories. Active rib chord values now come from ACF using the recovered conversion, and all 48 wing slots are scanned; dynamic geometry updates and force-arm/normal construction remain incomplete. Recovered body/gear point-list passes narrow the reference-bounds gap; propeller point production is now recovered too. The ordered main point vector and bounds reducer now match a single original-code pass on synthetic state. Next recover runtime body-grid and propeller field preparation, auxiliary outputs and the live geometry update schedule. Body-grid unit conversion now has exact checks on three real aircraft; runtime inclusion, normal generation and geometry scheduling remain work.
4. **World** (started): Airport ground from apt.dat. Left: DSF terrain, library objects, lights, start locations, weather.
5. **Cockpit and systems** (planned): Instruments, panels, the stock Cessna's Lua scripts and controls.
6. **Original UI and graphics fidelity** (started): Whole-game X-Plane 12 UI/graphics parity remains unproven. Available local resources: 25 menu textures and 12 fonts. Recovered input/state includes all 182 default chords, keyboard/UI gates, focused dispatch, window classification/opacity, focus transitions and complete command phases. The supplied Windows XPLM key hook now has 1090 local native comparisons; the missing-file pause is resolved, and 2152 paired CPU cases now execute EXE/DLL keyboard routing together; real module initialization and release-build pairing remain unproven. Left: actual TLS/module initialization and callback-list lifecycle, show_menu/Alt actions and widget registration, live viewer integration, original layout/font roles/GPU rendering and matched screenshots/GIF/video. The viewer still exits on Esc and renders no original menu; service-boundary replays are not pixel evidence. SDK context/activation now has 1108 native cases and runs unchanged in hook/paired tests on ready TLS; actual initialization/module lifecycle remains unfinished. Effective owner lookup through zero context frames now matches 1657 native stacks. Deferred services validate/drain deletions/validate; standalone queue mutation now has 1704 native comparisons for sniffer/hotkey deletion, with diagnostic/tracking/clock/free services controlled. Sniffer registration/unregistration now has 1126 native scenarios and connects to deferred deletion. Registered token-based sniffer phases 0/2 now match 524 native mutable event scenarios, including callback API calls and context-sensitive deletion order. 528 additional combined EXE/DLL cases now verify actual registrations/list mutation across keyboard routing, including callback mode/block gates. Hotkey/window registration and real loader/viewer/widget lifetime integration remain unfinished. Kind-11 construction now identifies the concrete menu panel; its own Esc/Return handler has 1940 native contexts for own/focused paths. Fallback effects and live creation/focus/layout remain unfinished. Concrete fallback request flow now matches 746 direct/own/focused contexts; property2 and empty-wrapper lifecycle are original. Real direct/submission/construction effects and pixels remain unverified.
7. **Audio, multiplayer, plugins** (planned): Sound, networking and the plugin interface.
8. **Platforms and distribution** (started): CI and nightly builds exist. Left: a launcher, settings, tagged releases.

Next up:

- Recover actual menu direct action 0x140f0a2c0, submission 0x140ef67c0 and constructor 0x140ec9c10, plus query A/B and manager/cache effects; concrete fallback control flow now has 746 direct/own/focused contexts. Connect kind-11 creation/registry/flags/focus/children, identify show_menu ID-3/Alt activation and recover original layout/font/GPU output with paired captures.
- Trace actual Alt/Esc show_menu/main-menu/settings actions: live observer/override/primary registration, held-command scheduling, UI classification/registry lifecycle, focus callers, leave/enter notifications, widget masks and concrete child callbacks. Connect the verified keyboard/command routing to the live viewer.
- Recover original menu state, layout and font roles; connect the 25 available textures and 12 fonts to GPU rendering, then compare paired screenshots and GIF/video under identical conditions. Esc still exits the viewer and no original menu is drawn.
- Recover dynamic wing geometry updates and complete reference bounds; connect recovered curvature setup and original element force arms/normals
- Recover runtime body grids and propeller radius/center fields; validate real-aircraft reference snapshots and connect the verified main reference-vector pipeline
- Port per-strip airflow (0x14121a9b0) and connect recovered propulsion and contact blocks to the live model
- Compare complete live trajectories against the original after runtime integration
- Fill datarefs from aircraft state and implement remaining command handlers
- Read DSF scenery tiles and implement cockpit instruments and the stock Cessna Lua systems

Details and the target crate layout: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
<!-- compat:end -->

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

## Airport scene

```sh
cargo run --offline -- airport-render Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" ksea.png
cargo run --offline -- airport-view Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
```

Draws an airport from `apt.dat` (runways with markings, taxiway and apron pavements with holes) and puts the
aircraft on the first runway. Static, no terrain or objects. Rules and limits:
[research/WORLD_GEOMETRY.md](research/WORLD_GEOMETRY.md).

![Cessna on runway 16L at KSEA](assets/screenshots/ksea-cessna.png)

## Datarefs

```sh
cargo run --offline -- datarefs "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
python3 tools/extract_datarefs.py Xplane12/X-Plane.exe --prefix sim/aircraft/weight/
```

`datarefs` registers the datarefs whose backing ACF fields are confirmed in the original (empty, maximum and fuel
mass, CG Y/Z, engine count) with the type and writability of the reference build. `extract_datarefs.py` lists all
5503 registration records of the original's table. Layout, evidence and limits:
[research/DATAREFS.md](research/DATAREFS.md).

## Flight (approximate flight model)

```sh
cargo run --release --offline -- fly Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
cargo run --release --offline -- fly-render Xplane12 KSEA "Xplane12/Cessna 172 SP/Cessna_172SP.acf" frame 0 20 30 45
cargo run --release --offline -- wing-info "Xplane12/Cessna 172 SP/Cessna_172SP.acf"
cargo run --release --offline -- fly-test Xplane12 "Xplane12/Cessna 172 SP/Cessna_172SP.acf" 60
```

`fly` opens a window with the Cessna on the first runway of the airport. It uses the **original's default
keyboard map**, read from the reference build: F1/F2/F3 throttle down/up/full, `1`/`2` flaps up/down, `B` brakes
(hold), `V` maximum brakes, `[` `]` pitch trim, `5 6 7` rudder trim, `8 9 0` aileron trim, `P` pause, `W` default
view, `Q E R F` and `= -` move the camera. The original has no keyboard stick, so openXplane adds one: arrow keys
pitch and roll, `Z`/`X` rudder (`Tab` gives them their original meaning back), `Delete` resets, `Esc` quits. `OPENXPLANE_DATA_OUTPUT=0,3,4,17,18,21` draws the reference build's data-output lines with those indexes (the Data Output list of X-Plane: 0 frame rate, 3 speeds, 4 Mach and vertical speed, 8 stick, 13 trims and flaps, 17 attitude, 18 alpha and flight path, 20 altitude, 21 position and velocity, 25 throttle; `OPENXPLANE_FRAME_RATE=1` adds line 0); see `crates/xp-app/src/dout.rs`.
Commands without anything to act on yet (mixture, magnetos, carb heat...) are reported in the window title as not
simulated. The title also shows speed, altitude, vertical speed, pitch, bank, heading, throttle, flaps and a stall
warning. Full table: [docs/KEYBOARD.md](docs/KEYBOARD.md); source of the map: [research/KEYMAP.md](research/KEYMAP.md).
`fly-render` runs a scripted takeoff and saves chase-camera frames at the given times.

![Cessna climbing out of KSEA](assets/screenshots/flight-climb.png)

An approximate rigid-body model on the Cessna's real ACF and airfoil data: the wing is split into elements and
each one uses the airfoil code verified against the original; thrust, landing gear and control surfaces are
openXplane's own simplifications. It takes off and climbs under a scripted pilot. It is **not** the original's
flight model; what is original and what is assumed is listed in
[research/FLIGHT_MODEL.md](research/FLIGHT_MODEL.md).

## Verification against the original machine code

`tools/emulate_xp.py` runs isolated functions of `X-Plane.exe` in a CPU emulator on synthetic inputs, and
`tests/original_vectors.rs` requires the Rust port to match their outputs bit for bit: 1200 cases of the profile
stage (angle correction, lookup, stall, buffet noise) and the interpolation helper all agree. Method and limits:
[research/VERIFICATION.md](research/VERIFICATION.md).

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

The original's command table (3138 native command records) is enumerated by the tool, and `CommandRegistry` loads it
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

While the viewer is open, Discord Rich Presence can show what you are looking at. It uses the project's own
Discord application (set `OPENXPLANE_DISCORD_APP_ID` to an empty value to turn it off, or to another id to use your
own); details and limits: [docs/DISCORD.md](docs/DISCORD.md). The
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

GPL-3.0-or-later, see [LICENSE](LICENSE) and [NOTICE](NOTICE). X-Plane is a trademark of its respective owner. openXplane is an independent
project and is not affiliated with it.
