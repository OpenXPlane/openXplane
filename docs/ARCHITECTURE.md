# openXplane architecture and roadmap

The model for the organisation is [openOMSI](https://github.com/openOMSI-Project/openOMSI): a simulator
rewritten from scratch in Rust that works on the content of an installed original and contains no foreign code
or assets. The goals are the same: 1) 1:1 behaviour on the original content, 2) no code or assets of the
original, with formats described in our own words, 3) a modern engine (wgpu, multithreaded loading, 64-bit).

The difference from openOMSI: X-Plane is a flight simulator, so the core is the flight model and the datarefs,
not timetables and AI traffic. Behaviour is confirmed by analysing the original EXE (`research/*.md`) and by
comparison with the reference build.

## Crates

The repository is a Cargo workspace with one crate per subsystem, in the manner of openOMSI. `xp-app` is the
only binary (`openxplane`); it re-exports the other crates through a small facade so the tools and tests name
everything in one place.

| Crate | Contents |
| --- | --- |
| `xp-acf` | the ACF property reader, typed aircraft parameters, lifting surfaces and their geometry |
| `xp-airfoil` | AFL tables, angle correction and lookup, stall, buffet, regimes, the profile function, the wing element function (all verified against the original) |
| `xp-obj` | the OBJ8 model reader |
| `xp-scenery` | the installation folder layout and precedence, apt.dat, airport ground geometry |
| `xp-dataref` | dataref and command registries, the original's default keyboard map |
| `xp-sim` | the approximate flight model, pilot commands |
| `xp-discord` | Discord Rich Presence |
| `xp-app` | the binary: viewer, flight, command line tools; the tests against the original |

Planned splits as the work grows: the renderer (`gpu`, `scene`, `viewer`) into `xp-render`, the Lua runtime into
`xp-script`, audio, networking and plugins into their own crates, a `tools/xp-check` coverage tool over a
content folder.

## Milestones

The order is set by dependencies: each next one builds on the previous. The criterion for "done" is a check on
the original content, not only unit tests.

1. **Content without simulation** (mostly done): ACF, OBJ8, AFL, apt.dat, the Cessna viewer. Remaining:
   PANEL_3D, materials and normal/lit maps, `xp-check` over the whole directory, an apt.dat index, decoding the
   surface codes.
2. **Datarefs and commands** (registries done): a typed dataref registry, the 5503-record dataref catalog and the
   3012-command catalog of the reference build, six ACF-backed datarefs with confirmed types, and a command
   registry with phases and handler chains. Remaining: backing values for the other datarefs, propagating writes
   to the aircraft state, the built-in command handlers. Everything after depends on it.
3. **Flight model**: wing forces from AFL, propeller and engine, landing gear, mass and centre of gravity,
   atmosphere; a simulation step; comparing trajectories with the reference on fixed inputs.
4. **World** (started): the airport ground from apt.dat is drawn with the aircraft on a runway. Remaining: DSF,
   terrain, library objects, airport lighting, spawn from start locations.
5. **Cockpit and systems**: instruments, panels, the stock Cessna's Lua, controls, input.
6. **Audio, weather, multiplayer, plugins.**
7. **Platforms and distribution**: macOS/Windows/Linux builds, CI.

## Working rules

- Each confirmed subsystem is written up in `research/*.md` with addresses/source and an explicit "not
  established" boundary.
- The unknown is not guessed or filled in by default; inputs that are not yet recovered are passed explicitly.
- User content is read separately from the code; the original files are not modified and do not enter the
  repository (`Xplane12/` is in `.gitignore`).
- Legal boundary: disassembly serves to understand formats and algorithms; the code and assets of the original
  are not copied.
