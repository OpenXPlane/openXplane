# openXplane architecture and roadmap

The model for the organisation is [openOMSI](https://github.com/openOMSI-Project/openOMSI): a simulator
rewritten from scratch in Rust that works on the content of an installed original and contains no foreign code
or assets. The goals are the same: 1) 1:1 behaviour on the original content, 2) no code or assets of the
original, with formats described in our own words, 3) a modern engine (wgpu, multithreaded loading, 64-bit).

The difference from openOMSI: X-Plane is a flight simulator, so the core is the flight model and the datarefs,
not timetables and AI traffic. Behaviour is confirmed by analysing the original EXE (`research/*.md`) and by
comparison with the reference build.

## Target structure (workspace)

For now everything lives in one crate, `openxplane`. It should be split into crates as a second consumer of a
module appears, not in advance.

| Crate | Contents | Today |
| --- | --- | --- |
| `xp-cfg` | text formats, virtual file system, content roots | `lib.rs` (ACF) |
| `xp-acf` | typed ACF, PANEL_3D | `aircraft.rs`, `lib.rs` |
| `xp-airfoil` | AFL, lookup, stall, buffet, regimes | `airfoil`, `aero`, `stall`, `buffet`, `regimes`, `profile` |
| `xp-obj` | OBJ8, animations, materials | `obj8.rs` |
| `xp-texture` | DDS/PNG, mipmaps, normal/lit | `image` in `gpu.rs` |
| `xp-scenery` | apt.dat, earth_nav.dat, DSF, libraries | `apt.rs` |
| `xp-dataref` | registry of datarefs and commands | `dataref.rs` (datarefs) |
| `xp-sim` | flight model, engines, landing gear, weather | none |
| `xp-script` | runtime for the stock Lua scripts | none |
| `xp-render` | wgpu renderer | `gpu.rs`, `scene.rs`, `viewer.rs` |
| `xp-audio`, `xp-net`, `xp-plugin` | audio, multiplayer, plugins | none |
| `xp-app` | binary, launcher, HUD | `main.rs` |
| `tools/xp-check` | format coverage over a content directory | the `inspect` command |

## Milestones

The order is set by dependencies: each next one builds on the previous. The criterion for "done" is a check on
the original content, not only unit tests.

1. **Content without simulation** (mostly done): ACF, OBJ8, AFL, apt.dat, the Cessna viewer. Remaining:
   PANEL_3D, materials and normal/lit maps, `xp-check` over the whole directory, an apt.dat index, decoding the
   surface codes.
2. **Datarefs and commands** (mostly done): a typed registry, the full 5503-record catalog of the reference
   build, and the six ACF-backed datarefs with confirmed types. Remaining: commands, backing values for the
   other datarefs, propagating writes to the aircraft state. Everything after depends on it.
3. **Flight model**: wing forces from AFL, propeller and engine, landing gear, mass and centre of gravity,
   atmosphere; a simulation step; comparing trajectories with the reference on fixed inputs.
4. **World**: DSF, terrain, library objects, airport lighting, placing the aircraft on a runway of the test
   airport.
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
